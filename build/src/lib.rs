//! Lado de build do `leptos-press` (host only): lê o Markdown, percorre a AST do comrak e
//! gera código Leptos e os arquivos `llms*.txt`. Nunca entra no wasm.
//!
//! Uso, no `build.rs` do site:
//!
//! ```ignore
//! leptos_press_build::Site::new("content")
//!     .title("meu site")
//!     .nav([("Guia", "/guide/")])
//!     .sidebar_from_folders()
//!     .build()
//!     .unwrap_or_else(|e| panic!("{e}"));
//! ```
//!
//! Escreve `$OUT_DIR/press.rs` (incluído pelo site) e `$OUT_DIR/press-static/`.

mod component;
mod error;
mod frontmatter;
mod highlight;
mod html;
mod page;
mod render;
mod site;

pub use component::{Attrs, Ctx, MdComponent};
pub use error::Error;
pub use frontmatter::{Feature, FrontMatter, Hero, HeroAction, Layout};
pub use page::{CompiledPage, compile_page, format_rust, highlight_html};
pub use render::Heading;
pub use site::{Generated, Site};
