//! Plugins de build ([ADR-0011]): recebem o site já compilado, validam páginas e emitem
//! arquivos estáticos (`press-static/`). Só leitura: um plugin não altera páginas nem código.
//!
//! Ordem em [`Site::generate`](crate::Site::generate): compilar as páginas → `check_page` de
//! cada plugin (na ordem de registro, página a página) → `emit` de cada plugin → validar os
//! links internos, aceitando como destino as rotas **e** os arquivos emitidos.
//!
//! [ADR-0011]: https://github.com/nebula-rs/leptos-next/blob/main/docs/decisions/0011-press-plugins.md

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use comrak::nodes::NodeValue;
use comrak::{Arena, Options, parse_document};

use crate::error::Error;
use crate::frontmatter::{FrontMatter, Layout};
use crate::render::{Heading, resolve_relative};

/// Um plugin de build, registrado com [`Site::plugin`](crate::Site::plugin).
pub trait Plugin {
    /// Nome curto (`"llms"`), usado nas mensagens de erro.
    fn name(&self) -> &'static str;

    /// Valida uma página. O erro ganha `arquivo.md:1` se não trouxer posição.
    fn check_page(&self, _page: &PageData) -> Result<(), Error> {
        Ok(())
    }

    /// Emite os arquivos do plugin.
    fn emit(&self, site: &SiteData, out: &mut Output) -> Result<(), Error>;
}

/// O site compilado, como os plugins o veem.
///
/// # Locales
///
/// Every site has at least one locale ([`LocaleData`]). A site that declares no locales (the
/// single-language case) has one **implicit** locale: code `""`, label `""`, prefix `""` — so
/// a plugin that writes its files under each locale's prefix writes them exactly where it
/// did before locales existed.
///
/// Contract for plugins (and for the core, which fills this in):
///
/// - [`PageData::route`] is the full site route, **already prefixed** with the page's locale
///   (`/en/guide/start`; the home of a locale with prefix `/en` is `/en`). Plugins never add
///   the prefix to page routes; they only add it to the paths of the files they emit
///   ([`LocaleData::output_path`]).
/// - [`GroupData::folder`] is relative to the locale root (`guide`, never `en/guide`): the
///   same folder name can be a group in every locale.
/// - [`SiteData::local_route`] gives a page's route relative to its locale root (`/en/guide`
///   → `/guide`; `/en` → `/`), for options that are written once for all locales.
///
/// To build one by hand (tests of a plugin): [`SiteData::builder`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SiteData {
    pub title: String,
    pub description: String,
    /// Páginas na ordem da sidebar; depois as que não estão nela (home incluída), por rota.
    ///
    /// With several locales, the pages of every locale, each locale in that order.
    pub pages: Vec<PageData>,
    /// Grupos da sidebar, na ordem de exibição (of every locale; see [`SiteData::groups_in`]).
    pub groups: Vec<GroupData>,
    /// The site's locales, in declaration order (never empty).
    pub locales: Vec<LocaleData>,
    /// Code of the default locale (one of [`SiteData::locales`]; `""` for the implicit one).
    pub default_locale: String,
}

impl SiteData {
    /// Starts a [`SiteDataBuilder`]: a `SiteData` built by hand, for testing a plugin without
    /// compiling a site from Markdown.
    ///
    /// ```
    /// use leptos_press_build::{GroupData, PageData, SiteData};
    ///
    /// let site = SiteData::builder("My site")
    ///     .locale("en", "English", "/en") // the first one is the default
    ///     .locale("pt", "Português", "/pt")
    ///     .page(PageData::new("/en/guide/start", "content/en/guide/start.md").with_locale("en"))
    ///     .page(PageData::new("/pt/guide/start", "content/pt/guide/start.md").with_locale("pt"))
    ///     .group(GroupData::new("en", "guide", "Guide", ["/en/guide/start"]))
    ///     .group(GroupData::new("pt", "guide", "Guia", ["/pt/guide/start"]))
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(site.default_locale, "en");
    /// assert_eq!(site.local_route(&site.pages[1]), "/guide/start");
    /// ```
    pub fn builder(title: impl Into<String>) -> SiteDataBuilder {
        SiteDataBuilder {
            title: title.into(),
            description: String::new(),
            locales: Vec::new(),
            default_locale: None,
            pages: Vec::new(),
            groups: Vec::new(),
        }
    }

    /// Página pela rota (`/components/btn`).
    pub fn page(&self, route: &str) -> Option<&PageData> {
        self.pages.iter().find(|p| p.route == route)
    }

    /// A locale by its code.
    pub fn locale(&self, code: &str) -> Option<&LocaleData> {
        self.locales.iter().find(|l| l.code == code)
    }

    /// More than one locale?
    pub fn is_multilingual(&self) -> bool {
        self.locales.len() > 1
    }

    /// The pages of a locale, in site order.
    pub fn pages_in<'a>(&'a self, locale: &'a str) -> impl Iterator<Item = &'a PageData> + 'a {
        self.pages.iter().filter(move |p| p.locale == locale)
    }

    /// The sidebar groups of a locale, in display order.
    pub fn groups_in<'a>(&'a self, locale: &'a str) -> impl Iterator<Item = &'a GroupData> + 'a {
        self.groups.iter().filter(move |g| g.locale == locale)
    }

    /// The page's route relative to its locale root: `/en/guide/start` → `/guide/start`,
    /// `/en` → `/`. With the implicit locale (or an unknown one), the route itself.
    pub fn local_route(&self, page: &PageData) -> String {
        self.locale(&page.locale)
            .and_then(|l| l.local_route(&page.route))
            .unwrap_or_else(|| page.route.clone())
    }
}

/// A locale of the site.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LocaleData {
    /// Language code (`en`, `pt-BR`); `""` for the implicit locale of a single-language site.
    pub code: String,
    /// Name shown to people (`English`, `Português`); `""` for the implicit locale.
    pub label: String,
    /// Route prefix of the locale (`/en`), or `""` when its pages are at the site root. No
    /// trailing `/`.
    pub prefix: String,
}

impl LocaleData {
    pub fn new(
        code: impl Into<String>,
        label: impl Into<String>,
        prefix: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            label: label.into(),
            prefix: prefix.into(),
        }
    }

    /// The implicit locale of a site that declares none: code, label and prefix `""`.
    pub fn implicit() -> Self {
        Self::new("", "", "")
    }

    /// `route` relative to this locale's root (`/en/guide` → `/guide`, `/en` and `/en/` →
    /// `/`), or `None` if the route is not under the prefix (`/english`). With prefix `""`,
    /// the route itself.
    pub fn local_route(&self, route: &str) -> Option<String> {
        if self.prefix.is_empty() {
            return Some(route.to_string());
        }
        let rest = route.strip_prefix(&self.prefix)?;
        if rest.is_empty() || rest == "/" {
            Some("/".into())
        } else if rest.starts_with('/') {
            Some(rest.to_string())
        } else {
            None
        }
    }

    /// The site route of a route relative to this locale (`/guide` → `/en/guide`, `/` →
    /// `/en`). The inverse of [`LocaleData::local_route`].
    pub fn site_route(&self, local: &str) -> String {
        if self.prefix.is_empty() {
            local.to_string()
        } else if local == "/" || local.is_empty() {
            self.prefix.clone()
        } else {
            format!("{}{local}", self.prefix)
        }
    }

    /// Path of a file emitted for this locale, relative to the output ([`Output::add`]):
    /// `llms.txt` → `en/llms.txt` (prefix `/en`) or `llms.txt` (prefix `""`).
    pub fn output_path(&self, file: &str) -> String {
        let dir = self.prefix.trim_start_matches('/');
        if dir.is_empty() {
            file.to_string()
        } else {
            format!("{dir}/{file}")
        }
    }

    /// The same file as a site URL: `/en/llms.txt` or `/llms.txt`.
    pub fn url(&self, file: &str) -> String {
        format!("/{}", self.output_path(file))
    }
}

/// Builds a [`SiteData`] by hand ([`SiteData::builder`]); [`SiteDataBuilder::build`] checks
/// the [locale contract](SiteData#locales).
#[derive(Debug, Clone)]
pub struct SiteDataBuilder {
    title: String,
    description: String,
    locales: Vec<LocaleData>,
    default_locale: Option<String>,
    pages: Vec<PageData>,
    groups: Vec<GroupData>,
}

impl SiteDataBuilder {
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Declares a locale. Without any, the site has the implicit locale (code and prefix `""`).
    pub fn locale(
        mut self,
        code: impl Into<String>,
        label: impl Into<String>,
        prefix: impl Into<String>,
    ) -> Self {
        self.locales.push(LocaleData::new(code, label, prefix));
        self
    }

    /// The default locale. Default: the first declared one.
    pub fn default_locale(mut self, code: impl Into<String>) -> Self {
        self.default_locale = Some(code.into());
        self
    }

    /// Adds a page. Pages keep the order they are added in (the core adds them in sidebar
    /// order, then the rest by route).
    pub fn page(mut self, page: PageData) -> Self {
        self.pages.push(page);
        self
    }

    /// Adds a sidebar group (display order = order added).
    pub fn group(mut self, group: GroupData) -> Self {
        self.groups.push(group);
        self
    }

    /// Checks and builds: unique locale codes and prefixes, prefixes `""` or `/a[/b…]` with no
    /// trailing `/`, a declared default locale, every page and group in a declared locale,
    /// page routes unique and under their locale's prefix, group routes pages of the same
    /// locale.
    pub fn build(self) -> Result<SiteData, Error> {
        let locales = if self.locales.is_empty() {
            vec![LocaleData::implicit()]
        } else {
            self.locales
        };
        for (i, locale) in locales.iter().enumerate() {
            let p = &locale.prefix;
            let valid = p.is_empty()
                || (p.starts_with('/')
                    && p[1..]
                        .split('/')
                        .all(|s| !s.is_empty() && s != "." && s != ".."));
            if !valid {
                return Err(Error::new(format!(
                    "idioma `{}`: prefixo inválido `{p}` (`\"\"` ou `/xx`, sem `/` no fim)",
                    locale.code
                )));
            }
            if let Some(other) = locales[..i]
                .iter()
                .find(|o| o.code == locale.code || o.prefix == locale.prefix)
            {
                return Err(Error::new(format!(
                    "idiomas `{}` e `{}` com o mesmo código ou prefixo",
                    other.code, locale.code
                )));
            }
        }
        let default_locale = match self.default_locale {
            Some(code) => code,
            None => locales[0].code.clone(),
        };
        let find = |code: &str| locales.iter().find(|l| l.code == code);
        if find(&default_locale).is_none() {
            return Err(Error::new(format!(
                "idioma padrão `{default_locale}` não declarado"
            )));
        }
        let mut routes = std::collections::BTreeSet::new();
        for page in &self.pages {
            let Some(locale) = find(&page.locale) else {
                return Err(Error::new(format!(
                    "página {}: idioma `{}` não declarado",
                    page.route, page.locale
                )));
            };
            if !page.route.starts_with('/') || locale.local_route(&page.route).is_none() {
                return Err(Error::new(format!(
                    "página {}: a rota precisa começar com `/` e com o prefixo `{}` do idioma `{}`",
                    page.route, locale.prefix, locale.code
                )));
            }
            if !routes.insert(page.route.as_str()) {
                return Err(Error::new(format!("rota {} repetida", page.route)));
            }
        }
        for group in &self.groups {
            if find(&group.locale).is_none() {
                return Err(Error::new(format!(
                    "grupo `{}`: idioma `{}` não declarado",
                    group.folder, group.locale
                )));
            }
            for route in &group.routes {
                if !self
                    .pages
                    .iter()
                    .any(|p| p.route == *route && p.locale == group.locale)
                {
                    return Err(Error::new(format!(
                        "grupo `{}` (`{}`): {route} não é uma página desse idioma",
                        group.folder, group.locale
                    )));
                }
            }
        }
        Ok(SiteData {
            title: self.title,
            description: self.description,
            pages: self.pages,
            groups: self.groups,
            locales,
            default_locale,
        })
    }
}

/// Um grupo da sidebar.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GroupData {
    /// Code of the group's locale (`""` for the implicit one).
    pub locale: String,
    /// First-level folder **relative to the locale root** (`components`, also for the pages
    /// under `content/en/components/`).
    pub folder: String,
    /// Título exibido (`Componentes`).
    pub text: String,
    /// Rotas das páginas, na ordem da sidebar.
    pub routes: Vec<String>,
}

impl GroupData {
    /// A group, for [`SiteDataBuilder::group`]. `routes` are full site routes (prefixed).
    pub fn new<I, R>(
        locale: impl Into<String>,
        folder: impl Into<String>,
        text: impl Into<String>,
        routes: I,
    ) -> Self
    where
        I: IntoIterator<Item = R>,
        R: Into<String>,
    {
        Self {
            locale: locale.into(),
            folder: folder.into(),
            text: text.into(),
            routes: routes.into_iter().map(Into::into).collect(),
        }
    }
}

/// Uma página compilada, como os plugins a veem.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PageData {
    /// Rota (`/components/btn`; a home é `/`). Already prefixed with the locale's prefix
    /// (`/en/components/btn`; the home of locale `/en` is `/en`).
    pub route: String,
    /// Caminho do `.md` (relativo à raiz do site).
    pub file: PathBuf,
    /// Code of the page's locale (`""` for the implicit one).
    pub locale: String,
    pub title: String,
    pub description: String,
    /// Front-matter completo, chaves extras (`extra`) incluídas.
    pub front: FrontMatter,
    /// Todos os cabeçalhos, na ordem do texto.
    pub headings: Vec<Heading>,
    /// Markdown da página com as tags expandidas ([`MdComponent::markdown`]), sem front-matter.
    ///
    /// [`MdComponent::markdown`]: crate::MdComponent::markdown
    pub markdown: String,
}

impl PageData {
    /// A page, for [`SiteDataBuilder::page`]: implicit locale (`""`), empty title,
    /// description and Markdown, default front-matter. Fill it in with the `with_*` methods.
    pub fn new(route: impl Into<String>, file: impl Into<PathBuf>) -> Self {
        Self {
            route: route.into(),
            file: file.into(),
            locale: String::new(),
            title: String::new(),
            description: String::new(),
            front: FrontMatter::default(),
            headings: Vec::new(),
            markdown: String::new(),
        }
    }

    pub fn with_locale(mut self, code: impl Into<String>) -> Self {
        self.locale = code.into();
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// The full front-matter (e.g. `layout: Layout::Home` for a home, `extra` keys).
    pub fn with_front(mut self, front: FrontMatter) -> Self {
        self.front = front;
        self
    }

    pub fn with_headings(mut self, headings: Vec<Heading>) -> Self {
        self.headings = headings;
        self
    }

    /// The page's Markdown (tags already expanded, no front-matter).
    pub fn with_markdown(mut self, markdown: impl Into<String>) -> Self {
        self.markdown = markdown.into();
        self
    }

    pub fn is_home(&self) -> bool {
        self.front.layout == Layout::Home
    }

    /// O `.md` de origem é um `index.md` (rota de pasta)?
    pub fn is_folder_index(&self) -> bool {
        self.file.file_stem().is_some_and(|s| s == "index")
    }

    /// Destino de um link **como escrito no Markdown desta página**, se for interno:
    /// `/a/b#x` → `/a/b#x`; `./outra.md#x` → rota da outra página + `#x`. Externos,
    /// âncoras (`#x`) e caminhos relativos que não são `.md` → `None`.
    pub fn resolve_link(&self, url: &str) -> Option<String> {
        if url.starts_with('/') {
            return Some(url.to_string());
        }
        resolve_relative(&self.route, &self.file, url)
    }
}

/// Arquivos emitidos pelos plugins, por caminho relativo à saída (`press-static/`).
#[derive(Debug, Default)]
pub struct Output {
    files: BTreeMap<String, (&'static str, String)>,
    current: &'static str,
}

impl Output {
    pub(crate) fn set_current(&mut self, plugin: &'static str) {
        self.current = plugin;
    }

    /// Emite um arquivo. `path` é relativo (`llms.txt`, `okf/index.md`), com `/`; o mesmo
    /// caminho emitido duas vezes (pelo mesmo plugin ou por outro) é erro.
    pub fn add(
        &mut self,
        path: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<(), Error> {
        let path = path.into();
        let bad = path.is_empty()
            || path.starts_with('/')
            || path.ends_with('/')
            || path.contains('\\')
            || path
                .split('/')
                .any(|c| c.is_empty() || c == "." || c == "..");
        if bad {
            return Err(Error::new(format!(
                "caminho inválido `{path}` (relativo, separado por `/`, sem `.`/`..`)"
            )));
        }
        if let Some((owner, _)) = self.files.get(&path) {
            return Err(Error::new(format!(
                "`{path}` emitido por dois plugins: `{owner}` e `{}`",
                self.current
            )));
        }
        self.files.insert(path, (self.current, content.into()));
        Ok(())
    }

    /// Um arquivo já emitido (por qualquer plugin).
    pub fn get(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(|(_, c)| c.as_str())
    }

    /// Every file emitted so far, by path (for tests of a plugin: run [`Plugin::emit`] on an
    /// `Output::default()` and inspect what it wrote).
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.files
            .iter()
            .map(|(path, (_, content))| (path.as_str(), content.as_str()))
    }

    pub(crate) fn into_files(self) -> BTreeMap<String, String> {
        self.files.into_iter().map(|(k, (_, v))| (k, v)).collect()
    }
}

/// Reescreve o destino dos links inline de um Markdown. `f` recebe a URL como escrita e
/// devolve a nova (ou `None` para manter). Blocos e trechos de código ficam intactos;
/// links de referência (`[x][id]`) e autolinks não são tocados.
pub fn map_links(markdown: &str, mut f: impl FnMut(&str) -> Option<String>) -> String {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &markdown_options());
    let starts = line_starts(markdown);
    let offset = |line: usize, col: usize| starts.get(line - 1).map(|s| s + col - 1);

    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for node in root.descendants() {
        let data = node.data();
        let NodeValue::Link(link) = &data.value else {
            continue;
        };
        let pos = data.sourcepos;
        let (Some(start), Some(end)) = (
            offset(pos.start.line, pos.start.column),
            offset(pos.end.line, pos.end.column),
        ) else {
            continue;
        };
        let Some(span) = markdown.get(start..=end) else {
            continue;
        };
        if !span.ends_with(')') {
            continue;
        }
        // Destino: logo depois do último `](` do trecho, opcionalmente entre `<…>`.
        let Some(open) = span.rfind("](") else {
            continue;
        };
        let mut dest = open + 2;
        if span[dest..].starts_with('<') {
            dest += 1;
        }
        if !span[dest..].starts_with(link.url.as_str()) {
            continue;
        }
        if let Some(new) = f(&link.url) {
            let at = start + dest;
            edits.push((at, at + link.url.len(), new));
        }
    }
    let mut out = markdown.to_string();
    edits.sort_by_key(|(at, _, _)| std::cmp::Reverse(*at));
    for (from, to, text) in edits {
        out.replace_range(from..to, &text);
    }
    out
}

pub(crate) fn markdown_options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.alerts = true;
    options.extension.block_directive = true;
    options
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

pub(crate) fn page_data(page: &crate::page::CompiledPage) -> PageData {
    PageData {
        route: page.route.clone(),
        file: page.file.clone(),
        locale: String::new(),
        title: page.title.clone(),
        description: page.description.clone(),
        front: page.front.clone(),
        headings: page.headings.clone(),
        markdown: page.markdown.clone(),
    }
}

/// Prefixa a mensagem com o nome do plugin.
pub(crate) fn plugin_error(plugin: &str, e: Error) -> Error {
    Error {
        message: format!("plugin `{plugin}`: {}", e.message),
        ..e
    }
}

/// Erro de `check_page`: prefixo do plugin e `arquivo.md:1` se não houver posição.
pub(crate) fn page_error(plugin: &str, file: &Path, e: Error) -> Error {
    plugin_error(plugin, e).at(file, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_inline_links_only() {
        let md = "Veja [a](/x) e [b `c`](/y#z \"t\") e `[n](/x)`.\n\n```md\n[n](/x)\n```\n\n| a |\n|---|\n| [t](/x) |\n";
        let out = map_links(md, |u| u.starts_with("/x").then(|| format!("{u}.md")));
        assert_eq!(
            out,
            "Veja [a](/x.md) e [b `c`](/y#z \"t\") e `[n](/x)`.\n\n```md\n[n](/x)\n```\n\n| a |\n|---|\n| [t](/x.md) |\n"
        );
    }

    #[test]
    fn maps_angle_and_multiline_links() {
        let md = "- [um\n  dois](</a b>) fim\n- [x](https://e.com)\n";
        let out = map_links(md, |u| (u == "/a b").then(|| "/c".to_string()));
        assert_eq!(out, "- [um\n  dois](</c>) fim\n- [x](https://e.com)\n");
    }

    #[test]
    fn locale_routes_and_paths() {
        let en = LocaleData::new("en", "English", "/en");
        assert_eq!(en.local_route("/en").as_deref(), Some("/"));
        assert_eq!(en.local_route("/en/").as_deref(), Some("/"));
        assert_eq!(en.local_route("/en/guide/x").as_deref(), Some("/guide/x"));
        assert_eq!(en.local_route("/english"), None);
        assert_eq!(en.local_route("/pt/x"), None);
        assert_eq!(en.site_route("/"), "/en");
        assert_eq!(en.site_route("/guide"), "/en/guide");
        assert_eq!(en.output_path("llms.txt"), "en/llms.txt");
        assert_eq!(en.url("llms.txt"), "/en/llms.txt");
        let root = LocaleData::implicit();
        assert_eq!(root.local_route("/guide").as_deref(), Some("/guide"));
        assert_eq!(root.site_route("/"), "/");
        assert_eq!(root.output_path("llms.txt"), "llms.txt");
        assert_eq!(root.url("llms.txt"), "/llms.txt");
    }

    #[test]
    fn builder_defaults_to_the_implicit_locale() {
        let site = SiteData::builder("t")
            .page(PageData::new("/", "content/index.md"))
            .group(GroupData::new("", "guide", "Guia", Vec::<String>::new()))
            .build()
            .unwrap();
        assert_eq!(site.locales, [LocaleData::implicit()]);
        assert_eq!(site.default_locale, "");
        assert!(!site.is_multilingual());
        assert_eq!(site.local_route(&site.pages[0]), "/");
    }

    #[test]
    fn builder_checks_the_locale_contract() {
        let two = || {
            SiteData::builder("t")
                .locale("en", "English", "/en")
                .locale("pt", "Português", "/pt")
        };
        let site = two()
            .default_locale("pt")
            .page(PageData::new("/en", "content/en/index.md").with_locale("en"))
            .page(PageData::new("/pt/a", "content/pt/a.md").with_locale("pt"))
            .group(GroupData::new("pt", "a", "A", ["/pt/a"]))
            .build()
            .unwrap();
        assert_eq!(site.default_locale, "pt");
        assert!(site.is_multilingual());
        assert_eq!(site.pages_in("pt").count(), 1);
        assert_eq!(site.groups_in("en").count(), 0);
        assert_eq!(site.local_route(&site.pages[0]), "/");

        let err = |b: SiteDataBuilder| b.build().unwrap_err().to_string();
        assert!(err(two().default_locale("de")).contains("idioma padrão `de`"));
        assert!(err(two().locale("en", "Again", "/x")).contains("mesmo código ou prefixo"));
        assert!(err(two().locale("de", "Deutsch", "/en")).contains("mesmo código ou prefixo"));
        assert!(err(two().locale("de", "Deutsch", "de")).contains("prefixo inválido"));
        assert!(err(two().locale("de", "Deutsch", "/de/")).contains("prefixo inválido"));
        assert!(err(two().page(PageData::new("/x", "x.md"))).contains("idioma `` não declarado"));
        assert!(
            err(two().page(PageData::new("/pt/x", "x.md").with_locale("en")))
                .contains("prefixo `/en`")
        );
        let page = || PageData::new("/en/x", "x.md").with_locale("en");
        assert!(err(two().page(page()).page(page())).contains("repetida"));
        assert!(
            err(two()
                .page(page())
                .group(GroupData::new("pt", "x", "X", ["/en/x"])))
            .contains("não é uma página desse idioma")
        );
    }

    #[test]
    fn duplicate_paths_name_both_plugins() {
        let mut out = Output::default();
        out.set_current("a");
        out.add("x.txt", "1").unwrap();
        out.set_current("b");
        let e = out.add("x.txt", "2").unwrap_err().to_string();
        assert_eq!(e, "`x.txt` emitido por dois plugins: `a` e `b`");
        assert!(out.add("../x", "").is_err());
        assert!(out.add("/x", "").is_err());
        assert!(out.add("a//b", "").is_err());
    }
}
