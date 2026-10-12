use leptos::prelude::*;
use leptos_nebula::{Item, ItemLabel, ItemSection, List};

use crate::PageMeta;

/// `h2`/`h3` da página, com links para as âncoras.
#[component]
pub fn Outline(page: &'static PageMeta) -> impl IntoView {
    let items = page
        .headings
        .iter()
        .map(|h| {
            let href = format!("#{}", h.id);
            let inset = if h.level >= 3 { 0.3 } else { 0.0 };
            view! {
                <Item href=href dense=true inset_level=inset>
                    <ItemSection>
                        <ItemLabel>{h.text}</ItemLabel>
                    </ItemSection>
                </Item>
            }
        })
        .collect_view();
    view! {
        <nav class="lxp-outline" aria-label="Nesta página">
            <ItemLabel header=true>"Nesta página"</ItemLabel>
            <List dense=true>{items}</List>
        </nav>
    }
}
