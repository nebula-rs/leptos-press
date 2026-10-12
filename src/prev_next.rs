use leptos::prelude::*;
use leptos_nebula::{Btn, BtnDesign, Card, Color};

use crate::SiteMeta;

/// Página anterior e próxima, na ordem da sidebar.
#[component]
pub fn PrevNext(path: &'static str) -> impl IntoView {
    let site = use_context::<&'static SiteMeta>()?;
    let (prev, next) = site.neighbors(path);
    if prev.is_none() && next.is_none() {
        return None;
    }
    Some(view! {
        <nav class="lxp-prev-next" aria-label="Páginas vizinhas">
            <div class="lxp-prev">
                {prev.map(|p| view! {
                    <Card bordered=true flat=true>
                        <Btn design=BtnDesign::Flat color=Color::Neutral href=p.link icon="i-lucide-arrow-left" label=p.text />
                    </Card>
                })}
            </div>
            <div class="lxp-next">
                {next.map(|n| view! {
                    <Card bordered=true flat=true>
                        <Btn design=BtnDesign::Flat color=Color::Neutral href=n.link icon_right="i-lucide-arrow-right" label=n.text />
                    </Card>
                })}
            </div>
        </nav>
    })
}
