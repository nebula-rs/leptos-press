//! AST do comrak → blocos de código Leptos + HTML estático + texto para LLM.
//!
//! O que é texto corrido (parágrafos, listas, cabeçalhos…) vira **HTML estático** gerado no
//! build e entregue ao runtime por `leptos_press::html(…)` (`inner_html`); só o que precisa de
//! componente (bloco de código, tabela, alerta, tags registradas) vira chamada própria.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use comrak::nodes::{AlertType, AstNode, ListType, NodeValue, TableAlignment};
use proc_macro2::TokenStream;
use quote::quote;

use crate::component::{Attrs, Ctx, MdComponent};
use crate::error::Error;
use crate::frontmatter::FrontMatter;
use crate::highlight::Highlighter;
use crate::html::{escape, push_escaped, slugify};

/// Cabeçalho encontrado na página.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub id: String,
    pub text: String,
}

/// Link interno (`/…`) a validar depois que todas as rotas são conhecidas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalLink {
    pub url: String,
    pub line: usize,
}

struct Replacement {
    start: usize,
    end: usize,
    text: String,
}

pub(crate) struct Renderer<'a> {
    pub file: &'a Path,
    pub route: &'a str,
    pub root: &'a Path,
    pub front: &'a FrontMatter,
    pub components: &'a [Box<dyn MdComponent>],
    pub highlighter: &'a Highlighter,
    pub reads: &'a RefCell<Vec<PathBuf>>,
    pub headings: RefCell<Vec<Heading>>,
    slugs: RefCell<HashMap<String, usize>>,
    pub items: RefCell<Vec<TokenStream>>,
    replacements: RefCell<Vec<Replacement>>,
    pub links: RefCell<Vec<InternalLink>>,
}

type Node<'a> = &'a AstNode<'a>;

fn line_of(node: Node) -> usize {
    node.data().sourcepos.start.line
}

impl<'a> Renderer<'a> {
    pub(crate) fn new(
        file: &'a Path,
        route: &'a str,
        root: &'a Path,
        front: &'a FrontMatter,
        components: &'a [Box<dyn MdComponent>],
        highlighter: &'a Highlighter,
        reads: &'a RefCell<Vec<PathBuf>>,
    ) -> Self {
        Self {
            file,
            route,
            root,
            front,
            components,
            highlighter,
            reads,
            headings: RefCell::default(),
            slugs: RefCell::default(),
            items: RefCell::default(),
            replacements: RefCell::default(),
            links: RefCell::default(),
        }
    }

    fn err(&self, line: usize, message: impl Into<String>) -> Error {
        Error::new(message).at(self.file, line)
    }

    // ── Blocos de topo (e conteúdo de alertas) ───────────────────────────────

    /// Um item por bloco que precisa de componente; o texto corrido entre eles é agrupado em
    /// uma única chamada `html(…)`.
    pub(crate) fn blocks(&self, parent: Node) -> Result<Vec<TokenStream>, Error> {
        let mut out = Vec::new();
        let mut html = String::new();
        for child in parent.children() {
            let line = line_of(child);
            let value = child.data().value.clone();
            match value {
                NodeValue::CodeBlock(cb) => {
                    flush(&mut html, &mut out);
                    let lang = lang_of(&cb.info);
                    let code = cb.literal.trim_end_matches('\n');
                    let highlighted = self.highlighter.highlight(lang, code);
                    out.push(quote! { ::leptos_press::code_block(#lang, #code, #highlighted) });
                }
                NodeValue::Table(_) => {
                    flush(&mut html, &mut out);
                    let inner = self.table_inner(child)?;
                    out.push(quote! { ::leptos_press::table(#inner) });
                }
                NodeValue::Alert(alert) => {
                    flush(&mut html, &mut out);
                    let kind = alert_kind(alert.alert_type);
                    let title = alert
                        .title
                        .clone()
                        .unwrap_or_else(|| alert.alert_type.default_title().to_string());
                    out.push(self.alert(child, &kind, &title)?);
                }
                NodeValue::BlockDirective(dir) => {
                    flush(&mut html, &mut out);
                    let info = dir.info.trim();
                    let (word, rest) = info.split_once(char::is_whitespace).unwrap_or((info, ""));
                    let (kind, default_title) = match word.to_ascii_lowercase().as_str() {
                        "tip" => ("Tip", "Tip"),
                        "warning" => ("Warning", "Warning"),
                        "danger" => ("Caution", "Danger"),
                        "info" => ("Note", "Info"),
                        "note" => ("Note", "Note"),
                        "important" => ("Important", "Important"),
                        "caution" => ("Caution", "Caution"),
                        other => {
                            return Err(self.err(
                                line,
                                format!(
                                    "container `::: {other}` desconhecido (use tip, warning, danger, info, note, important ou caution)"
                                ),
                            ));
                        }
                    };
                    let title = if rest.trim().is_empty() {
                        default_title
                    } else {
                        rest.trim()
                    };
                    let kind = syn::Ident::new(kind, proc_macro2::Span::call_site());
                    out.push(self.alert(child, &quote! { #kind }, title)?);
                }
                NodeValue::HtmlBlock(block) => {
                    if let Some(tokens) = self.tag_block(child, &block.literal, line)? {
                        flush(&mut html, &mut out);
                        out.push(tokens);
                    }
                }
                _ => self.write_block(child, &mut html)?,
            }
        }
        flush(&mut html, &mut out);
        Ok(out)
    }

    fn alert(&self, node: Node, kind: &TokenStream, title: &str) -> Result<TokenStream, Error> {
        let inner = self.blocks(node)?;
        Ok(quote! {
            ::leptos_press::alert(
                ::leptos_press::AlertKind::#kind,
                #title,
                ::std::vec![ #( #inner ),* ],
            )
        })
    }

    // ── Tags registradas ─────────────────────────────────────────────────────

    fn tag_block(
        &self,
        node: Node,
        literal: &str,
        line: usize,
    ) -> Result<Option<TokenStream>, Error> {
        let text = literal.trim();
        if text.starts_with("<!--") {
            return if text.ends_with("-->") {
                Ok(None)
            } else {
                Err(self.err(line, "comentário HTML não fechado"))
            };
        }
        let name = tag_name(text);
        if name.is_empty() || name.chars().next().is_some_and(char::is_lowercase) {
            return Err(self.err(
                line,
                format!(
                    "HTML cru não é permitido (`{}`); use Markdown ou uma tag registrada",
                    first_line(text)
                ),
            ));
        }
        let Some(component) = self.components.iter().find(|c| c.tag() == name) else {
            return Err(self.err(line, format!("tag `<{name}>` não registrada")));
        };
        let (_, attrs) = parse_tag(text).map_err(|m| self.err(line, m))?;
        let attrs = Attrs::new(attrs);
        let ctx = Ctx {
            page_file: self.file,
            route: self.route,
            front: self.front,
            root: self.root,
            reads: self.reads,
        };
        let tokens = component
            .codegen(&attrs, &ctx)
            .map_err(|e| e.at(self.file, line))?;
        let items = component
            .items(&attrs, &ctx)
            .map_err(|e| e.at(self.file, line))?;
        let llms = component
            .llms(&attrs, &ctx)
            .map_err(|e| e.at(self.file, line))?;
        if let Some(extra) = attrs.unused().first() {
            return Err(self.err(
                line,
                format!("atributo desconhecido `{extra}` em `<{name}>`"),
            ));
        }
        if !items.is_empty() {
            self.items.borrow_mut().push(items);
        }
        self.replacements.borrow_mut().push(Replacement {
            start: line,
            end: node.data().sourcepos.end.line,
            text: llms,
        });
        Ok(Some(
            quote! { ::leptos::prelude::IntoAny::into_any(#tokens) },
        ))
    }

    /// Texto da página para LLM: o Markdown original (sem front-matter) com as tags
    /// substituídas pela saída `llms` de cada uma.
    pub(crate) fn llms_markdown(&self, src: &str, skip_lines: usize) -> String {
        let lines: Vec<&str> = src.split('\n').collect();
        let replacements = self.replacements.borrow();
        let mut out = String::new();
        let mut i = skip_lines;
        while i < lines.len() {
            let number = i + 1;
            if let Some(r) = replacements.iter().find(|r| r.start == number) {
                out.push_str(r.text.trim_end());
                out.push('\n');
                i = r.end;
            } else {
                out.push_str(lines[i]);
                out.push('\n');
                i += 1;
            }
        }
        let trimmed = out.trim();
        format!("{trimmed}\n")
    }

    // ── HTML estático ────────────────────────────────────────────────────────

    fn write_block(&self, node: Node, out: &mut String) -> Result<(), Error> {
        let line = line_of(node);
        let value = node.data().value.clone();
        match value {
            NodeValue::Paragraph => {
                out.push_str("<p>");
                self.write_children(node, out)?;
                out.push_str("</p>\n");
            }
            NodeValue::Heading(h) => {
                let text = plain_text(node);
                let id = self.unique_slug(&text);
                self.headings.borrow_mut().push(Heading {
                    level: h.level,
                    id: id.clone(),
                    text,
                });
                out.push_str(&format!("<h{} id=\"{id}\">", h.level));
                self.write_children(node, out)?;
                out.push_str(&format!(
                    "<a class=\"header-anchor\" href=\"#{id}\" aria-label=\"Permalink\">#</a></h{}>\n",
                    h.level
                ));
            }
            NodeValue::ThematicBreak => out.push_str("<hr>\n"),
            NodeValue::BlockQuote => {
                out.push_str("<blockquote>\n");
                self.write_children(node, out)?;
                out.push_str("</blockquote>\n");
            }
            NodeValue::List(list) => {
                let ordered = list.list_type == ListType::Ordered;
                if ordered {
                    if list.start == 1 {
                        out.push_str("<ol>\n");
                    } else {
                        out.push_str(&format!("<ol start=\"{}\">\n", list.start));
                    }
                } else {
                    out.push_str("<ul>\n");
                }
                for item in node.children() {
                    self.write_item(item, list.tight, out)?;
                }
                out.push_str(if ordered { "</ol>\n" } else { "</ul>\n" });
            }
            NodeValue::CodeBlock(cb) => {
                let lang = lang_of(&cb.info);
                let code = cb.literal.trim_end_matches('\n');
                let highlighted = self.highlighter.highlight(lang, code);
                out.push_str(&format!(
                    "<pre><code class=\"language-{}\">{highlighted}</code></pre>\n",
                    escape(lang)
                ));
            }
            NodeValue::Table(_) => {
                out.push_str("<table>");
                out.push_str(&self.table_inner(node)?);
                out.push_str("</table>\n");
            }
            NodeValue::HtmlBlock(block) => {
                // Dentro de lista/citação só comentários são aceitos.
                if !block.literal.trim().starts_with("<!--") {
                    return Err(self.err(
                        line,
                        "tags e HTML só são aceitos no nível de bloco da página ou dentro de alertas",
                    ));
                }
            }
            other => {
                return Err(self.err(
                    line,
                    format!("elemento Markdown não suportado: {}", other.xml_node_name()),
                ));
            }
        }
        Ok(())
    }

    fn write_item(&self, item: Node, tight: bool, out: &mut String) -> Result<(), Error> {
        let value = item.data().value.clone();
        out.push_str("<li>");
        if let NodeValue::TaskItem(task) = value {
            let checked = if task.symbol.is_some() {
                " checked"
            } else {
                ""
            };
            out.push_str(&format!("<input type=\"checkbox\" disabled{checked}> "));
        }
        for child in item.children() {
            if tight && matches!(child.data().value, NodeValue::Paragraph) {
                self.write_children(child, out)?;
            } else {
                self.write_block(child, out)?;
            }
        }
        out.push_str("</li>\n");
        Ok(())
    }

    fn table_inner(&self, table: Node) -> Result<String, Error> {
        let alignments = match &table.data().value {
            NodeValue::Table(t) => t.alignments.clone(),
            _ => Vec::new(),
        };
        let mut head = String::new();
        let mut body = String::new();
        for row in table.children() {
            let is_header = matches!(row.data().value, NodeValue::TableRow(true));
            let target = if is_header { &mut head } else { &mut body };
            target.push_str("<tr>");
            for (i, cell) in row.children().enumerate() {
                let tag = if is_header { "th" } else { "td" };
                let style = match alignments.get(i) {
                    Some(TableAlignment::Left) => " style=\"text-align:left\"",
                    Some(TableAlignment::Center) => " style=\"text-align:center\"",
                    Some(TableAlignment::Right) => " style=\"text-align:right\"",
                    _ => "",
                };
                target.push_str(&format!("<{tag}{style}>"));
                self.write_children(cell, target)?;
                target.push_str(&format!("</{tag}>"));
            }
            target.push_str("</tr>\n");
        }
        let mut out = String::new();
        if !head.is_empty() {
            out.push_str(&format!("<thead>\n{head}</thead>\n"));
        }
        if !body.is_empty() {
            out.push_str(&format!("<tbody>\n{body}</tbody>\n"));
        }
        Ok(out)
    }

    fn write_children(&self, node: Node, out: &mut String) -> Result<(), Error> {
        for child in node.children() {
            self.write_node(child, out)?;
        }
        Ok(())
    }

    /// Inlines (e blocos, quando um bloco aparece dentro de um item de lista, etc.).
    fn write_node(&self, node: Node, out: &mut String) -> Result<(), Error> {
        let line = line_of(node);
        let value = node.data().value.clone();
        match value {
            NodeValue::Text(t) => push_escaped(out, &t),
            NodeValue::SoftBreak => out.push('\n'),
            NodeValue::LineBreak => out.push_str("<br>\n"),
            NodeValue::Code(c) => {
                out.push_str("<code>");
                push_escaped(out, &c.literal);
                out.push_str("</code>");
            }
            NodeValue::Emph => self.wrap("em", node, out)?,
            NodeValue::Strong => self.wrap("strong", node, out)?,
            NodeValue::Strikethrough => self.wrap("del", node, out)?,
            NodeValue::Link(link) => {
                let href = self.resolve_link(&link.url, line)?;
                out.push_str(&format!("<a href=\"{}\"", escape(&href)));
                if is_external(&href) {
                    out.push_str(" target=\"_blank\" rel=\"noopener noreferrer\"");
                }
                if !link.title.is_empty() {
                    out.push_str(&format!(" title=\"{}\"", escape(&link.title)));
                }
                out.push('>');
                self.write_children(node, out)?;
                out.push_str("</a>");
            }
            NodeValue::Image(img) => {
                out.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\" loading=\"lazy\">",
                    escape(&img.url),
                    escape(&plain_text(node))
                ));
            }
            NodeValue::HtmlInline(html) => {
                let name = tag_name(html.trim());
                let message = if name.chars().next().is_some_and(char::is_uppercase) {
                    format!(
                        "a tag `<{name}>` precisa ficar sozinha no seu bloco (linha em branco antes e depois) e ser autofechada (`<{name} … />`)"
                    )
                } else {
                    format!("HTML cru não é permitido (`{}`)", html.trim())
                };
                return Err(self.err(line, message));
            }
            NodeValue::Paragraph
            | NodeValue::Heading(_)
            | NodeValue::List(_)
            | NodeValue::BlockQuote
            | NodeValue::CodeBlock(_)
            | NodeValue::Table(_)
            | NodeValue::ThematicBreak
            | NodeValue::HtmlBlock(_) => self.write_block(node, out)?,
            other => {
                return Err(self.err(
                    line,
                    format!("elemento Markdown não suportado: {}", other.xml_node_name()),
                ));
            }
        }
        Ok(())
    }

    fn wrap(&self, tag: &str, node: Node, out: &mut String) -> Result<(), Error> {
        out.push_str(&format!("<{tag}>"));
        self.write_children(node, out)?;
        out.push_str(&format!("</{tag}>"));
        Ok(())
    }

    /// Links `./outra.md` viram a rota da página; `/…` é registrado para validação.
    fn resolve_link(&self, url: &str, line: usize) -> Result<String, Error> {
        if is_external(url) || url.starts_with('#') {
            return Ok(url.to_string());
        }
        if url.starts_with('/') {
            self.links.borrow_mut().push(InternalLink {
                url: url.to_string(),
                line,
            });
            return Ok(url.to_string());
        }
        let (path, fragment) = url.split_once('#').map_or((url, ""), |(p, f)| (p, f));
        if let Some(stem) = path.strip_suffix(".md") {
            let base = Path::new(self.route.trim_start_matches('/'));
            let base = if self.file.file_stem().is_some_and(|s| s == "index") {
                base
            } else {
                base.parent().unwrap_or(Path::new(""))
            };
            let joined = normalize(&base.join(stem));
            let mut route = format!("/{joined}");
            if route.ends_with("/index") {
                route.truncate(route.len() - "/index".len());
            }
            if route.is_empty() {
                route.push('/');
            }
            self.links.borrow_mut().push(InternalLink {
                url: route.clone(),
                line,
            });
            return Ok(if fragment.is_empty() {
                route
            } else {
                format!("{route}#{fragment}")
            });
        }
        Ok(url.to_string())
    }

    fn unique_slug(&self, text: &str) -> String {
        let base = slugify(text);
        let mut slugs = self.slugs.borrow_mut();
        let count = slugs.entry(base.clone()).or_insert(0);
        let id = if *count == 0 {
            base
        } else {
            format!("{base}-{count}")
        };
        *count += 1;
        id
    }
}

fn flush(html: &mut String, out: &mut Vec<TokenStream>) {
    if !html.is_empty() {
        let text = std::mem::take(html);
        out.push(quote! { ::leptos_press::html(#text) });
    }
}

fn lang_of(info: &str) -> &str {
    info.split([' ', ',', '{', '\t']).next().unwrap_or("")
}

fn alert_kind(kind: AlertType) -> TokenStream {
    let name = match kind {
        AlertType::Note => "Note",
        AlertType::Tip => "Tip",
        AlertType::Important => "Important",
        AlertType::Warning => "Warning",
        AlertType::Caution => "Caution",
    };
    let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
    quote! { #ident }
}

fn is_external(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:")
}

/// Resolve `.` e `..` em um caminho relativo (separador `/`).
fn normalize(path: &Path) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for c in path.components() {
        match c {
            std::path::Component::Normal(p) => parts.push(p.to_str().unwrap_or("")),
            std::path::Component::ParentDir => {
                parts.pop();
            }
            _ => {}
        }
    }
    parts.join("/")
}

/// Texto puro de um nó (sem marcação), para ids e outline.
pub(crate) fn plain_text(node: Node) -> String {
    let mut out = String::new();
    for d in node.descendants() {
        match &d.data().value {
            NodeValue::Text(t) => out.push_str(t),
            NodeValue::Code(c) => out.push_str(&c.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => out.push(' '),
            _ => {}
        }
    }
    out
}

/// Nome de uma tag (`<Tag …` → `Tag`); vazio se o texto não começa uma tag.
fn tag_name(text: &str) -> &str {
    let rest = text.strip_prefix('<').unwrap_or("");
    let len = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
        .unwrap_or(rest.len());
    &rest[..len]
}

/// `<Tag a="b" c='d' />` → (nome, atributos). Só a forma autofechada: é a única que o
/// CommonMark reconhece como bloco HTML sozinho.
fn parse_tag(text: &str) -> Result<(String, BTreeMap<String, String>), String> {
    let rest = text.strip_prefix('<').ok_or("bloco HTML inesperado")?;
    let name_len = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
        .unwrap_or(rest.len());
    let name = &rest[..name_len];
    if name.is_empty() {
        return Err(format!("HTML cru não é permitido (`{}`)", first_line(text)));
    }
    let mut rest = &rest[name_len..];
    let mut attrs = BTreeMap::new();
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix("/>") {
            return if after.trim().is_empty() {
                Ok((name.to_string(), attrs))
            } else {
                Err("mais de um elemento HTML no mesmo bloco; separe cada tag com uma linha em branco".into())
            };
        }
        if rest.starts_with('>') {
            return Err(format!(
                "`<{name}>` precisa ser autofechada (`<{name} … />`)"
            ));
        }
        let key_len = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == ':'))
            .unwrap_or(rest.len());
        if key_len == 0 {
            return Err(format!(
                "atributo inválido em `<{name}>`: `{}`",
                first_line(rest)
            ));
        }
        let key = &rest[..key_len];
        let after_key = rest[key_len..].trim_start();
        let Some(after_eq) = after_key.strip_prefix('=') else {
            return Err(format!(
                "atributo `{key}` sem valor em `<{name}>` (use {key}=\"…\")"
            ));
        };
        let after_eq = after_eq.trim_start();
        let quote = after_eq.chars().next().filter(|c| *c == '"' || *c == '\'');
        let Some(quote) = quote else {
            return Err(format!("valor de `{key}` em `<{name}>` precisa de aspas"));
        };
        let value_rest = &after_eq[1..];
        let Some(end) = value_rest.find(quote) else {
            return Err(format!(
                "aspas não fechadas no atributo `{key}` de `<{name}>`"
            ));
        };
        if attrs
            .insert(key.to_string(), value_rest[..end].to_string())
            .is_some()
        {
            return Err(format!("atributo `{key}` repetido em `<{name}>`"));
        }
        rest = &value_rest[end + 1..];
    }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_self_closing() {
        let (n, a) = parse_tag(r#"<DocExample file="Basic" title='Um "x"' />"#).unwrap();
        assert_eq!(n, "DocExample");
        assert_eq!(a["file"], "Basic");
        assert_eq!(a["title"], "Um \"x\"");
    }

    #[test]
    fn requires_self_closing() {
        assert!(parse_tag(r#"<A x="1">"#).is_err());
        assert!(parse_tag(r#"<A x="1"></A>"#).is_err());
    }

    #[test]
    fn rejects_bad_attrs() {
        assert!(parse_tag("<A flag />").is_err());
        assert!(parse_tag("<A x=1 />").is_err());
        assert!(parse_tag(r#"<A x="1 />"#).is_err());
        assert!(parse_tag(r#"<A x="1" x="2" />"#).is_err());
    }

    #[test]
    fn rejects_two_elements() {
        assert!(parse_tag("<A /> <B />").is_err());
    }

    #[test]
    fn normalizes() {
        assert_eq!(normalize(Path::new("a/b/../c")), "a/c");
    }
}
