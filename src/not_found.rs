use leptos::prelude::*;
use leptos_meta::Title;
use leptos_nebula::{Btn, Color};

/// Página 404 (fallback das rotas).
#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <Title text="404" />
        <div class="lxp-not-found">
            <h1>"404"</h1>
            <p>"Esta página não existe."</p>
            <Btn label="Voltar ao início" href="/" color=Color::Primary />
        </div>
    }
}
