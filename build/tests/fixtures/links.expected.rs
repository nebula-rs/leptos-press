// title: Links
// description: 
// h1 #links "Links"
/// /teste (`content/teste.md`)
fn page() -> ::leptos::prelude::AnyView {
    ::leptos_press::render_page(
        &SITE.pages[0],
        ::std::vec![
            ::leptos_press::html("<h1 id=\"links\">Links<a class=\"header-anchor\" href=\"#links\" aria-label=\"Permalink\">#</a></h1>\n<p><a href=\"https://example.com\" target=\"_blank\" rel=\"noopener noreferrer\">externo</a>, <a href=\"#links\">âncora</a>, <a href=\"/guide/other\">interno</a> e <a href=\"/other#x\">relativo</a>.</p>\n<p><img src=\"/img.png\" alt=\"imagem\" loading=\"lazy\"></p>\n")
        ],
    )
}
