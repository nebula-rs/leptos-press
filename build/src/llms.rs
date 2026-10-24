//! Plugin interno **temporário**: `llms.txt`, `llms-full.txt` e uma cópia `.md` por página.

use crate::error::Error;
use crate::plugin::{Output, PageData, Plugin, SiteData};

/// Gera `llms.txt` ([llmstxt.org](https://llmstxt.org)), `llms-full.txt` e `<rota>.md`.
///
/// Temporário: prova a API de plugins sem mudar a saída — **sai na R-41**, para o crate
/// `leptos-press-llms`.
pub struct LlmsPlugin;

impl Plugin for LlmsPlugin {
    fn name(&self) -> &'static str {
        "llms"
    }

    fn emit(&self, site: &SiteData, out: &mut Output) -> Result<(), Error> {
        for page in &site.pages {
            out.add(md_name(page), page.markdown.clone())?;
        }
        let link = |route: &str| site.page(route).map(md_name).unwrap_or_default();

        let mut txt = format!("# {}\n\n", site.title);
        if !site.description.is_empty() {
            txt.push_str(&format!("> {}\n\n", site.description));
        }
        for group in &site.groups {
            txt.push_str(&format!("## {}\n\n", group.text));
            for route in &group.routes {
                let Some(page) = site.page(route) else {
                    continue;
                };
                txt.push_str(&format!("- [{}](/{})", page.title, link(route)));
                if !page.description.is_empty() {
                    txt.push_str(&format!(": {}", page.description));
                }
                txt.push('\n');
            }
            txt.push('\n');
        }
        out.add("llms.txt", format!("{}\n", txt.trim_end()))?;

        // Home primeiro, depois a ordem do site (sidebar e o resto).
        let home = site.pages.iter().filter(|p| p.route == "/");
        let rest = site.pages.iter().filter(|p| p.route != "/");
        let full = home
            .chain(rest)
            .map(|page| {
                format!(
                    "---\nurl: /{}\n---\n\n{}",
                    md_name(page),
                    page.markdown.trim_end()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        out.add("llms-full.txt", format!("{full}\n"))
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
