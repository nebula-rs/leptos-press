use leptos::prelude::*;
use leptos_nebula::{Btn, BtnDesign, Card, CardSection, Color};

use crate::HomeMeta;

/// `layout: home`: hero (nome, texto, tagline, ações) e features, como a home do VitePress.
/// O Markdown da página (se houver) vem abaixo.
#[component]
pub fn HomeLayout(home: &'static HomeMeta, children: Children) -> impl IntoView {
    let actions = home
        .actions
        .iter()
        .map(|a| {
            let (design, color) = if a.primary {
                (BtnDesign::Unelevated, Color::Primary)
            } else {
                (BtnDesign::Outline, Color::Neutral)
            };
            view! { <Btn label=a.text href=a.link design=design color=color /> }
        })
        .collect_view();
    let features = home
        .features
        .iter()
        .map(|f| {
            view! {
                <Card bordered=true flat=true>
                    <CardSection>
                        {(!f.icon.is_empty()).then(|| view! { <div class="lxp-feature-icon">{f.icon}</div> })}
                        <h3 class="lxp-feature-title">{f.title}</h3>
                        <p class="lxp-feature-details">{f.details}</p>
                    </CardSection>
                </Card>
            }
        })
        .collect_view();
    view! {
        <section class="lxp-hero">
            {home.name.map(|n| view! { <h1 class="lxp-hero-name">{n}</h1> })}
            {home.text.map(|t| view! { <p class="lxp-hero-text">{t}</p> })}
            {home.tagline.map(|t| view! { <p class="lxp-hero-tagline">{t}</p> })}
            <div class="lxp-hero-actions">{actions}</div>
        </section>
        <section class="lxp-features">{features}</section>
        {children()}
    }
}
