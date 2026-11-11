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
//! # Locales
//!
//! [`Okf::locales`] picks the locales that go into the bundle (default: all of the site's).
//!
//! - **One locale** (always the case for a single-language site): the bundle is `okf/` as
//!   described above. Concept IDs are relative to the locale root, **without** its prefix
//!   (`/en/components/btn` → `okf/components/btn.md`; the locale's home → `okf/overview.md`).
//! - **Several locales**: one directory per locale code (`okf/en/…`, `okf/pt/…`), each with
//!   its own `index.md` (like the root one, but without front-matter) and subdirectory
//!   indexes; the root `okf/index.md` (with `okf_version`) lists the locales.
//! - Internal links to pages of a locale in the bundle point to its bundle path
//!   (`/components/btn.md`, or `/pt/components/btn.md` with several locales); links to pages
//!   of locales left out stay as site URLs (absolute with [`Okf::base_url`]).
//! - [`Okf::group_type`] folders are relative to the locale root; `resource` is the page's
//!   real URL (with the locale prefix).
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
use leptos_press_build::{Error, LocaleData, Output, PageData, Plugin, SiteData, map_links};

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
    /// `None` = every locale of the site.
    locales: Option<Vec<String>>,
}

impl Default for Okf {
    fn default() -> Self {
        Self {
            base_url: None,
            default_type: "Documentation Page".into(),
            group_types: Vec::new(),
            locales: None,
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

    /// The locales that go into the bundle, by code (`["en"]`). Default: every locale of the
    /// site. One locale → the bundle is `okf/` with IDs without the locale prefix; several →
    /// one directory per locale (`okf/en/`, `okf/pt/`). A code the site does not have (or an
    /// empty list) is a build error.
    pub fn locales<I, S>(mut self, codes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.locales = Some(codes.into_iter().map(Into::into).collect());
        self
    }

    /// Is the locale in the bundle (before checking the codes against the site)?
    fn selects(&self, code: &str) -> bool {
        self.locales
            .as_ref()
            .is_none_or(|codes| codes.iter().any(|c| c == code))
    }

    /// The selected locales, in site order.
    fn selected<'a>(&self, site: &'a SiteData) -> Result<Vec<&'a LocaleData>, Error> {
        let Some(codes) = &self.locales else {
            return Ok(site.locales.iter().collect());
        };
        let known = || {
            site.locales
                .iter()
                .map(|l| format!("`{}`", l.code))
                .collect::<Vec<_>>()
                .join(", ")
        };
        if codes.is_empty() {
            return Err(Error::new(format!(
                "`locales([])`: nenhum idioma selecionado (idiomas do site: {})",
                known()
            )));
        }
        if let Some(bad) = codes.iter().find(|c| site.locale(c).is_none()) {
            return Err(Error::new(format!(
                "`locales(...)`: o site não tem o idioma `{bad}` (idiomas do site: {})",
                known()
            )));
        }
        Ok(site
            .locales
            .iter()
            .filter(|l| codes.contains(&l.code))
            .collect())
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
        let group = site
            .groups_in(&page.locale)
            .find(|g| g.routes.contains(&page.route));
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

    fn concept(&self, bundle: &Bundle, page: &PageData) -> Result<String, Error> {
        let site = bundle.site;
        let front = self.front_matter(site, page)?;
        let body = map_links(&page.markdown, |url| {
            let target = page.resolve_link(url)?;
            let (path, fragment) = target.split_once('#').unwrap_or((&target, ""));
            let route = normalize_route(path);
            let concept = site.page(&route)?;
            // A page of a locale left out of the bundle: its URL on the site.
            let mut out = match bundle.path(concept) {
                Some(path) => format!("/{path}"),
                None => format!(
                    "{}{}",
                    self.base_url.as_deref().unwrap_or(""),
                    concept.route
                ),
            };
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
        if !self.selects(&page.locale) {
            return Ok(());
        }
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
        let selected = self.selected(site)?;
        let multi = selected.len() > 1;
        let mut files: BTreeMap<String, String> = BTreeMap::new();
        for &locale in &selected {
            if multi
                && (locale.code.is_empty()
                    || locale.code.contains(['/', '\\'])
                    || locale.code == "."
                    || locale.code == "..")
            {
                return Err(Error::new(format!(
                    "o código de idioma `{}` não serve de diretório do bundle",
                    locale.code
                )));
            }
            let bundle = Bundle {
                site,
                locale,
                dir: if multi {
                    format!("{}/", locale.code)
                } else {
                    String::new()
                },
                selected: &selected,
            };
            let local = |p: &&PageData| site.local_route(p);
            if site.pages_in(&locale.code).any(|p| local(&p) == "/")
                && site
                    .pages_in(&locale.code)
                    .any(|p| local(&p) == "/overview")
            {
                return Err(Error::new(format!(
                    "a home e a página /overview iriam ambas para `{DIR}/{}overview.md`",
                    bundle.dir
                )));
            }
            for page in site.pages_in(&locale.code) {
                files.insert(bundle.own_path(page), self.concept(&bundle, page)?);
            }
            files.insert(
                format!("{}index.md", bundle.dir),
                locale_index(&bundle, !multi),
            );
            for (dir, text) in dir_indexes(&bundle) {
                files.insert(format!("{}{dir}/index.md", bundle.dir), text);
            }
        }
        if multi {
            files.insert("index.md".into(), languages_index(site, &selected));
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

/// One locale of the bundle being emitted.
struct Bundle<'a> {
    site: &'a SiteData,
    locale: &'a LocaleData,
    /// Directory of the locale in the bundle: `""` (one locale) or `"en/"`.
    dir: String,
    selected: &'a [&'a LocaleData],
}

impl Bundle<'_> {
    /// Path in the bundle of a page of this locale.
    fn own_path(&self, page: &PageData) -> String {
        format!("{}{}", self.dir, concept_path(&self.site.local_route(page)))
    }

    /// Path in the bundle of any page, or `None` if its locale is not in the bundle.
    fn path(&self, page: &PageData) -> Option<String> {
        if page.locale == self.locale.code {
            return Some(self.own_path(page));
        }
        self.selected.iter().find(|l| l.code == page.locale)?;
        // Several locales (otherwise only this one is selected): the directory is the code.
        Some(format!(
            "{}/{}",
            page.locale,
            concept_path(&self.site.local_route(page))
        ))
    }

    /// The pages of this locale.
    fn pages(&self) -> impl Iterator<Item = &PageData> {
        self.site.pages_in(&self.locale.code)
    }

    fn local(&self, page: &PageData) -> String {
        self.site.local_route(page)
    }

    /// `* [título](/caminho.md) - descrição`
    fn entry(&self, page: &PageData) -> String {
        let mut line = format!("* [{}](/{})", page.title, self.own_path(page));
        if !page.description.is_empty() {
            line.push_str(&format!(" - {}", page.description));
        }
        line.push('\n');
        line
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

/// `index.md` da raiz: título e descrição do site, os conceitos da raiz (home primeiro) e uma
/// seção `##` por grupo da sidebar (mais uma por pasta de páginas fora dos grupos).
///
/// With several locales, the same for each locale directory, without front-matter
/// (`root = false`) and with the locale's label in the title.
fn locale_index(bundle: &Bundle, root: bool) -> String {
    let site = bundle.site;
    let mut text = if root {
        format!(
            "---\nokf_version: \"{OKF_VERSION}\"\n---\n\n# {}\n\n",
            site.title
        )
    } else {
        format!("# {} ({})\n\n", site.title, label(bundle.locale))
    };
    if !site.description.is_empty() {
        text.push_str(&format!("{}\n\n", site.description));
    }
    let mut listed: BTreeSet<&str> = BTreeSet::new();
    let home = bundle.pages().filter(|p| bundle.local(p) == "/");
    let top_level = bundle.pages().filter(|p| {
        let local = bundle.local(p);
        local != "/" && concept_dir(&local).is_empty()
    });
    let top: String = home
        .chain(top_level)
        .map(|p| {
            listed.insert(&p.route);
            bundle.entry(p)
        })
        .collect();
    if !top.is_empty() {
        text.push_str(&format!("{top}\n"));
    }
    for group in site.groups_in(&bundle.locale.code) {
        let items: String = group
            .routes
            .iter()
            .filter_map(|r| site.page(r))
            .map(|p| {
                listed.insert(&p.route);
                bundle.entry(p)
            })
            .collect();
        if !items.is_empty() {
            text.push_str(&format!("## {}\n\n{items}\n", group.text));
        }
    }
    let mut rest: BTreeMap<String, String> = BTreeMap::new();
    for page in bundle
        .pages()
        .filter(|p| !listed.contains(p.route.as_str()))
    {
        let local = bundle.local(page);
        let folder = local
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();
        rest.entry(folder)
            .or_default()
            .push_str(&bundle.entry(page));
    }
    for (folder, items) in rest {
        text.push_str(&format!("## {folder}\n\n{items}\n"));
    }
    format!("{}\n", text.trim_end())
}

/// Root `index.md` of a bundle with several locales: `okf_version`, the site's title and
/// description and one entry per locale (`* [English](/en/index.md) - en`).
fn languages_index(site: &SiteData, locales: &[&LocaleData]) -> String {
    let mut text = format!(
        "---\nokf_version: \"{OKF_VERSION}\"\n---\n\n# {}\n\n",
        site.title
    );
    if !site.description.is_empty() {
        text.push_str(&format!("{}\n\n", site.description));
    }
    for locale in locales {
        text.push_str(&format!(
            "* [{}](/{}/index.md) - {}\n",
            label(locale),
            locale.code,
            locale.code
        ));
    }
    text
}

/// The locale's label, or its code when it has none.
fn label(locale: &LocaleData) -> &str {
    if locale.label.is_empty() {
        &locale.code
    } else {
        &locale.label
    }
}

/// Um `index.md` (sem front-matter) por subdiretório: os conceitos dele, na ordem do site, e
/// os subdiretórios. Keys are relative to the locale directory.
fn dir_indexes(bundle: &Bundle) -> BTreeMap<String, String> {
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    for page in bundle.pages() {
        let local = bundle.local(page);
        let mut dir = concept_dir(&local);
        while !dir.is_empty() {
            dirs.insert(dir.to_string());
            dir = dir.rsplit_once('/').map_or("", |(d, _)| d);
        }
    }
    dirs.iter()
        .map(|dir| {
            let title = bundle
                .site
                .groups_in(&bundle.locale.code)
                .find(|g| g.folder == *dir)
                .map_or_else(
                    || dir.rsplit('/').next().unwrap_or(dir).to_string(),
                    |g| g.text.clone(),
                );
            let mut text = format!("# {title}\n\n");
            for page in bundle
                .pages()
                .filter(|p| concept_dir(&bundle.local(p)) == dir)
            {
                text.push_str(&bundle.entry(page));
            }
            for sub in dirs
                .iter()
                .filter(|d| d.rsplit_once('/').is_some_and(|(parent, _)| parent == dir))
            {
                let name = sub.rsplit('/').next().unwrap_or(sub);
                text.push_str(&format!("* [{name}](/{}{sub}/index.md)\n", bundle.dir));
            }
            (dir.clone(), format!("{}\n", text.trim_end()))
        })
        .collect()
}
