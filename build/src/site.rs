//! `Site`: o builder usado no `build.rs` do site.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::component::MdComponent;
use crate::error::Error;
use crate::frontmatter::Layout;
use crate::page::{CompiledPage, compile_page, format_rust};

/// Configuração do site e geração de `press.rs` + arquivos estáticos.
pub struct Site {
    root: PathBuf,
    content: PathBuf,
    title: String,
    description: String,
    nav: Vec<(String, String)>,
    groups: Vec<(String, String)>,
    components: Vec<Box<dyn MdComponent>>,
}

/// Saída da geração (antes de escrever em disco).
pub struct Generated {
    /// Conteúdo de `press.rs` (já formatado).
    pub rust: String,
    /// Arquivos estáticos (`llms.txt`, `llms-full.txt`, `<rota>.md`), por caminho relativo.
    pub files: BTreeMap<String, String>,
    /// Arquivos e diretórios que o build leu.
    pub inputs: Vec<PathBuf>,
}

impl Site {
    /// `content` é a pasta dos `.md`, relativa à raiz do site (padrão: `CARGO_MANIFEST_DIR`).
    pub fn new(content: impl Into<PathBuf>) -> Self {
        let root = std::env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            root,
            content: content.into(),
            title: String::new(),
            description: String::new(),
            nav: Vec::new(),
            groups: Vec::new(),
            components: Vec::new(),
        }
    }

    /// Raiz do site (para testes; no `build.rs` vale o padrão).
    pub fn root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = root.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Links do cabeçalho. Um link que não é uma página (ex.: `/components/`) aponta para a
    /// primeira página dessa pasta; um que não resolve é erro de build.
    pub fn nav<I, A, B>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = (A, B)>,
        A: Into<String>,
        B: Into<String>,
    {
        self.nav = items
            .into_iter()
            .map(|(a, b)| (a.into(), b.into()))
            .collect();
        self
    }

    /// Sidebar: um grupo por pasta de primeiro nível; ordem das páginas = `order` do
    /// front-matter, depois título. Grupos declarados com [`Site::group`] vêm primeiro.
    pub fn sidebar_from_folders(self) -> Self {
        self
    }

    /// Título e posição de um grupo da sidebar (`folder` = nome da pasta).
    pub fn group(mut self, folder: impl Into<String>, text: impl Into<String>) -> Self {
        self.groups.push((folder.into(), text.into()));
        self
    }

    /// Registra uma tag aceita no Markdown.
    pub fn component(mut self, component: impl MdComponent + 'static) -> Self {
        self.components.push(Box::new(component));
        self
    }

    /// Gera tudo em memória.
    pub fn generate(&self) -> Result<Generated, Error> {
        let content_dir = self.root.join(&self.content);
        let mut files = Vec::new();
        collect_md(&content_dir, &mut files)?;
        files.sort();
        if files.is_empty() {
            return Err(Error::new(format!(
                "nenhum .md em {}",
                content_dir.display()
            )));
        }

        let reads = RefCell::new(Vec::<PathBuf>::new());
        let mut inputs = vec![content_dir.clone()];
        let mut pages: Vec<CompiledPage> = Vec::new();
        let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
        for path in &files {
            let rel = path.strip_prefix(&self.root).unwrap_or(path).to_path_buf();
            let in_content = path.strip_prefix(&content_dir).unwrap_or(path);
            let route = route_of(in_content);
            if let Some(prev) = seen.insert(route.clone(), rel.clone()) {
                return Err(Error::new(format!(
                    "a rota {route} é gerada por {} e por {}",
                    prev.display(),
                    rel.display()
                ))
                .at(&rel, 1));
            }
            let src = std::fs::read_to_string(path).map_err(|e| Error::io(&rel, &e))?;
            inputs.push(path.clone());
            let page = compile_page(&src, &rel, &route, &self.root, &self.components)?;
            reads.borrow_mut().extend(page.reads.iter().cloned());
            pages.push(page);
        }
        inputs.extend(reads.into_inner());

        let md_files: BTreeMap<String, String> = pages
            .iter()
            .zip(&files)
            .map(|(p, f)| (p.route.clone(), md_name(&p.route, f)))
            .collect();
        self.validate_links(&pages, &md_files)?;

        let sidebar = self.sidebar(&pages);
        let nav = self.resolve_nav(&pages, &sidebar)?;
        let rust = self.emit_rust(&pages, &sidebar, &nav)?;
        let files = self.static_files(&pages, &sidebar, &md_files);
        Ok(Generated {
            rust,
            files,
            inputs,
        })
    }

    /// Gera e escreve `$OUT_DIR/press.rs` e `$OUT_DIR/press-static/`, com os
    /// `cargo:rerun-if-changed`. Para o `build.rs`.
    pub fn build(self) -> Result<(), Error> {
        let out = std::env::var_os("OUT_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| Error::new("OUT_DIR não definido (rode dentro do build.rs)"))?;
        let generated = self.generate()?;
        write_all(&out, &generated)?;
        for input in &generated.inputs {
            println!("cargo:rerun-if-changed={}", input.display());
        }
        println!("cargo:rerun-if-changed=build.rs");
        Ok(())
    }

    fn validate_links(
        &self,
        pages: &[CompiledPage],
        md_files: &BTreeMap<String, String>,
    ) -> Result<(), Error> {
        let mut valid: BTreeSet<String> = pages.iter().map(|p| p.route.clone()).collect();
        valid.insert("/llms.txt".into());
        valid.insert("/llms-full.txt".into());
        valid.extend(md_files.values().map(|m| format!("/{m}")));
        for page in pages {
            for link in &page.links {
                let path = link.url.split(['#', '?']).next().unwrap_or("");
                if !valid.contains(&normalize_route(path)) && !valid.contains(path) {
                    return Err(Error::new(format!(
                        "link interno quebrado: {} não é uma página do site",
                        link.url
                    ))
                    .at(&page.file, link.line));
                }
            }
        }
        Ok(())
    }

    fn sidebar(&self, pages: &[CompiledPage]) -> Vec<Group> {
        let mut by_folder: BTreeMap<String, Vec<&CompiledPage>> = BTreeMap::new();
        for page in pages {
            if page.front.layout == Layout::Home {
                continue;
            }
            let Some(folder) = page.route.trim_start_matches('/').split('/').next() else {
                continue;
            };
            // Páginas na raiz (sem pasta) não entram na sidebar.
            if page.route.trim_start_matches('/') == folder && !is_index_of_folder(page) {
                continue;
            }
            by_folder.entry(folder.to_string()).or_default().push(page);
        }
        let mut order: Vec<String> = self.groups.iter().map(|(f, _)| f.clone()).collect();
        let rest: Vec<String> = by_folder
            .keys()
            .filter(|f| !order.contains(f))
            .cloned()
            .collect();
        order.extend(rest);
        order
            .into_iter()
            .filter_map(|folder| {
                let mut list = by_folder.remove(&folder)?;
                list.sort_by_key(|p| (p.front.order.unwrap_or(i64::MAX), p.title.to_lowercase()));
                let text = self
                    .groups
                    .iter()
                    .find(|(f, _)| *f == folder)
                    .map_or_else(|| title_case(&folder), |(_, t)| t.clone());
                Some(Group {
                    text,
                    items: list
                        .into_iter()
                        .map(|p| (p.title.clone(), p.route.clone(), p.description.clone()))
                        .collect(),
                })
            })
            .collect()
    }

    fn resolve_nav(
        &self,
        pages: &[CompiledPage],
        sidebar: &[Group],
    ) -> Result<Vec<(String, String, String)>, Error> {
        self.nav
            .iter()
            .map(|(text, link)| {
                let prefix = normalize_route(link);
                if pages.iter().any(|p| p.route == prefix) {
                    return Ok((text.clone(), prefix.clone(), prefix));
                }
                let first = sidebar
                    .iter()
                    .flat_map(|g| g.items.iter())
                    .find(|(_, route, _)| route.starts_with(&format!("{prefix}/")));
                match first {
                    Some((_, route, _)) => Ok((text.clone(), route.clone(), prefix)),
                    None => Err(Error::new(format!(
                        "o link do nav `{link}` ({text}) não resolve para nenhuma página"
                    ))),
                }
            })
            .collect()
    }

    fn emit_rust(
        &self,
        pages: &[CompiledPage],
        sidebar: &[Group],
        nav: &[(String, String, String)],
    ) -> Result<String, Error> {
        let title = &self.title;
        let description = &self.description;
        let nav_items = nav.iter().map(|(text, link, prefix)| {
            quote! { ::leptos_press::NavLink { text: #text, link: #link, prefix: #prefix } }
        });
        let groups = sidebar.iter().map(|g| {
            let text = &g.text;
            let items = g.items.iter().map(|(t, route, _)| {
                quote! { ::leptos_press::SidebarItem { text: #t, link: #route } }
            });
            quote! { ::leptos_press::SidebarGroup { text: #text, items: &[ #( #items ),* ] } }
        });
        let metas = pages.iter().map(page_meta);
        let mut items: Vec<TokenStream> = Vec::new();
        for item in pages.iter().flat_map(|p| p.items.iter()) {
            if !items.iter().any(|i| i.to_string() == item.to_string()) {
                items.push(item.clone());
            }
        }
        let fns = pages.iter().enumerate().map(|(i, p)| {
            let name = format_ident!("page_{i}");
            p.page_fn(&name, i)
        });
        let routes = nest(
            pages
                .iter()
                .enumerate()
                .map(|(i, p)| route_tokens(&p.route, &format_ident!("page_{i}")))
                .collect(),
        );
        let tokens = quote! {
            // @generated por leptos-press-build — não editar.
            // O crate que inclui este arquivo depende de `leptos`, `leptos_router` e `leptos-press`.

            /// Metadados do site: título, nav, sidebar e páginas.
            pub static SITE: ::leptos_press::SiteMeta = ::leptos_press::SiteMeta {
                title: #title,
                description: #description,
                nav: &[ #( #nav_items ),* ],
                sidebar: &[ #( #groups ),* ],
                pages: &[ #( #metas ),* ],
            };

            #( #items )*

            #( #fns )*

            /// Uma rota estática (`SsrMode::Static`) por página.
            pub fn routes() -> impl ::leptos_router::MatchNestedRoutes
                + ::core::clone::Clone
                + ::core::marker::Send
                + 'static {
                #routes
            }
        };
        format_rust(tokens)
    }

    fn static_files(
        &self,
        pages: &[CompiledPage],
        sidebar: &[Group],
        md_files: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let mut files = BTreeMap::new();
        for page in pages {
            files.insert(md_files[&page.route].clone(), page.markdown.clone());
        }
        let by_route: BTreeMap<&str, &CompiledPage> =
            pages.iter().map(|p| (p.route.as_str(), p)).collect();

        // llms.txt (llmstxt.org)
        let mut txt = format!("# {}\n\n", self.title);
        if !self.description.is_empty() {
            txt.push_str(&format!("> {}\n\n", self.description));
        }
        for group in sidebar {
            txt.push_str(&format!("## {}\n\n", group.text));
            for (title, route, desc) in &group.items {
                txt.push_str(&format!("- [{title}](/{})", md_files[route]));
                if !desc.is_empty() {
                    txt.push_str(&format!(": {desc}"));
                }
                txt.push('\n');
            }
            txt.push('\n');
        }
        files.insert("llms.txt".into(), format!("{}\n", txt.trim_end()));

        // llms-full.txt: home, sidebar na ordem, depois o resto.
        let mut ordered: Vec<&str> = Vec::new();
        if by_route.contains_key("/") {
            ordered.push("/");
        }
        for (_, route, _) in sidebar.iter().flat_map(|g| g.items.iter()) {
            ordered.push(route);
        }
        for route in by_route.keys() {
            if !ordered.contains(route) {
                ordered.push(route);
            }
        }
        let full = ordered
            .iter()
            .map(|route| {
                format!(
                    "---\nurl: /{}\n---\n\n{}",
                    md_files[*route],
                    by_route[route].markdown.trim_end()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        files.insert("llms-full.txt".into(), format!("{full}\n"));
        files
    }
}

struct Group {
    text: String,
    /// (título, rota, descrição)
    items: Vec<(String, String, String)>,
}

fn write_all(out: &Path, generated: &Generated) -> Result<(), Error> {
    let io = |p: &Path, e: std::io::Error| Error::io(p, &e);
    let rust_path = out.join("press.rs");
    std::fs::write(&rust_path, &generated.rust).map_err(|e| io(&rust_path, e))?;
    let static_dir = out.join("press-static");
    if static_dir.exists() {
        std::fs::remove_dir_all(&static_dir).map_err(|e| io(&static_dir, e))?;
    }
    for (rel, text) in &generated.files {
        let path = static_dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
        }
        std::fs::write(&path, text).map_err(|e| io(&path, e))?;
    }
    Ok(())
}

fn page_meta(page: &CompiledPage) -> TokenStream {
    let route = &page.route;
    let title = &page.title;
    let description = &page.description;
    let layout = match page.front.layout {
        Layout::Doc => quote! { ::leptos_press::PageLayout::Doc },
        Layout::Home => quote! { ::leptos_press::PageLayout::Home },
    };
    let headings = page
        .headings
        .iter()
        .filter(|h| h.level == 2 || h.level == 3)
        .map(|h| {
            let level = h.level;
            let id = &h.id;
            let text = &h.text;
            quote! { ::leptos_press::Heading { level: #level, id: #id, text: #text } }
        });
    let home = if page.front.layout == Layout::Home {
        let hero = page.front.hero.clone().unwrap_or_default();
        let opt = |v: &Option<String>| match v {
            Some(s) => quote! { ::core::option::Option::Some(#s) },
            None => quote! { ::core::option::Option::None },
        };
        let (name, text, tagline) = (opt(&hero.name), opt(&hero.text), opt(&hero.tagline));
        let actions = hero.actions.iter().enumerate().map(|(i, a)| {
            let (t, l) = (&a.text, &a.link);
            let primary = match a.theme.as_deref() {
                Some(theme) => theme == "brand",
                None => i == 0,
            };
            quote! { ::leptos_press::HeroAction { text: #t, link: #l, primary: #primary } }
        });
        let features = page.front.features.iter().map(|f| {
            let title = &f.title;
            let details = f.details.clone().unwrap_or_default();
            let icon = f.icon.clone().unwrap_or_default();
            quote! { ::leptos_press::Feature { title: #title, details: #details, icon: #icon } }
        });
        quote! {
            ::core::option::Option::Some(&::leptos_press::HomeMeta {
                name: #name,
                text: #text,
                tagline: #tagline,
                actions: &[ #( #actions ),* ],
                features: &[ #( #features ),* ],
            })
        }
    } else {
        quote! { ::core::option::Option::None }
    };
    quote! {
        ::leptos_press::PageMeta {
            path: #route,
            title: #title,
            description: #description,
            layout: #layout,
            headings: &[ #( #headings ),* ],
            home: #home,
        }
    }
}

fn route_tokens(route: &str, func: &syn::Ident) -> TokenStream {
    let path = route.trim_start_matches('/');
    quote! {
        ::leptos_router::components::Route(
            ::leptos_router::components::RouteProps::builder()
                .path(::leptos_router::path!(#path))
                .view(#func)
                .ssr(::leptos_router::SsrMode::Static(
                    ::leptos_router::static_routes::StaticRoute::new(),
                ))
                .build(),
        )
    }
}

/// `MatchNestedRoutes` existe para tuplas de tamanho limitado: agrupa em tuplas aninhadas.
fn nest(items: Vec<TokenStream>) -> TokenStream {
    const MAX: usize = 12;
    match items.len() {
        0 => quote! { () },
        1 => items.into_iter().next().unwrap_or_default(),
        n if n <= MAX => quote! { ( #( #items ),* ) },
        _ => nest(items.chunks(MAX).map(|c| nest(c.to_vec())).collect()),
    }
}

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Error> {
    let entries = std::fs::read_dir(dir).map_err(|e| Error::io(dir, &e))?;
    for entry in entries {
        let path = entry.map_err(|e| Error::io(dir, &e))?.path();
        if path.is_dir() {
            collect_md(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
    Ok(())
}

/// `guide/getting-started.md` → `/guide/getting-started`; `a/index.md` → `/a`; `index.md` → `/`.
fn route_of(rel: &Path) -> String {
    let mut parts: Vec<String> = rel
        .with_extension("")
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.last().is_some_and(|p| p == "index") {
        parts.pop();
    }
    format!("/{}", parts.join("/"))
}

/// Caminho da cópia `.md` (relativo à saída): `/` → `index.md`; pasta com `index.md` →
/// `pasta/index.md`; página → `rota.md`.
fn md_name(route: &str, file: &Path) -> String {
    if route == "/" {
        return "index.md".into();
    }
    let trimmed = route.trim_start_matches('/');
    if file.file_stem().is_some_and(|s| s == "index") {
        format!("{trimmed}/index.md")
    } else {
        format!("{trimmed}.md")
    }
}

fn normalize_route(link: &str) -> String {
    let trimmed = link.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".into()
    } else {
        trimmed.to_string()
    }
}

fn is_index_of_folder(page: &CompiledPage) -> bool {
    page.file.file_stem().is_some_and(|s| s == "index")
}

fn title_case(folder: &str) -> String {
    let spaced = folder.replace(['-', '_'], " ");
    let mut chars = spaced.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}
