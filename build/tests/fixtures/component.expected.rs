// title: Tags
// description: 
// h1 #tags "Tags"
// item: mod mod_basic ;
// item: mod mod_outro ;
/// /teste (`content/teste.md`)
fn page() -> ::leptos::prelude::AnyView {
    ::leptos_press::render_page(
        &SITE.pages[0],
        ::std::vec![
            ::leptos_press::html("<h1 id=\"tags\">Tags<a class=\"header-anchor\" href=\"#tags\" aria-label=\"Permalink\">#</a></h1>\n<p>Antes.</p>\n"),
            ::leptos::prelude::IntoAny::into_any(echo("Btn", "Basic", "Um exemplo")),
            ::leptos_press::html("<p>Depois.</p>\n"),
            ::leptos::prelude::IntoAny::into_any(echo("Btn", "Outro", ""))
        ],
    )
}
