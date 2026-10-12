use leptos::prelude::*;
use leptos_nebula::{Item, ItemLabel, ItemSection, List};
use leptos_router::hooks::use_location;

use crate::{SiteMeta, normalize_path};

/// Grupos da sidebar; o item da rota atual fica ativo.
#[component]
pub fn Sidebar(site: &'static SiteMeta) -> impl IntoView {
    let location = use_location();
    let groups = site
        .sidebar
        .iter()
        .map(|group| {
            let items = group
                .items
                .iter()
                .map(|item| {
                    let link = item.link;
                    let active =
                        Signal::derive(move || normalize_path(&location.pathname.get()) == link);
                    view! {
                        <Item href=link active=active dense=true>
                            <ItemSection>
                                <ItemLabel>{item.text}</ItemLabel>
                            </ItemSection>
                        </Item>
                    }
                })
                .collect_view();
            view! {
                <ItemLabel header=true>{group.text}</ItemLabel>
                {items}
            }
        })
        .collect_view();
    view! {
        <nav class="lxp-sidebar" aria-label="Menu lateral">
            <List padding=true>{groups}</List>
        </nav>
    }
}
