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

use leptos_press_build::{Error, Output, PageData, Plugin, SiteData};

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

    fn included(&self, page: &PageData) -> bool {
        !self.exclude.iter().any(|prefix| {
            page.route == *prefix
                || (prefix != "/" && page.route.starts_with(&format!("{prefix}/")))
        })
    }

    /// Destino de uma página no `llms.txt`/`llms-full.txt`.
    fn url(&self, page: &PageData) -> String {
        if self.page_copies {
            format!("/{}", md_name(page))
        } else {
            page.route.clone()
        }
    }

    fn list(&self, site: &SiteData, routes: &[String]) -> String {
        let mut txt = String::new();
        for page in routes.iter().filter_map(|r| site.page(r)) {
            if !self.included(page) {
                continue;
            }
            txt.push_str(&format!("- [{}]({})", page.title, self.url(page)));
            if !page.description.is_empty() {
                txt.push_str(&format!(": {}", page.description));
            }
            txt.push('\n');
        }
        txt
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
        let pages: Vec<&PageData> = site.pages.iter().filter(|p| self.included(p)).collect();

        if self.page_copies {
            for page in &pages {
                out.add(md_name(page), page.markdown.clone())?;
            }
        }

        let mut txt = format!("# {}\n\n", site.title);
        let summary = self.summary.as_deref().unwrap_or(&site.description);
        if !summary.is_empty() {
            txt.push_str(&format!("> {summary}\n\n"));
        }
        let mut optional = String::new();
        for group in &site.groups {
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
        if !optional.is_empty() {
            txt.push_str(&format!("## Optional\n\n{optional}"));
        }
        out.add("llms.txt", format!("{}\n", txt.trim_end()))?;

        if self.full {
            // Home primeiro, depois a ordem do site (sidebar e o resto).
            let home = pages.iter().filter(|p| p.route == "/");
            let rest = pages.iter().filter(|p| p.route != "/");
            let full = home
                .chain(rest)
                .map(|page| {
                    format!(
                        "---\nurl: {}\n---\n\n{}",
                        self.url(page),
                        page.markdown.trim_end()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            out.add("llms-full.txt", format!("{full}\n"))?;
        }
        Ok(())
    }
}

/// Caminho da cópia `.md` (relativo à saída): `/` → `index.md`; pasta com `index.md` →
/// `pasta/index.md`; página → `rota.md`.
fn md_name(page: &PageData) -> String {
    if page.route == "/" {
        return "index.md".into();
    }
    let trimmed = page.route.trim_start_matches('/');
    if page.is_folder_index() {
        format!("{trimmed}/index.md")
    } else {
        format!("{trimmed}.md")
    }
}
