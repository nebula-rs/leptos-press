//! Busca do site (R-37, ADR-0015): local, no navegador, sobre o `llms-full.txt` do plugin
//! `leptos-press-llms`.
//!
//! - [`index`]: leitor do arquivo e índice por seção (puro, testado no host);
//! - [`query`]: termos, ranking, agrupamento por página e trecho com destaque (puro);
//! - `view` (interno): o botão "Buscar" do cabeçalho, os atalhos e o diálogo.
//!
//! Ver `docs/press/overview.md` (seção "Busca").

pub mod index;
pub mod query;
mod view;

pub use index::{IndexedPage, RawPage, SearchIndex, Section, split_pages, url_to_route};
pub use query::{
    Hit, HitGroup, MAX_RESULTS, SNIPPET_CHARS, Segment, group, highlight, search, snippet, terms,
};
pub(crate) use view::Search;
