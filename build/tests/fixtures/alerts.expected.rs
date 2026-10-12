// title: Alertas
// description: 
// h1 #alertas "Alertas"
/// /teste (`content/teste.md`)
fn page() -> ::leptos::prelude::AnyView {
    ::leptos_press::render_page(
        &SITE.pages[0],
        ::std::vec![
            ::leptos_press::html("<h1 id=\"alertas\">Alertas<a class=\"header-anchor\" href=\"#alertas\" aria-label=\"Permalink\">#</a></h1>\n"),
            ::leptos_press::alert(::leptos_press::AlertKind::Note, "Note",
            ::std::vec![::leptos_press::html("<p>Uma nota com <code>código</code>.</p>\n")],),
            ::leptos_press::alert(::leptos_press::AlertKind::Tip, "Tip",
            ::std::vec![::leptos_press::html("<p>Dica.</p>\n")],),
            ::leptos_press::alert(::leptos_press::AlertKind::Important, "Important",
            ::std::vec![::leptos_press::html("<p>Importante.</p>\n")],),
            ::leptos_press::alert(::leptos_press::AlertKind::Warning, "Warning",
            ::std::vec![::leptos_press::html("<p>Aviso.</p>\n")],),
            ::leptos_press::alert(::leptos_press::AlertKind::Caution, "Caution",
            ::std::vec![::leptos_press::html("<p>Cuidado.</p>\n")],),
            ::leptos_press::alert(::leptos_press::AlertKind::Tip, "Tip",
            ::std::vec![::leptos_press::html("<p>Container com <strong>negrito</strong>.</p>\n")],),
            ::leptos_press::alert(::leptos_press::AlertKind::Warning, "Título próprio",
            ::std::vec![::leptos_press::html("<p>Texto.</p>\n"),
            ::leptos_press::code_block("rust", "fn dentro() {}",
            "<span class=\"source rust\"><span class=\"meta function rust\"><span class=\"meta function rust\"><span class=\"storage type function rust\">fn</span> </span><span class=\"entity name function rust\">dentro</span></span><span class=\"meta function rust\"><span class=\"meta function parameters rust\"><span class=\"punctuation section parameters begin rust\">(</span></span><span class=\"meta function rust\"><span class=\"meta function parameters rust\"><span class=\"punctuation section parameters end rust\">)</span></span></span></span><span class=\"meta function rust\"> </span><span class=\"meta function rust\"><span class=\"meta block rust\"><span class=\"punctuation section block begin rust\">{</span></span><span class=\"meta block rust\"><span class=\"punctuation section block end rust\">}</span></span></span></span>")],),
            ::leptos_press::alert(::leptos_press::AlertKind::Caution, "Danger",
            ::std::vec![::leptos_press::html("<p>Perigo.</p>\n")],),
            ::leptos_press::alert(::leptos_press::AlertKind::Note, "Info",
            ::std::vec![::leptos_press::html("<p>Informação.</p>\n")],)
        ],
    )
}
