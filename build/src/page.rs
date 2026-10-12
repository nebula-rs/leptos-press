//! Compilação de uma página: Markdown → blocos de código + texto para LLM.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use comrak::{Arena, Options, parse_document};
use proc_macro2::TokenStream;
use quote::quote;

use crate::component::MdComponent;
use crate::error::Error;
use crate::frontmatter::{self, FrontMatter};
use crate::highlight::Highlighter;
use crate::render::{Heading, InternalLink, Renderer};

/// HTML realçado (spans com classes) de um trecho de código, como o dos blocos ```` ``` ````.
/// Para tags de site (ex.: `DocExample`) que mostram código-fonte.
pub fn highlight_html(lang: &str, code: &str) -> String {
    highlighter().highlight(lang, code)
}

fn highlighter() -> &'static Highlighter {
    static HIGHLIGHTER: OnceLock<Highlighter> = OnceLock::new();
    HIGHLIGHTER.get_or_init(Highlighter::new)
}

/// Resultado da compilação de um `.md`.
pub struct CompiledPage {
    /// Rota (`/components/btn`).
    pub route: String,
    /// Caminho do `.md`, como foi informado.
    pub file: PathBuf,
    pub front: FrontMatter,
    pub title: String,
    pub description: String,
    /// Todos os cabeçalhos, na ordem do texto.
    pub headings: Vec<Heading>,
    /// Um item por bloco; cada um avalia para `AnyView`.
    pub blocks: Vec<TokenStream>,
    /// Itens de módulo pedidos pelas tags (sem repetição).
    pub items: Vec<TokenStream>,
    /// Markdown da página com as tags já expandidas pela saída `llms` (sem front-matter).
    pub markdown: String,
    pub(crate) links: Vec<InternalLink>,
    /// Arquivos lidos pelas tags.
    pub reads: Vec<PathBuf>,
}

/// Compila uma página. `file` e `route` só alimentam mensagens e o contexto das tags.
pub fn compile_page(
    src: &str,
    file: &Path,
    route: &str,
    root: &Path,
    components: &[Box<dyn MdComponent>],
) -> Result<CompiledPage, Error> {
    let (front, body) = frontmatter::split(src, file)?;
    let skip_lines = body.bytes().take_while(|b| *b == b'\n').count();

    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.alerts = true;
    options.extension.block_directive = true;

    let arena = Arena::new();
    let document = parse_document(&arena, &body, &options);

    let reads = RefCell::new(Vec::new());
    let renderer = Renderer::new(file, route, root, &front, components, highlighter(), &reads);
    let blocks = renderer.blocks(document)?;
    let headings = renderer.headings.borrow().clone();

    let h1 = headings
        .iter()
        .find(|h| h.level == 1)
        .map(|h| h.text.clone());
    let title = front
        .title
        .clone()
        .or(h1.clone())
        .or_else(|| file.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let description = front.description.clone().unwrap_or_default();

    let mut markdown = renderer.llms_markdown(src, skip_lines);
    if h1.is_none() {
        markdown = format!("# {title}\n\n{markdown}");
    }
    let mut items: Vec<TokenStream> = Vec::new();
    for item in renderer.items.borrow().iter() {
        if !items.iter().any(|i| i.to_string() == item.to_string()) {
            items.push(item.clone());
        }
    }
    let links = renderer.links.borrow().clone();
    drop(renderer);

    Ok(CompiledPage {
        route: route.to_string(),
        file: file.to_path_buf(),
        front,
        title,
        description,
        headings,
        blocks,
        items,
        markdown,
        links,
        reads: reads.into_inner(),
    })
}

impl CompiledPage {
    /// A função da página, como entra no `press.rs` (`meta_index` = posição em `SITE.pages`).
    pub fn page_fn(&self, name: &syn::Ident, meta_index: usize) -> TokenStream {
        let blocks = &self.blocks;
        let meta_index = proc_macro2::Literal::usize_unsuffixed(meta_index);
        let doc = format!(" {} (`{}`)", self.route, self.file.display());
        quote! {
            #[doc = #doc]
            fn #name() -> ::leptos::prelude::AnyView {
                ::leptos_press::render_page(
                    &SITE.pages[#meta_index],
                    ::std::vec![ #( #blocks ),* ],
                )
            }
        }
    }
}

/// Formata código gerado com `prettyplease`.
pub fn format_rust(tokens: TokenStream) -> Result<String, Error> {
    let file: syn::File = syn::parse2(tokens)
        .map_err(|e| Error::new(format!("o código gerado não é Rust válido: {e}")))?;
    Ok(prettyplease::unparse(&file))
}
