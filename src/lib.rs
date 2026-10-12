//! Documentação estilo VitePress em Leptos, com o visual do `leptos-nebula`.
//!
//! Runtime (wasm + ssr) do site; o lado de build (Markdown → código) vive em
//! `leptos-press-build`, que gera um `press.rs` com `SITE`, uma função por página e `routes()`.
//! Este crate fornece o layout (`PressApp`, `PressLayout`, `Sidebar`, `Outline`, `PrevNext`,
//! `HomeLayout`), os blocos que o código gerado chama (`html`, `table`, `code_block`, `alert`,
//! `render_page`) e o CSS (`PressStyles`).
//!
//! Seguro para SSR: nada de API do navegador durante a renderização (ADR-0007). As features
//! `csr`/`hydrate`/`ssr` são repassadas pelo app. Ver `docs/press/overview.md`.

// O `view!` do `PressLayout` aninha muitos tipos; no build release (ssr) o cálculo de layout
// estoura o limite padrão de 128.
#![recursion_limit = "256"]

mod code;
mod content;
mod home;
mod layout;
mod meta;
mod not_found;
mod outline;
mod prev_next;
mod sidebar;
mod styles;

pub use code::CodeBlock;
pub use content::{
    AlertKind, ExampleBox, Prose, alert, code_block, code_block_attached, html, render_page, table,
};
pub use home::HomeLayout;
pub use layout::{PressApp, PressLayout};
pub use meta::{
    Feature, Heading, HeroAction, HomeMeta, NavLink, PageLayout, PageMeta, SidebarGroup,
    SidebarItem, SiteMeta, normalize_path,
};
pub use not_found::NotFound;
pub use outline::Outline;
pub use prev_next::PrevNext;
pub use sidebar::Sidebar;
pub use styles::{CSS, PressStyles};
