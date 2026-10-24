//! Plugin `okf` do `leptos-press`: a documentação como *bundle* do
//! [Open Knowledge Format](https://github.com/GoogleCloudPlatform/open-knowledge-format/blob/main/SPEC.md)
//! v0.2, no diretório `okf/` da saída. Usa só a API pública do `leptos-press-build`.
//!
//! - Cada página vira um **conceito** (`/guide/start` → `okf/guide/start.md`; a home →
//!   `okf/overview.md`, porque `index.md` é reservado), com front-matter YAML (`type`,
//!   `title`, `description`, `resource`, `tags`) e o Markdown da página com as tags
//!   expandidas; links internos apontam para os caminhos do bundle (`/guide/start.md`).
//! - `index.md` (reservado) na raiz — com `okf_version: "0.2"` — e em cada subdiretório.
//! - Sem `log.md` e sem datas geradas: a saída é determinística.
//!
//! No `build.rs` do site:
//!
//! ```ignore
//! leptos_press_build::Site::new("content")
//!     // …
//!     .plugin(
//!         leptos_press_okf::Okf::new()
//!             .group_type("components", "Leptos Component")
//!             .group_type("guide", "Guide"),
//!     )
//!     .build()
//!     .unwrap_or_else(|e| panic!("{e}"));
//! ```

mod check;

use std::collections::{BTreeMap, BTreeSet};

use leptos_press_build::yaml::{Mapping, Value};
use leptos_press_build::{Error, Output, PageData, Plugin, SiteData, map_links};

pub use check::check_bundle;

/// Versão da especificação OKF seguida (declarada no `index.md` da raiz).
pub const OKF_VERSION: &str = "0.2";

/// Diretório do bundle na saída.
const DIR: &str = "okf";

/// Gera o bundle OKF do site em `okf/`.
#[derive(Debug, Clone)]
pub struct Okf {
    base_url: Option<String>,
    default_type: String,
    group_types: Vec<(String, String)>,
}

impl Default for Okf {
    fn default() -> Self {
        Self {
            base_url: None,
            default_type: "Documentation Page".into(),
            group_types: Vec::new(),
        }
    }
}

impl Okf {
    pub fn new() -> Self {
        Self::default()
    }

    /// URL pública do site (`https://exemplo.dev/docs`): cada conceito ganha
    /// `resource: <base_url><rota>`. Sem ela, `resource` é omitido.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into().trim_end_matches('/').to_string());
        self
    }

    /// `type` dos conceitos sem tipo próprio nem de grupo. Padrão: `"Documentation Page"`.
    pub fn default_type(mut self, ty: impl Into<String>) -> Self {
        self.default_type = ty.into();
        self
    }

    /// `type` das páginas de um grupo da sidebar (pela pasta: `"components"`). Repetível.
    pub fn group_type(mut self, folder: impl Into<String>, ty: impl Into<String>) -> Self {
        self.group_types.push((folder.into(), ty.into()));
        self
    }

    /// `type` de uma página: o do front-matter → o do grupo → o padrão.
    fn concept_type(&self, site: &SiteData, page: &PageData) -> String {
        if let Some(ty) = page.front.str("type") {
            return ty.to_string();
        }
        let group = site.groups.iter().find(|g| g.routes.contains(&page.route));
        group
            .and_then(|g| self.group_types.iter().find(|(f, _)| *f == g.folder))
            .map_or_else(|| self.default_type.clone(), |(_, t)| t.clone())
    }

    fn front_matter(&self, site: &SiteData, page: &PageData) -> Result<String, Error> {
        let mut front = Mapping::new();
        let mut put = |k: &str, v: Value| front.insert(Value::String(k.into()), v);
        put("type", self.concept_type(site, page).into());
        put("title", page.title.clone().into());
        if !page.description.is_empty() {
            put("description", page.description.clone().into());
        }
        if let Some(base) = &self.base_url {
            put("resource", format!("{base}{}", page.route).into());
        }
        if let Some(tags) = tags(page)? {
            put("tags", tags);
        }
        if let Some(extra) = okf_map(page)? {
            for (k, v) in extra {
                front.insert(k.clone(), v.clone());
            }
        }
        let yaml = leptos_press_build::yaml::to_string(&Value::Mapping(front))
            .map_err(|e| Error::new(format!("front-matter do conceito: {e}")))?;
        Ok(yaml)
    }

    fn concept(&self, site: &SiteData, page: &PageData) -> Result<String, Error> {
        let front = self.front_matter(site, page)?;
        let body = map_links(&page.markdown, |url| {
            let target = page.resolve_link(url)?;
            let (path, fragment) = target.split_once('#').unwrap_or((&target, ""));
            let route = normalize_route(path);
            let concept = site.page(&route)?;
            let mut out = format!("/{}", concept_path(&concept.route));
            if !fragment.is_empty() {
                out.push('#');
                out.push_str(fragment);
            }
            Some(out)
        });
        Ok(format!("---\n{front}---\n\n{}", body.trim_start()))
    }
}

impl Plugin for Okf {
    fn name(&self) -> &'static str {
        "okf"
    }

    fn check_page(&self, page: &PageData) -> Result<(), Error> {
        let path = concept_path(&page.route);
        let name = path.rsplit('/').next().unwrap_or(&path);
        if name == "index.md" || name == "log.md" {
            return Err(Error::new(format!(
                "a rota {} vira `{DIR}/{path}`, nome reservado no OKF; renomeie a página",
                page.route
            )));
        }
        match page.front.extra.get("type") {
            None => {}
            Some(Value::String(t)) if !t.trim().is_empty() => {}
            Some(_) => {
                return Err(Error::new(
                    "`type` do front-matter precisa ser texto não vazio",
                ));
            }
        }
        tags(page)?;
        if let Some(extra) = okf_map(page)? {
            match extra.get("type") {
                None => {}
                Some(Value::String(t)) if !t.trim().is_empty() => {}
                Some(_) => return Err(Error::new("`okf.type` precisa ser texto não vazio")),
            }
        }
        Ok(())
    }

    fn emit(&self, site: &SiteData, out: &mut Output) -> Result<(), Error> {
        if site.page("/").is_some() && site.page("/overview").is_some() {
            return Err(Error::new(
                "a home e a página /overview iriam ambas para `okf/overview.md`",
            ));
        }
        let mut files: BTreeMap<String, String> = BTreeMap::new();
        for page in &site.pages {
            files.insert(concept_path(&page.route), self.concept(site, page)?);
        }
        files.insert("index.md".into(), root_index(site));
        for (dir, text) in dir_indexes(site) {
            files.insert(format!("{dir}/index.md"), text);
        }
        // O plugin nunca publica um bundle fora da especificação.
        check::check_files(&files)
            .map_err(|errors| Error::new(format!("bundle não conforme: {}", errors.join("; "))))?;
        for (path, text) in files {
            out.add(format!("{DIR}/{path}"), text)?;
        }
        Ok(())
    }
}

/// Caminho do conceito no bundle: `/` → `overview.md`; `/guide/start` → `guide/start.md`.
fn concept_path(route: &str) -> String {
    if route == "/" {
        "overview.md".into()
    } else {
        format!("{}.md", route.trim_start_matches('/'))
    }
}

fn concept_dir(route: &str) -> &str {
    let path = route.trim_start_matches('/');
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn normalize_route(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".into()
    } else {
        trimmed.to_string()
    }
}

/// `tags` da página: lista de textos (ou um texto só → lista de um).
fn tags(page: &PageData) -> Result<Option<Value>, Error> {
    match page.front.extra.get("tags") {
        None => Ok(None),
        Some(Value::String(t)) => Ok(Some(Value::Sequence(vec![t.clone().into()]))),
        Some(Value::Sequence(items)) if items.iter().all(|v| v.as_str().is_some()) => {
            Ok(Some(Value::Sequence(items.clone())))
        }
        Some(_) => Err(Error::new("`tags` precisa ser uma lista de textos")),
    }
}

/// Mapa `okf:` do front-matter da página (mesclado no do conceito).
fn okf_map(page: &PageData) -> Result<Option<&Mapping>, Error> {
    match page.front.extra.get("okf") {
        None => Ok(None),
        Some(Value::Mapping(m)) => {
            if m.keys().any(|k| k.as_str().is_none()) {
                return Err(Error::new("as chaves de `okf` precisam ser texto"));
            }
            Ok(Some(m))
        }
        Some(_) => Err(Error::new(
            "`okf` precisa ser um mapa (ex.: `okf: { status: draft }`)",
        )),
    }
}

/// `* [título](/caminho.md) - descrição`
fn entry(page: &PageData) -> String {
    let mut line = format!("* [{}](/{})", page.title, concept_path(&page.route));
    if !page.description.is_empty() {
        line.push_str(&format!(" - {}", page.description));
    }
    line.push('\n');
    line
}

/// `index.md` da raiz: título e descrição do site, os conceitos da raiz (home primeiro) e uma
/// seção `##` por grupo da sidebar (mais uma por pasta de páginas fora dos grupos).
fn root_index(site: &SiteData) -> String {
    let mut text = format!(
        "---\nokf_version: \"{OKF_VERSION}\"\n---\n\n# {}\n\n",
        site.title
    );
    if !site.description.is_empty() {
        text.push_str(&format!("{}\n\n", site.description));
    }
    let mut listed: BTreeSet<&str> = BTreeSet::new();
    let home = site.pages.iter().filter(|p| p.route == "/");
    let root = site
        .pages
        .iter()
        .filter(|p| p.route != "/" && concept_dir(&p.route).is_empty());
    let top: String = home
        .chain(root)
        .map(|p| {
            listed.insert(&p.route);
            entry(p)
        })
        .collect();
    if !top.is_empty() {
        text.push_str(&format!("{top}\n"));
    }
    for group in &site.groups {
        let items: String = group
            .routes
            .iter()
            .filter_map(|r| site.page(r))
            .map(|p| {
                listed.insert(&p.route);
                entry(p)
            })
            .collect();
        if !items.is_empty() {
            text.push_str(&format!("## {}\n\n{items}\n", group.text));
        }
    }
    let mut rest: BTreeMap<&str, String> = BTreeMap::new();
    for page in site
        .pages
        .iter()
        .filter(|p| !listed.contains(p.route.as_str()))
    {
        let folder = page
            .route
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or("");
        rest.entry(folder).or_default().push_str(&entry(page));
    }
    for (folder, items) in rest {
        text.push_str(&format!("## {folder}\n\n{items}\n"));
    }
    format!("{}\n", text.trim_end())
}

/// Um `index.md` (sem front-matter) por subdiretório: os conceitos dele, na ordem do site, e
/// os subdiretórios.
fn dir_indexes(site: &SiteData) -> BTreeMap<String, String> {
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    for page in &site.pages {
        let mut dir = concept_dir(&page.route);
        while !dir.is_empty() {
            dirs.insert(dir.to_string());
            dir = dir.rsplit_once('/').map_or("", |(d, _)| d);
        }
    }
    dirs.iter()
        .map(|dir| {
            let title = site.groups.iter().find(|g| g.folder == *dir).map_or_else(
                || dir.rsplit('/').next().unwrap_or(dir).to_string(),
                |g| g.text.clone(),
            );
            let mut text = format!("# {title}\n\n");
            for page in site.pages.iter().filter(|p| concept_dir(&p.route) == dir) {
                text.push_str(&entry(page));
            }
            for sub in dirs
                .iter()
                .filter(|d| d.rsplit_once('/').is_some_and(|(parent, _)| parent == dir))
            {
                let name = sub.rsplit('/').next().unwrap_or(sub);
                text.push_str(&format!("* [{name}](/{sub}/index.md)\n"));
            }
            (dir.clone(), format!("{}\n", text.trim_end()))
        })
        .collect()
}
