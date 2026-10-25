use leptos::prelude::*;
use leptos_nebula::{ExpansionItem, ExpansionItemHeader, Item, ItemLabel, ItemSection, List};
use leptos_router::hooks::use_location;

use crate::{SiteMeta, normalize_path};

/// Grupos da sidebar; o item da rota atual fica ativo.
///
/// Grupo com `collapsed` (`Some`) é um [`ExpansionItem`]: começa aberto se `Some(false)` ou se
/// contém a página atual (a rota vem do `Router`, então o HTML do SSG já sai certo e a
/// hidratação não muda nada); ao navegar para uma página do grupo, ele abre sozinho.
#[component]
pub fn Sidebar(site: &'static SiteMeta) -> impl IntoView {
    let location = use_location();
    let groups = site
        .sidebar
        .iter()
        .enumerate()
        .map(|(index, group)| {
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
            match group.collapsed {
                None => view! {
                    <ItemLabel header=true>{group.text}</ItemLabel>
                    {items}
                }
                .into_any(),
                Some(_) => {
                    let open = RwSignal::new(group.open_for(&location.pathname.get_untracked()));
                    Effect::new(move |_| {
                        if group.contains(&location.pathname.get()) {
                            open.set(true);
                        }
                    });
                    let class = if index == 0 {
                        "lxp-sidebar-group"
                    } else {
                        "lxp-sidebar-group-spaced"
                    };
                    // O cabeçalho usa o `ItemLabel header` (mesmo visual do grupo fixo); só o
                    // chevron do `ExpansionItem` fica à direita.
                    view! {
                        <div class=class>
                            <ExpansionItem value=open dense=true>
                                <ExpansionItemHeader slot:header>
                                    <ItemSection>
                                        <ItemLabel header=true>{group.text}</ItemLabel>
                                    </ItemSection>
                                </ExpansionItemHeader>
                                {items}
                            </ExpansionItem>
                        </div>
                    }
                    .into_any()
                }
            }
        })
        .collect_view();
    view! {
        <nav class="lxp-sidebar" aria-label="Menu lateral">
            <List padding=true>{groups}</List>
        </nav>
    }
}
