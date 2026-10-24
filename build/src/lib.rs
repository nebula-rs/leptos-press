//! Lado de build do `leptos-press` (host only): lê o Markdown, percorre a AST do comrak e
//! gera código Leptos; os arquivos estáticos (`llms.txt`, bundle OKF…) vêm dos plugins
//! ([`Plugin`]). Nunca entra no wasm.
//!
//! Uso, no `build.rs` do site:
//!
//! ```ignore
//! leptos_press_build::Site::new("content")
//!     .title("meu site")
//!     .nav([("Guia", "/guide/")])
//!     .sidebar_from_folders()
//!     .plugin(leptos_press_build::LlmsPlugin)
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
mod llms;
mod page;
mod plugin;
mod render;
mod site;

pub use component::{Attrs, Ctx, MdComponent};
pub use error::Error;
pub use frontmatter::{Feature, FrontMatter, Hero, HeroAction, Layout};
pub use llms::LlmsPlugin;
pub use page::{CompiledPage, compile_page, format_rust, highlight_html};
pub use plugin::{GroupData, Output, PageData, Plugin, SiteData, map_links};
pub use render::Heading;
pub use site::{Generated, Site};

/// O YAML usado no front-matter ([`FrontMatter::extra`]), para os plugins usarem a mesma versão.
pub use serde_yaml_ng as yaml;
