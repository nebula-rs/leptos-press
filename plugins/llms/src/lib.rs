//! Plugin `llms` do `leptos-press`: `llms.txt` ([llmstxt.org](https://llmstxt.org)),
//! `llms-full.txt` e uma cópia `.md` por página. Usa só a API pública do `leptos-press-build`.
//!
//! No `build.rs` do site:
//!
//! ```ignore
//! leptos_press_build::Site::new("content")
//!     // …
//!     .plugin(leptos_press_llms::Llms::default())
//!     .build()
//!     .unwrap_or_else(|e| panic!("{e}"));
//! ```
//!
//! # Locales
//!
//! The plugin always generates for **every** locale of the site ([`SiteData::locales`]); there
//! is no option to pick some. This is deliberate: the search of each locale reads that locale's
//! own `llms-full.txt` (ADR-0015), so every locale needs one.
//!
//! For each locale, under its prefix (`""` or `/en`):
//!
//! - `<prefix>/llms.txt`: only the pages of that locale, with a link to the `llms-full.txt` of
//!   the same locale (when `full(true)`);
//! - `<prefix>/llms-full.txt`: only the pages of that locale;
//! - `<route>.md` copies (page routes already carry the prefix: `/en/guide/start.md`).
//!
//! With more than one locale, the root `/llms.txt` is the default locale's, plus a
//! `## Other languages` section that links the `llms.txt` of the other locales (by their
//! `label`), and the root `/llms-full.txt` is the default locale's. (A file at a sub-path
//! covers the URLs below it, as llmstxt.org allows.) With a single locale — a site that
//! declares none has one implicit locale with prefix `""` — there is only the root pair.
//!
//! [`exclude`](Llms::exclude) prefixes and [`optional_group`](Llms::optional_group) folders
//! are relative to the root of each locale and apply to all of them: `exclude("/guide")`
//! leaves out `/en/guide…` and `/pt/guide…`.
//!
//! # Formato do `llms-full.txt` (API pública)
//!
//! O `llms-full.txt` é lido pela busca do `leptos-press` (`Site::search(true)`, ADR-0015);
//! por isso o formato é **contrato**, protegido pelo teste `full_format_contract`. Mudá-lo
//! quebra a busca: o leitor (`leptos_press::search`) muda junto.
//!
//! ```text
//! ---
//! url: /index.md
//! ---
//!
//! # Início
//!
//! ---
//! url: /guide/start.md
//! ---
//!
//! # Começando
//! …
//! ```
//!
//! - One file per locale, with only that locale's pages (see *Locales*).
//! - Uma página após a outra: **a home primeiro**, depois as páginas na ordem do site (a da
//!   sidebar, depois as que não estão nela, por rota). Páginas sob um `exclude` não entram.
//! - Cada página começa por um bloco de três linhas, **no início de linha**: `---`,
//!   `url: <caminho>`, `---`; depois uma linha em branco e o Markdown da página (tags expandidas
//!   por `MdComponent::markdown`, sem front-matter, sem espaços no fim). Entre duas páginas, uma
//!   linha em branco. O arquivo termina com uma quebra de linha.
//! - `<caminho>` com as cópias (`page_copies(true)`, padrão) é o da cópia `.md`: `/index.md`
//!   (home), `/pasta/index.md` (página de um `index.md` de pasta) ou `/rota.md`; sem as cópias
//!   (`page_copies(false)`), é a própria rota (`/`, `/pasta`, `/guide/start`). In a locale
//!   with a prefix, the path carries it: `/en/index.md`, `/en/guide/start.md` (or `/en`,
//!   `/en/guide/start`).
//! - O Markdown de uma página pode conter `---` e até `url: …` dentro de **blocos de código
//!   cercados** (```` ``` ```` / `~~~`): quem lê o arquivo ignora o que está dentro deles.

use leptos_press_build::{Error, LocaleData, Output, PageData, Plugin, SiteData};

/// Gera `llms.txt`, `llms-full.txt` e `<rota>.md`. Padrão: os três, com todas as páginas.
#[derive(Debug, Clone)]
pub struct Llms {
    full: bool,
    page_copies: bool,
    exclude: Vec<String>,
    optional_groups: Vec<String>,
    summary: Option<String>,
}

impl Default for Llms {
    fn default() -> Self {
        Self {
            full: true,
            page_copies: true,
            exclude: Vec::new(),
            optional_groups: Vec::new(),
            summary: None,
        }
    }
}

impl Llms {
    pub fn new() -> Self {
        Self::default()
    }

    /// Gera o `llms-full.txt` (todas as páginas num arquivo). Padrão: `true`.
    pub fn full(mut self, on: bool) -> Self {
        self.full = on;
        self
    }

    /// Gera a cópia `<rota>.md` de cada página. Padrão: `true`. Sem as cópias, os links do
    /// `llms.txt` (e o `url:` do `llms-full.txt`) apontam para a rota HTML da página.
    pub fn page_copies(mut self, on: bool) -> Self {
        self.page_copies = on;
        self
    }

    /// Deixa de fora as páginas sob um prefixo de rota (`/internal` exclui `/internal` e
    /// `/internal/…`, não `/internals`; `/` exclui só a home). Repetível.
    pub fn exclude(mut self, route_prefix: impl Into<String>) -> Self {
        let prefix = route_prefix.into();
        let trimmed = prefix.trim_end_matches('/');
        self.exclude.push(if trimmed.is_empty() {
            "/".into()
        } else {
            trimmed.to_string()
        });
        self
    }

    /// Manda um grupo da sidebar (pela pasta, ex.: `"components"`) para a seção `## Optional`
    /// do `llms.txt` — o que um agente pode omitir com pouco contexto. Repetível.
    pub fn optional_group(mut self, folder: impl Into<String>) -> Self {
        self.optional_groups.push(folder.into());
        self
    }

    /// Texto do bloco `>` do `llms.txt`. Padrão: a descrição do site.
    pub fn summary(mut self, text: impl Into<String>) -> Self {
        self.summary = Some(text.into());
        self
    }

    /// Not under an `exclude` prefix (relative to the page's locale root).
    fn included(&self, site: &SiteData, page: &PageData) -> bool {
        let route = site.local_route(page);
        !self.exclude.iter().any(|prefix| {
            route == *prefix || (prefix != "/" && route.starts_with(&format!("{prefix}/")))
        })
    }

    /// Destino de uma página no `llms.txt`/`llms-full.txt`.
    fn url(&self, site: &SiteData, page: &PageData) -> String {
        if self.page_copies {
            format!("/{}", md_name(site, page))
        } else {
            page.route.clone()
        }
    }

    fn list(&self, site: &SiteData, routes: &[String]) -> String {
        let mut txt = String::new();
        for page in routes.iter().filter_map(|r| site.page(r)) {
            if !self.included(site, page) {
                continue;
            }
            txt.push_str(&format!("- [{}]({})", page.title, self.url(site, page)));
            if !page.description.is_empty() {
                txt.push_str(&format!(": {}", page.description));
            }
            txt.push('\n');
        }
        txt
    }

    /// The `llms.txt` of a locale. `full` = URL of the `llms-full.txt` to link; `others` = the
    /// other locales, for the root file of a multilingual site.
    fn index(
        &self,
        site: &SiteData,
        locale: &LocaleData,
        full: Option<&str>,
        others: &[&LocaleData],
    ) -> String {
        let mut txt = format!("# {}\n\n", site.title);
        let summary = self.summary.as_deref().unwrap_or(&site.description);
        if !summary.is_empty() {
            txt.push_str(&format!("> {summary}\n\n"));
        }
        if let Some(full) = full {
            txt.push_str(&format!(
                "Full documentation in a single file: [llms-full.txt]({full})\n\n"
            ));
        }
        let mut optional = String::new();
        for group in site.groups_in(&locale.code) {
            let list = self.list(site, &group.routes);
            if list.is_empty() {
                continue;
            }
            if self.optional_groups.contains(&group.folder) {
                optional.push_str(&list);
            } else {
                txt.push_str(&format!("## {}\n\n{list}\n", group.text));
            }
        }
        if !others.is_empty() {
            txt.push_str("## Other languages\n\n");
            for other in others {
                let label = if other.label.is_empty() {
                    &other.code
                } else {
                    &other.label
                };
                txt.push_str(&format!(
                    "- [{label}]({}): {}\n",
                    other.url("llms.txt"),
                    other.code
                ));
            }
            txt.push('\n');
        }
        if !optional.is_empty() {
            txt.push_str(&format!("## Optional\n\n{optional}"));
        }
        format!("{}\n", txt.trim_end())
    }

    /// The `llms-full.txt` of a locale: home first, then the site order.
    fn full_text(&self, site: &SiteData, pages: &[&PageData]) -> String {
        let is_home = |p: &&&PageData| site.local_route(p) == "/";
        let home = pages.iter().filter(is_home);
        let rest = pages.iter().filter(|p| !is_home(p));
        let full = home
            .chain(rest)
            .map(|page| {
                format!(
                    "---\nurl: {}\n---\n\n{}",
                    self.url(site, page),
                    page.markdown.trim_end()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        format!("{full}\n")
    }
}

impl Plugin for Llms {
    fn name(&self) -> &'static str {
        "llms"
    }

    fn emit(&self, site: &SiteData, out: &mut Output) -> Result<(), Error> {
        for folder in &self.optional_groups {
            if !site.groups.iter().any(|g| g.folder == *folder) {
                return Err(Error::new(format!(
                    "`optional_group(\"{folder}\")`: não há grupo da sidebar com essa pasta"
                )));
            }
        }
        let multilingual = site.is_multilingual();
        for locale in &site.locales {
            let is_default = locale.code == site.default_locale;
            if multilingual && locale.prefix.is_empty() && !is_default {
                return Err(Error::new(format!(
                    "o idioma `{}` não tem prefixo, mas não é o padrão: o `llms.txt` da raiz \
                     é o do idioma padrão",
                    locale.code
                )));
            }
            let pages: Vec<&PageData> = site
                .pages_in(&locale.code)
                .filter(|p| self.included(site, p))
                .collect();

            if self.page_copies {
                for page in &pages {
                    out.add(md_name(site, page), page.markdown.clone())?;
                }
            }

            // The root files of a multilingual site are the default locale's, plus links to
            // the other locales; a default locale with a prefix also keeps its own pair.
            let at_root = locale.prefix.is_empty();
            let others: Vec<&LocaleData> = if multilingual && is_default {
                site.locales
                    .iter()
                    .filter(|l| l.code != locale.code)
                    .collect()
            } else {
                Vec::new()
            };
            let full_url = |l: &LocaleData| self.full.then(|| l.url("llms-full.txt"));
            if !at_root || !is_default {
                let txt = self.index(site, locale, full_url(locale).as_deref(), &[]);
                out.add(locale.output_path("llms.txt"), txt)?;
            }
            if is_default {
                let root = LocaleData::implicit();
                let txt = self.index(site, locale, full_url(&root).as_deref(), &others);
                out.add("llms.txt", txt)?;
            }
            if self.full {
                let full = self.full_text(site, &pages);
                if !at_root {
                    out.add(locale.output_path("llms-full.txt"), full.clone())?;
                }
                if is_default {
                    out.add("llms-full.txt", full)?;
                }
            }
        }
        Ok(())
    }
}

/// Caminho da cópia `.md` (relativo à saída): `/` → `index.md`; pasta com `index.md` →
/// `pasta/index.md`; página → `rota.md`. The home of a locale with prefix `/en` → `en/index.md`.
fn md_name(site: &SiteData, page: &PageData) -> String {
    if site.local_route(page) == "/" {
        return site
            .locale(&page.locale)
            .map_or_else(|| "index.md".into(), |l| l.output_path("index.md"));
    }
    let trimmed = page.route.trim_start_matches('/');
    if page.is_folder_index() {
        format!("{trimmed}/index.md")
    } else {
        format!("{trimmed}.md")
    }
}
