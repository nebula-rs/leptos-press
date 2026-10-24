//! Plugins de build ([ADR-0011]): recebem o site já compilado, validam páginas e emitem
//! arquivos estáticos (`press-static/`). Só leitura: um plugin não altera páginas nem código.
//!
//! Ordem em [`Site::generate`](crate::Site::generate): compilar as páginas → `check_page` de
//! cada plugin (na ordem de registro, página a página) → `emit` de cada plugin → validar os
//! links internos, aceitando como destino as rotas **e** os arquivos emitidos.
//!
//! [ADR-0011]: https://github.com/nebula-rs/leptos-next/blob/main/docs/decisions/0011-plugins-do-press.md

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
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SiteData {
    pub title: String,
    pub description: String,
    /// Páginas na ordem da sidebar; depois as que não estão nela (home incluída), por rota.
    pub pages: Vec<PageData>,
    /// Grupos da sidebar, na ordem de exibição.
    pub groups: Vec<GroupData>,
}

impl SiteData {
    /// Página pela rota (`/components/btn`).
    pub fn page(&self, route: &str) -> Option<&PageData> {
        self.pages.iter().find(|p| p.route == route)
    }
}

/// Um grupo da sidebar.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GroupData {
    /// Pasta de primeiro nível (`components`).
    pub folder: String,
    /// Título exibido (`Componentes`).
    pub text: String,
    /// Rotas das páginas, na ordem da sidebar.
    pub routes: Vec<String>,
}

/// Uma página compilada, como os plugins a veem.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PageData {
    /// Rota (`/components/btn`; a home é `/`).
    pub route: String,
    /// Caminho do `.md` (relativo à raiz do site).
    pub file: PathBuf,
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
