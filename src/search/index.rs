//! Leitor do `llms-full.txt` e índice da busca (ADR-0015). Puro: sem DOM e sem Leptos,
//! testado no host.
//!
//! O texto é o do plugin `leptos-press-llms` (contrato no doc comment daquele crate): páginas
//! separadas por `---` / `url: …` / `---` no início de linha, cada uma com o Markdown da página.
//! Cada página vira **seções**: a do topo (o texto antes do primeiro `##`) e uma por `##`/`###`.
//! A âncora de cada seção vem dos cabeçalhos do [`PageMeta`] (os do outline), casados pelo
//! texto normalizado e pela ocorrência; título e grupo da sidebar vêm do [`SiteMeta`].

use std::collections::HashMap;

use leptos_nebula::search::normalize;

use crate::meta::{PageMeta, SiteMeta};

/// Índice montado uma vez, a partir do `llms-full.txt` e do [`SiteMeta`].
#[derive(Debug, Clone, Default)]
pub struct SearchIndex {
    /// Páginas, na ordem do arquivo (home primeiro, depois a ordem do site).
    pub pages: Vec<IndexedPage>,
    /// Seções de todas as páginas, na ordem do texto.
    pub sections: Vec<Section>,
}

/// Uma página do índice.
#[derive(Debug, Clone)]
pub struct IndexedPage {
    /// Rota (`/components/select`; a home é `/`).
    pub route: &'static str,
    /// Título do `PageMeta`.
    pub title: &'static str,
    /// Título do grupo da sidebar que contém a página, se houver.
    pub group: Option<&'static str>,
    pub(crate) norm_title: String,
}

/// Uma seção: o topo da página ou um `##`/`###`.
#[derive(Debug, Clone)]
pub struct Section {
    /// Índice em [`SearchIndex::pages`].
    pub page: usize,
    /// `id` do cabeçalho na página (sem `#`); `None` = topo da página (seção do topo, ou
    /// cabeçalho sem par no `PageMeta`).
    pub anchor: Option<&'static str>,
    /// Cabeçalhos até a seção, sem o título da página: `[]` no topo, `["Filtro"]` num `##`,
    /// `["Formatos de options", "Formato 3"]` num `###` (em claro, sem marcação).
    pub headings: Vec<String>,
    /// Texto corrido em claro (sem marcação), espaços colapsados. Inclui `####`… e tabelas.
    pub text: String,
    /// Conteúdo dos blocos de código cercados, espaços colapsados (pesa menos na busca).
    pub code: String,
    pub(crate) norm_headings: String,
    pub(crate) norm_text: String,
    pub(crate) norm_code: String,
}

impl SearchIndex {
    /// Monta o índice. Páginas cuja `url` não corresponde a uma rota do `site` são ignoradas
    /// (não haveria para onde levar o resultado).
    pub fn build(full: &str, site: &'static SiteMeta) -> Self {
        let mut index = SearchIndex::default();
        for raw in split_pages(full) {
            let route = url_to_route(raw.url);
            let Some(meta) = site.page(&route) else {
                continue;
            };
            let page = index.pages.len();
            let group = site
                .sidebar
                .iter()
                .find(|g| g.contains(meta.path))
                .map(|g| g.text);
            index.pages.push(IndexedPage {
                route: meta.path,
                title: meta.title,
                group,
                norm_title: normalize(meta.title),
            });
            index_page(&mut index.sections, page, raw.markdown, meta);
        }
        index
    }

    /// `rota#âncora` (ou só a rota) de uma seção.
    pub fn href(&self, section: usize) -> String {
        let s = &self.sections[section];
        let route = self.pages[s.page].route;
        match s.anchor {
            Some(id) => format!("{route}#{id}"),
            None => route.to_string(),
        }
    }
}

/// Uma página do `llms-full.txt`, ainda em Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPage<'a> {
    /// O `url:` do separador (`/components/btn.md`, ou a rota sem as cópias `.md`).
    pub url: &'a str,
    /// O Markdown da página (sem o separador).
    pub markdown: &'a str,
}

/// Divide o `llms-full.txt` em páginas pelo separador `---` / `url: …` / `---` (no início de
/// linha), **ignorando** blocos de código cercados: um separador dentro de um exemplo não
/// divide. Texto antes do primeiro separador é ignorado.
pub fn split_pages(full: &str) -> Vec<RawPage<'_>> {
    let lines = line_spans(full);
    let mut pages = Vec::new();
    // (url, início do Markdown)
    let mut current: Option<(&str, usize)> = None;
    let mut fence = Fence::default();
    let mut i = 0;
    while i < lines.len() {
        let (start, line) = lines[i];
        if !fence.is_open() && line == "---" && i + 2 < lines.len() {
            let url = lines[i + 1].1.strip_prefix("url: ");
            if let (Some(url), "---") = (url, lines[i + 2].1) {
                if let Some((u, from)) = current.take() {
                    pages.push(RawPage {
                        url: u,
                        markdown: full[from..start].trim(),
                    });
                }
                let after = lines.get(i + 3).map_or(full.len(), |(s, _)| *s);
                current = Some((url.trim(), after));
                i += 3;
                continue;
            }
        }
        fence.feed(line);
        i += 1;
    }
    if let Some((u, from)) = current {
        pages.push(RawPage {
            url: u,
            markdown: full[from..].trim(),
        });
    }
    pages
}

/// `url` do `llms-full.txt` → rota do site: tira o `.md`; `/index` → `/`; `/x/index` → `/x`.
/// Sem as cópias `.md` (`page_copies(false)`), a `url` já é a rota e passa como está (só a
/// barra final sai).
pub fn url_to_route(url: &str) -> String {
    let path = url.split(['#', '?']).next().unwrap_or("");
    let Some(stem) = path.strip_suffix(".md") else {
        return crate::meta::normalize_path(path).to_string();
    };
    let route = if stem == "/index" {
        ""
    } else {
        stem.strip_suffix("/index").unwrap_or(stem)
    };
    crate::meta::normalize_path(route).to_string()
}

/// Seção em construção.
#[derive(Default)]
struct Draft {
    headings: Vec<String>,
    /// Texto do cabeçalho do `PageMeta` casado (âncora).
    anchor: Option<&'static str>,
    text: Vec<String>,
    code: Vec<String>,
}

fn index_page(out: &mut Vec<Section>, page: usize, markdown: &str, meta: &PageMeta) {
    // Cabeçalhos do outline, normalizados, para casar por texto + ocorrência.
    let outline: Vec<(String, &'static str)> = meta
        .headings
        .iter()
        .map(|h| (normalize(h.text.trim()), h.id))
        .collect();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut anchor_for = |text: &str| -> Option<&'static str> {
        let norm = normalize(text);
        let n = seen.entry(norm.clone()).or_insert(0);
        let k = *n;
        *n += 1;
        outline
            .iter()
            .filter(|(t, _)| *t == norm)
            .nth(k)
            .map(|(_, id)| *id)
    };

    let mut draft = Draft::default();
    // Cabeçalho `##` atual (para o caminho dos `###`).
    let mut h2: Option<String> = None;
    let mut fence = Fence::default();
    for line in markdown.lines() {
        if fence.is_open() {
            if !fence.feed(line) {
                draft.code.push(line.to_string());
            }
            continue;
        }
        if fence.feed(line) {
            continue;
        }
        if let Some((level, text)) = heading(line) {
            let text = inline_plain(text).trim().to_string();
            match level {
                1 => continue,
                2 | 3 => {
                    flush(out, page, std::mem::take(&mut draft));
                    let headings = if level == 2 {
                        h2 = Some(text.clone());
                        vec![text.clone()]
                    } else {
                        h2.iter().cloned().chain([text.clone()]).collect()
                    };
                    draft.anchor = anchor_for(&text);
                    draft.headings = headings;
                }
                _ => draft.text.push(text),
            }
            continue;
        }
        if let Some(text) = prose_line(line) {
            draft.text.push(text);
        }
    }
    flush(out, page, draft);
}

fn flush(out: &mut Vec<Section>, page: usize, d: Draft) {
    let text = collapse(&d.text.join(" "));
    let code = collapse(&d.code.join(" "));
    // Um cabeçalho vazio no fim (sem texto) ainda é uma seção: leva ao lugar certo.
    out.push(Section {
        page,
        anchor: d.anchor,
        norm_headings: normalize(&d.headings.join("\n")),
        norm_text: normalize(&text),
        norm_code: normalize(&code),
        headings: d.headings,
        text,
        code,
    });
}

/// Bloco de código cercado (```` ``` ```` ou `~~~`), aberto ou não.
#[derive(Default)]
struct Fence {
    open: Option<(char, usize)>,
}

impl Fence {
    fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Processa uma linha; `true` se ela abriu ou fechou um bloco (a linha da cerca).
    fn feed(&mut self, line: &str) -> bool {
        let t = line.trim_start();
        let Some(c) = t.chars().next().filter(|c| *c == '`' || *c == '~') else {
            return false;
        };
        let n = t.chars().take_while(|x| *x == c).count();
        if n < 3 {
            return false;
        }
        match self.open {
            None => {
                // Cerca de crases não pode ter crase na info.
                if c == '`' && t[n..].contains('`') {
                    return false;
                }
                self.open = Some((c, n));
                true
            }
            Some((oc, on)) if oc == c && n >= on && t[n..].trim().is_empty() => {
                self.open = None;
                true
            }
            Some(_) => false,
        }
    }
}

/// (início em bytes, linha sem `\n`/`\r`) de cada linha.
fn line_spans(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        out.push((start, line.trim_end_matches(['\n', '\r'])));
        start += line.len();
    }
    out
}

/// `## Título ##` → (2, "Título").
fn heading(line: &str) -> Option<(usize, &str)> {
    let t = line.trim_start();
    if line.len() - t.len() > 3 {
        return None;
    }
    let level = t.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &t[level..];
    if !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
        return None;
    }
    let rest = rest.trim();
    // `#`s de fechamento opcionais (precedidos de espaço)
    let closed = rest.trim_end_matches('#');
    let rest = if closed.len() < rest.len() && (closed.is_empty() || closed.ends_with(' ')) {
        closed.trim_end()
    } else {
        rest
    };
    Some((level, rest))
}

/// Uma linha de texto corrido em claro; `None` se a linha não tem texto (linha de tabela
/// `|---|`, regra `---`, cerca de contêiner `:::`).
fn prose_line(line: &str) -> Option<String> {
    let mut t = line.trim();
    if t.is_empty() {
        return None;
    }
    // `::: info Título` → "Título"; `:::` sozinho → nada.
    if let Some(rest) = t.strip_prefix(":::") {
        let mut words = rest.split_whitespace();
        words.next();
        let title = words.collect::<Vec<_>>().join(" ");
        return (!title.is_empty()).then(|| inline_plain(&title));
    }
    // Separador de tabela e regra horizontal.
    if t.chars()
        .all(|c| matches!(c, '|' | '-' | ':' | ' ' | '*' | '_' | '=' | '\t'))
        && t.contains(['-', '*', '_', '='])
    {
        return None;
    }
    // Citação e alertas (`> [!NOTE]`).
    while let Some(rest) = t.strip_prefix('>') {
        t = rest.trim_start();
    }
    if t.starts_with("[!") && t.ends_with(']') && !t.contains(' ') {
        return None;
    }
    // Marcadores de lista.
    t = strip_list_marker(t);
    let plain = inline_plain(t);
    (!plain.trim().is_empty()).then_some(plain)
}

fn strip_list_marker(t: &str) -> &str {
    for m in ["- ", "* ", "+ "] {
        if let Some(rest) = t.strip_prefix(m) {
            return strip_task(rest.trim_start());
        }
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if (1..=9).contains(&digits) {
        let rest = &t[digits..];
        if let Some(r) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return r.trim_start();
        }
    }
    t
}

fn strip_task(t: &str) -> &str {
    ["[ ] ", "[x] ", "[X] "]
        .iter()
        .find_map(|m| t.strip_prefix(m))
        .unwrap_or(t)
}

/// Marcação de linha → texto em claro: `[t](u)` → `t`, `![a](u)` → `a`, ênfase sem os
/// marcadores, código entre crases sem as crases (conteúdo intacto, inclusive `_`), escapes `\x`,
/// `|` de tabela vira espaço, entidades comuns decodificadas.
pub(crate) fn inline_plain(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if i + 1 < chars.len() && chars[i + 1].is_ascii_punctuation() => {
                out.push(chars[i + 1]);
                i += 2;
            }
            '`' => {
                let n = run(&chars, i, '`');
                match find_run(&chars, i + n, '`', n) {
                    Some(end) => {
                        let code: String = chars[i + n..end].iter().collect();
                        let code = code.replace("\\|", "|");
                        let trimmed = if code.len() > 2
                            && code.starts_with(' ')
                            && code.ends_with(' ')
                            && !code.trim().is_empty()
                        {
                            &code[1..code.len() - 1]
                        } else {
                            &code
                        };
                        out.push_str(trimmed);
                        i = end + n;
                    }
                    None => {
                        out.extend(&chars[i..i + n]);
                        i += n;
                    }
                }
            }
            '!' if chars.get(i + 1) == Some(&'[') => match link(&chars, i + 1) {
                Some((label, next)) => {
                    out.push_str(&inline_plain(&label));
                    i = next;
                }
                None => {
                    out.push('!');
                    i += 1;
                }
            },
            '[' => match link(&chars, i) {
                Some((label, next)) => {
                    out.push_str(&inline_plain(&label));
                    i = next;
                }
                None => {
                    out.push('[');
                    i += 1;
                }
            },
            '*' | '~' => i += run(&chars, i, c),
            '_' => {
                let n = run(&chars, i, '_');
                let before = i.checked_sub(1).map(|j| chars[j]);
                let after = chars.get(i + n).copied();
                let intraword = before.is_some_and(char::is_alphanumeric)
                    && after.is_some_and(char::is_alphanumeric);
                if intraword {
                    out.extend(&chars[i..i + n]);
                }
                i += n;
            }
            '|' => {
                out.push(' ');
                i += 1;
            }
            '&' => match entity(&chars, i) {
                Some((s, next)) => {
                    out.push(s);
                    i = next;
                }
                None => {
                    out.push('&');
                    i += 1;
                }
            },
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

fn run(chars: &[char], from: usize, c: char) -> usize {
    chars[from..].iter().take_while(|x| **x == c).count()
}

/// Próxima sequência de exatamente `n` caracteres `c` a partir de `from`.
fn find_run(chars: &[char], from: usize, c: char, n: usize) -> Option<usize> {
    let mut j = from;
    while j < chars.len() {
        if chars[j] == c {
            let m = run(chars, j, c);
            if m == n {
                return Some(j);
            }
            j += m;
        } else {
            j += 1;
        }
    }
    None
}

/// `[rótulo](destino)` ou `[rótulo][ref]` a partir de `chars[at] == '['` → (rótulo, próximo).
fn link(chars: &[char], at: usize) -> Option<(String, usize)> {
    let mut depth = 0usize;
    let mut j = at;
    let close = loop {
        match chars.get(j)? {
            '\\' => j += 1,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    break j;
                }
            }
            _ => {}
        }
        j += 1;
    };
    let label: String = chars[at + 1..close].iter().collect();
    let (open, end) = match chars.get(close + 1) {
        Some('(') => ('(', ')'),
        Some('[') => ('[', ']'),
        _ => return None,
    };
    let mut k = close + 2;
    let mut depth = 1usize;
    while k < chars.len() {
        let x = chars[k];
        if x == open && open == '(' {
            depth += 1;
        } else if x == end {
            depth -= 1;
            if depth == 0 {
                return Some((label, k + 1));
            }
        }
        k += 1;
    }
    None
}

fn entity(chars: &[char], at: usize) -> Option<(char, usize)> {
    const NAMED: [(&str, char); 6] = [
        ("&amp;", '&'),
        ("&lt;", '<'),
        ("&gt;", '>'),
        ("&quot;", '"'),
        ("&#39;", '\''),
        ("&nbsp;", ' '),
    ];
    let rest: String = chars[at..chars.len().min(at + 6)].iter().collect();
    NAMED
        .iter()
        .find(|(name, _)| rest.starts_with(name))
        .map(|(name, c)| (*c, at + name.chars().count()))
}

/// Espaços em sequência (inclusive quebras) viram um; sem espaço nas pontas.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::meta::{Heading, PageLayout, SidebarGroup, SidebarItem};

    const fn h(level: u8, id: &'static str, text: &'static str) -> Heading {
        Heading { level, id, text }
    }

    static SELECT_HEADINGS: [Heading; 6] = [
        h(2, "formatos-de-options", "Formatos de options"),
        h(
            3,
            "formato-3-com-key_as_value",
            "Formato 3 com key_as_value",
        ),
        h(2, "filtro", "Filtro"),
        h(2, "exemplo", "Exemplo"),
        h(2, "exemplo-1", "Exemplo"),
        h(2, "api", "API"),
    ];
    static GUIDE_HEADINGS: [Heading; 1] = [h(2, "instalação", "Instalação")];
    static PAGES: [PageMeta; 4] = [
        PageMeta {
            path: "/",
            title: "Início",
            description: "",
            layout: PageLayout::Home,
            headings: &[],
            home: None,
        },
        PageMeta {
            path: "/guide",
            title: "Guia",
            description: "",
            layout: PageLayout::Doc,
            headings: &[],
            home: None,
        },
        PageMeta {
            path: "/guide/start",
            title: "Começando",
            description: "",
            layout: PageLayout::Doc,
            headings: &GUIDE_HEADINGS,
            home: None,
        },
        PageMeta {
            path: "/components/select",
            title: "Select",
            description: "",
            layout: PageLayout::Doc,
            headings: &SELECT_HEADINGS,
            home: None,
        },
    ];
    static ITEMS: [SidebarItem; 1] = [SidebarItem {
        text: "Select",
        link: "/components/select",
    }];
    static GROUPS: [SidebarGroup; 1] = [SidebarGroup {
        text: "Componentes",
        collapsed: None,
        items: &ITEMS,
    }];
    pub(crate) static SITE: SiteMeta = SiteMeta {
        title: "t",
        description: "",
        nav: &[],
        sidebar: &GROUPS,
        pages: &PAGES,
        search: Some("/llms-full.txt"),
    };

    pub(crate) const FULL: &str = "---\nurl: /index.md\n---\n\n# Início\n\n---\nurl: /guide/index.md\n---\n\n# Guia\n\nO guia.\n\n---\nurl: /guide/start.md\n---\n\n# Começando\n\nTexto do **topo**, com [um link](/x) e `código_inline`.\n\n## Instalação\n\nRode `cargo add`.\n\n---\nurl: /components/select.md\n---\n\n# Select\n\nEscolha uma opção. São Paulo fica no topo.\n\n```text\n---\nurl: /falso.md\n---\n## Não é cabeçalho\n```\n\n## Formatos de options\n\n| Formato | Modelo |\n|---|---|\n| `Vec<P>` | `P` |\n\n### Formato 3 com `key_as_value`\n\nO modelo é a key.\n\n```rust\nlet d = ObjectOptions::new(users).key_as_value();\n```\n\n## Filtro\n\n> [!NOTE]\n> Busca _sem_ acentos.\n\n## Exemplo\n\nPrimeiro.\n\n### Slots\n\nVeio de uma tag.\n\n## Exemplo\n\nSegundo.\n\n## Extra de tag\n\nSem par.\n\n## API\n\n- `use_filter`: liga o filtro.\n";

    #[test]
    fn separator_inside_a_code_block_does_not_split() {
        let pages = split_pages(FULL);
        let urls: Vec<&str> = pages.iter().map(|p| p.url).collect();
        assert_eq!(
            urls,
            [
                "/index.md",
                "/guide/index.md",
                "/guide/start.md",
                "/components/select.md"
            ]
        );
        assert!(pages[3].markdown.contains("url: /falso.md"));
        assert!(pages[3].markdown.starts_with("# Select"));
        assert!(pages[3].markdown.ends_with("liga o filtro."));
        assert_eq!(pages[0].markdown, "# Início");
    }

    #[test]
    fn text_before_the_first_separator_is_ignored() {
        let pages = split_pages("lixo\n---\nurl: /a.md\n---\n\n# A\n");
        assert_eq!(
            pages,
            [RawPage {
                url: "/a.md",
                markdown: "# A"
            }]
        );
        assert!(split_pages("").is_empty());
    }

    #[test]
    fn tilde_fences_and_longer_fences_hide_separators() {
        let full = "---\nurl: /a.md\n---\n\n~~~\n---\nurl: /b.md\n---\n~~~\n\n````md\n```\n---\nurl: /c.md\n---\n```\n````\n\n---\nurl: /d.md\n---\n\nD\n";
        let urls: Vec<&str> = split_pages(full).iter().map(|p| p.url).collect();
        assert_eq!(urls, ["/a.md", "/d.md"]);
    }

    #[test]
    fn url_maps_to_route() {
        assert_eq!(url_to_route("/index.md"), "/");
        assert_eq!(url_to_route("/guide/index.md"), "/guide");
        assert_eq!(url_to_route("/components/btn.md"), "/components/btn");
        // `page_copies(false)`: a url já é a rota
        assert_eq!(url_to_route("/"), "/");
        assert_eq!(url_to_route("/guide"), "/guide");
        assert_eq!(url_to_route("/guide/"), "/guide");
        assert_eq!(url_to_route("/components/btn"), "/components/btn");
    }

    #[test]
    fn sections_paths_and_anchors() {
        let index = SearchIndex::build(FULL, &SITE);
        let routes: Vec<&str> = index.pages.iter().map(|p| p.route).collect();
        assert_eq!(
            routes,
            ["/", "/guide", "/guide/start", "/components/select"]
        );
        assert_eq!(index.pages[3].group, Some("Componentes"));
        assert_eq!(index.pages[2].group, None);

        let select: Vec<&Section> = index.sections.iter().filter(|s| s.page == 3).collect();
        let shape: Vec<(Vec<&str>, Option<&str>)> = select
            .iter()
            .map(|s| (s.headings.iter().map(String::as_str).collect(), s.anchor))
            .collect();
        assert_eq!(
            shape,
            [
                (vec![], None),
                (vec!["Formatos de options"], Some("formatos-de-options")),
                (
                    vec!["Formatos de options", "Formato 3 com key_as_value"],
                    Some("formato-3-com-key_as_value")
                ),
                (vec!["Filtro"], Some("filtro")),
                (vec!["Exemplo"], Some("exemplo")),
                // cabeçalho extra vindo de uma tag: sem par, sem âncora
                (vec!["Exemplo", "Slots"], None),
                // cabeçalho repetido: a 2ª ocorrência casa com o 2º do outline
                (vec!["Exemplo"], Some("exemplo-1")),
                // sem par no outline
                (vec!["Extra de tag"], None),
                (vec!["API"], Some("api")),
            ]
        );
        assert_eq!(
            index.href(
                index
                    .sections
                    .iter()
                    .position(|s| s.anchor == Some("filtro"))
                    .unwrap()
            ),
            "/components/select#filtro"
        );
        // a home não tem cabeçalhos: só a seção do topo
        assert_eq!(index.href(0), "/");
    }

    #[test]
    fn anchors_match_accented_heading_text() {
        let index = SearchIndex::build(FULL, &SITE);
        let s = index
            .sections
            .iter()
            .find(|s| s.headings == ["Instalação"])
            .unwrap();
        assert_eq!(s.anchor, Some("instalação"));
        assert_eq!(s.text, "Rode cargo add.");
    }

    #[test]
    fn plain_text_and_code_are_separated() {
        let index = SearchIndex::build(FULL, &SITE);
        let top = &index.sections[2];
        assert_eq!(top.headings, Vec::<String>::new());
        assert_eq!(top.text, "Texto do topo, com um link e código_inline.");
        let select_top = index.sections.iter().find(|s| s.page == 3).unwrap();
        assert_eq!(
            select_top.text,
            "Escolha uma opção. São Paulo fica no topo."
        );
        assert_eq!(select_top.code, "--- url: /falso.md --- ## Não é cabeçalho");
        let formats = index
            .sections
            .iter()
            .find(|s| s.anchor == Some("formatos-de-options"))
            .unwrap();
        assert_eq!(formats.text, "Formato Modelo Vec<P> P");
        let kav = index
            .sections
            .iter()
            .find(|s| s.anchor == Some("formato-3-com-key_as_value"))
            .unwrap();
        assert_eq!(
            kav.code,
            "let d = ObjectOptions::new(users).key_as_value();"
        );
        let note = index
            .sections
            .iter()
            .find(|s| s.anchor == Some("filtro"))
            .unwrap();
        assert_eq!(note.text, "Busca sem acentos.");
        let api = index.sections.last().unwrap();
        assert_eq!(api.text, "use_filter: liga o filtro.");
        assert_eq!(api.norm_text, "use_filter: liga o filtro.");
        assert_eq!(
            select_top.norm_text,
            "escolha uma opcao. sao paulo fica no topo."
        );
    }

    #[test]
    fn pages_missing_from_the_site_are_skipped() {
        let index = SearchIndex::build("---\nurl: /nao-existe.md\n---\n\n# X\n", &SITE);
        assert!(index.pages.is_empty() && index.sections.is_empty());
    }

    #[test]
    fn inline_markup_becomes_plain_text() {
        let cases = [
            ("[texto](/a) e ![alt](/i.png)", "texto e alt"),
            (
                "**negrito**, *itálico*, __forte__, _ênfase_",
                "negrito, itálico, forte, ênfase",
            ),
            (
                "`key_as_value` e key_as_value",
                "key_as_value e key_as_value",
            ),
            ("`` x ` y ``", "x ` y"),
            ("a \\* z \\_c\\_", "a * z _c_"),
            ("| a | `x \\| y` |", "  a   x | y  "),
            ("&lt;Btn&gt; &amp; ~~velho~~", "<Btn> & velho"),
            ("[ref][1] e [solto", "ref e [solto"),
            ("[`código` no link](/x)", "código no link"),
        ];
        for (md, plain) in cases {
            assert_eq!(inline_plain(md), plain, "{md:?}");
        }
    }

    #[test]
    fn headings_are_parsed_like_atx() {
        assert_eq!(heading("## Título"), Some((2, "Título")));
        assert_eq!(heading("### Título ###"), Some((3, "Título")));
        assert_eq!(heading("#### C# ##"), Some((4, "C#")));
        assert_eq!(heading("#sem espaço"), None);
        assert_eq!(heading("####### sete"), None);
        assert_eq!(heading("    ## recuado demais"), None);
    }

    #[test]
    fn page_without_outline_still_has_its_sections() {
        // página sem outline: tudo sem âncora, mas as seções existem
        static P: [PageMeta; 1] = [PageMeta {
            path: "/a",
            title: "A",
            description: "",
            layout: PageLayout::Doc,
            headings: &[],
            home: None,
        }];
        static S: SiteMeta = SiteMeta {
            title: "",
            description: "",
            nav: &[],
            sidebar: &[],
            pages: &P,
            search: None,
        };
        let index = SearchIndex::build("---\nurl: /a\n---\n\n# A\n\n## B\n\nb\n", &S);
        assert_eq!(index.sections.len(), 2);
        assert_eq!(index.sections[1].anchor, None);
        assert_eq!(index.href(1), "/a");
    }
}
