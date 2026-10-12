//! Entrega do CSS do press (mesmo modelo do `UiStyles`, ADR-0005).

use leptos::prelude::*;
use leptos_meta::Style;

/// CSS pronto do press (prose, realce de código e shortcuts `lxp-*`). Gerado por
/// `moon run leptos-press:css` — nunca editar `style/dist/press.css` à mão. Depende do
/// `ui.css` (tokens e reset): carregue [`PressStyles`] **depois** de `UiStyles`.
pub const CSS: &str = include_str!("../style/dist/press.css");

/// Injeta [`CSS`] no `<head>` via `leptos_meta` (requer `provide_meta_context()`).
#[component]
pub fn PressStyles() -> impl IntoView {
    view! { <Style>{CSS}</Style> }
}
