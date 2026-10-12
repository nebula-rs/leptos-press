use leptos::children::ToChildren;
use leptos::prelude::*;
use leptos_meta::provide_meta_context;
use leptos_nebula::{
    Btn, BtnDesign, Color, Drawer, Header, Layout, Page, PageContainer, ThemeSwitcher, Toolbar,
    ToolbarTitle, UiStyles, provide_theme,
};
use leptos_router::MatchNestedRoutes;
use leptos_router::components::{Router, Routes, RoutesProps};
use leptos_router::hooks::use_location;

use crate::{NotFound, Outline, PageLayout, PressStyles, Sidebar, SiteMeta, normalize_path};

/// Largura (px) até a qual a sidebar vira gaveta sobreposta.
const SIDEBAR_BREAKPOINT: u32 = 1023;

/// Raiz do site: contextos (meta, tema), estilos, `Router`, layout e as rotas geradas.
///
/// `routes` é a `routes()` gerada em `press.rs`.
#[component]
pub fn PressApp<Defs>(site: &'static SiteMeta, routes: fn() -> Defs) -> impl IntoView
where
    Defs: MatchNestedRoutes + Clone + Send + 'static,
{
    provide_meta_context();
    provide_theme();
    provide_context(site);
    // `Routes` é chamado como função: no `view!`, `{routes()}` seria tratado como view.
    let router = move || {
        Routes(
            RoutesProps::builder()
                .fallback(NotFound)
                .children(ToChildren::to_children(routes))
                .build(),
        )
    };
    view! {
        <UiStyles />
        <PressStyles />
        <Router>
            <PressLayout site=site>{router()}</PressLayout>
        </Router>
    }
}

/// Cabeçalho (título, nav, tema, menu no mobile), sidebar à esquerda e, no centro, conteúdo +
/// outline juntos num container centralizado (`lxp-doc-*`, como o `VPDoc` do VitePress). O
/// outline é um `<aside>` sticky que só aparece a partir de 1280px. A home (`layout: home`) não
/// tem sidebar nem outline.
#[component]
pub fn PressLayout(site: &'static SiteMeta, children: Children) -> impl IntoView {
    let location = use_location();
    let current = Memo::new(move |_| site.page(&location.pathname.get()));
    let is_doc = Memo::new(move |_| current.get().is_none_or(|p| p.layout == PageLayout::Doc));
    let has_outline = Memo::new(move |_| current.get().is_some_and(|p| !p.headings.is_empty()));

    let sidebar_open = RwSignal::new(true);

    // Em tela estreita a gaveta fecha ao navegar. A largura só existe no navegador (Effect).
    Effect::new(move |_| {
        location.pathname.track();
        let narrow = web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|w| w.as_f64())
            .is_some_and(|w| w <= f64::from(SIDEBAR_BREAKPOINT));
        if narrow {
            sidebar_open.set(false);
        }
    });

    let nav = move || {
        let path = normalize_path(&location.pathname.get()).to_string();
        site.nav
            .iter()
            .map(|item| {
                let active = path == item.prefix || path.starts_with(&format!("{}/", item.prefix));
                // Dois shortcuts completos, não base + modificador.
                let class = if active {
                    "lxp-nav-link-active"
                } else {
                    "lxp-nav-link"
                };
                let current = active.then_some("page");
                view! {
                    <a class=class href=item.link aria-current=current>
                        {item.text}
                    </a>
                }
            })
            .collect_view()
    };

    let doc_class = move || {
        if !is_doc.get() {
            "lxp-doc-wide"
        } else if has_outline.get() {
            "lxp-doc-aside"
        } else {
            "lxp-doc-plain"
        }
    };

    view! {
        <Layout>
            <Header bordered=true>
                <Toolbar>
                    <Show when=move || is_doc.get()>
                        <div class="lxp-menu-btn">
                            <Btn
                                round=true
                                dense=true
                                design=BtnDesign::Flat
                                color=Color::Neutral
                                icon="i-lucide-menu"
                                aria_label="Abrir menu"
                                on_click=move |_| sidebar_open.update(|v| *v = !*v)
                            />
                        </div>
                    </Show>
                    <ToolbarTitle>
                        <a class="lxp-brand" href="/">{site.title}</a>
                    </ToolbarTitle>
                    <nav class="lxp-nav" aria-label="Principal">{nav}</nav>
                    <ThemeSwitcher />
                </Toolbar>
            </Header>
            <Show when=move || is_doc.get()>
                <Drawer
                    show=sidebar_open
                    bordered=true
                    width=280
                    breakpoint=SIDEBAR_BREAKPOINT
                    show_if_above=true
                >
                    <Sidebar site=site />
                </Drawer>
            </Show>
            <PageContainer>
                <Page>
                    <div class="lxp-page">
                        <div class=doc_class>
                            <div class="lxp-content">{children()}</div>
                            <Show when=move || has_outline.get()>
                                <aside class="lxp-aside">
                                    {move || current.get().map(|page| view! { <Outline page=page /> })}
                                </aside>
                            </Show>
                        </div>
                    </div>
                </Page>
            </PageContainer>
        </Layout>
    }
}
