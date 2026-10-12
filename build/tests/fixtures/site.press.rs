/// Metadados do site: título, nav, sidebar e páginas.
pub static SITE: ::leptos_press::SiteMeta = ::leptos_press::SiteMeta {
    title: "leptos-nebula",
    description: "Componentes Leptos",
    nav: &[
        ::leptos_press::NavLink {
            text: "Guia",
            link: "/guide/start",
            prefix: "/guide",
        },
        ::leptos_press::NavLink {
            text: "Componentes",
            link: "/components/btn",
            prefix: "/components",
        },
    ],
    sidebar: &[
        ::leptos_press::SidebarGroup {
            text: "Guia",
            items: &[
                ::leptos_press::SidebarItem {
                    text: "Começando",
                    link: "/guide/start",
                },
                ::leptos_press::SidebarItem {
                    text: "Temas",
                    link: "/guide/theming",
                },
            ],
        },
        ::leptos_press::SidebarGroup {
            text: "Componentes",
            items: &[
                ::leptos_press::SidebarItem {
                    text: "Btn",
                    link: "/components/btn",
                },
            ],
        },
    ],
    pages: &[
        ::leptos_press::PageMeta {
            path: "/components/btn",
            title: "Btn",
            description: "Botão",
            layout: ::leptos_press::PageLayout::Doc,
            headings: &[
                ::leptos_press::Heading {
                    level: 2u8,
                    id: "props",
                    text: "Props",
                },
            ],
            home: ::core::option::Option::None,
        },
        ::leptos_press::PageMeta {
            path: "/guide/start",
            title: "Começando",
            description: "Como começar",
            layout: ::leptos_press::PageLayout::Doc,
            headings: &[],
            home: ::core::option::Option::None,
        },
        ::leptos_press::PageMeta {
            path: "/guide/theming",
            title: "Temas",
            description: "Tokens",
            layout: ::leptos_press::PageLayout::Doc,
            headings: &[],
            home: ::core::option::Option::None,
        },
        ::leptos_press::PageMeta {
            path: "/",
            title: "Início",
            description: "",
            layout: ::leptos_press::PageLayout::Home,
            headings: &[],
            home: ::core::option::Option::Some(
                &::leptos_press::HomeMeta {
                    name: ::core::option::Option::Some("Meu site"),
                    text: ::core::option::Option::None,
                    tagline: ::core::option::Option::Some("Um site"),
                    actions: &[
                        ::leptos_press::HeroAction {
                            text: "Começar",
                            link: "/guide/start",
                            primary: true,
                        },
                    ],
                    features: &[
                        ::leptos_press::Feature {
                            title: "Rápido",
                            details: "Muito",
                            icon: "",
                        },
                    ],
                },
            ),
        },
    ],
};
mod mod_basic;
/// /components/btn (`content/components/btn.md`)
fn page_0() -> ::leptos::prelude::AnyView {
    ::leptos_press::render_page(
        &SITE.pages[0],
        ::std::vec![
            ::leptos_press::html("<h1 id=\"btn\">Btn<a class=\"header-anchor\" href=\"#btn\" aria-label=\"Permalink\">#</a></h1>\n<h2 id=\"props\">Props<a class=\"header-anchor\" href=\"#props\" aria-label=\"Permalink\">#</a></h2>\n")
        ],
    )
}
/// /guide/start (`content/guide/start.md`)
fn page_1() -> ::leptos::prelude::AnyView {
    ::leptos_press::render_page(
        &SITE.pages[1],
        ::std::vec![
            ::leptos_press::html("<h1 id=\"começando\">Começando<a class=\"header-anchor\" href=\"#começando\" aria-label=\"Permalink\">#</a></h1>\n<p>Veja <a href=\"/components/btn\">a API</a>.</p>\n")
        ],
    )
}
/// /guide/theming (`content/guide/theming.md`)
fn page_2() -> ::leptos::prelude::AnyView {
    ::leptos_press::render_page(
        &SITE.pages[2],
        ::std::vec![
            ::leptos_press::html("<h1 id=\"temas\">Temas<a class=\"header-anchor\" href=\"#temas\" aria-label=\"Permalink\">#</a></h1>\n"),
            ::leptos::prelude::IntoAny::into_any(echo("", "Basic", ""))
        ],
    )
}
/// / (`content/index.md`)
fn page_3() -> ::leptos::prelude::AnyView {
    ::leptos_press::render_page(&SITE.pages[3], ::std::vec![])
}
/// Uma rota estática (`SsrMode::Static`) por página.
pub fn routes() -> impl ::leptos_router::MatchNestedRoutes + ::core::clone::Clone + ::core::marker::Send + 'static {
    (
        ::leptos_router::components::Route(
            ::leptos_router::components::RouteProps::builder()
                .path(::leptos_router::path!("components/btn"))
                .view(page_0)
                .ssr(
                    ::leptos_router::SsrMode::Static(
                        ::leptos_router::static_routes::StaticRoute::new(),
                    ),
                )
                .build(),
        ),
        ::leptos_router::components::Route(
            ::leptos_router::components::RouteProps::builder()
                .path(::leptos_router::path!("guide/start"))
                .view(page_1)
                .ssr(
                    ::leptos_router::SsrMode::Static(
                        ::leptos_router::static_routes::StaticRoute::new(),
                    ),
                )
                .build(),
        ),
        ::leptos_router::components::Route(
            ::leptos_router::components::RouteProps::builder()
                .path(::leptos_router::path!("guide/theming"))
                .view(page_2)
                .ssr(
                    ::leptos_router::SsrMode::Static(
                        ::leptos_router::static_routes::StaticRoute::new(),
                    ),
                )
                .build(),
        ),
        ::leptos_router::components::Route(
            ::leptos_router::components::RouteProps::builder()
                .path(::leptos_router::path!(""))
                .view(page_3)
                .ssr(
                    ::leptos_router::SsrMode::Static(
                        ::leptos_router::static_routes::StaticRoute::new(),
                    ),
                )
                .build(),
        ),
    )
}
