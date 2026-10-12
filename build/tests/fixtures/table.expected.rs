// title: Tabela
// description: 
// h1 #tabela "Tabela"
/// /teste (`content/teste.md`)
fn page() -> ::leptos::prelude::AnyView {
    ::leptos_press::render_page(
        &SITE.pages[0],
        ::std::vec![
            ::leptos_press::html("<h1 id=\"tabela\">Tabela<a class=\"header-anchor\" href=\"#tabela\" aria-label=\"Permalink\">#</a></h1>\n"),
            ::leptos_press::table("<thead>\n<tr><th style=\"text-align:left\">Prop</th><th style=\"text-align:center\">Tipo</th><th style=\"text-align:right\">Padrão</th></tr>\n</thead>\n<tbody>\n<tr><td style=\"text-align:left\"><code>color</code></td><td style=\"text-align:center\"><code>Color</code></td><td style=\"text-align:right\"><code>Primary</code></td></tr>\n<tr><td style=\"text-align:left\"><code>round</code></td><td style=\"text-align:center\"><code>bool</code></td><td style=\"text-align:right\"><code>false</code></td></tr>\n</tbody>\n")
        ],
    )
}
