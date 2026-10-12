// title: Cabeçalhos
// description: Teste de cabeçalhos e outline
// h1 #cabeçalhos "Cabeçalhos"
// h2 #primeira-seção "Primeira seção"
// h3 #subseção "Subseção"
// h2 #primeira-seção-1 "Primeira seção"
// h4 #nível-quatro-fora-do-outline "Nível quatro (fora do outline)"
// h2 #com-código-e-acentuação-símbolos "Com código e <acentuação> & símbolos"
/// /teste (`content/teste.md`)
fn page() -> ::leptos::prelude::AnyView {
    ::leptos_press::render_page(
        &SITE.pages[0],
        ::std::vec![
            ::leptos_press::html("<h1 id=\"cabeçalhos\">Cabeçalhos<a class=\"header-anchor\" href=\"#cabeçalhos\" aria-label=\"Permalink\">#</a></h1>\n<p>Texto com <strong>negrito</strong>, <em>ênfase</em>, <del>riscado</del> e <code>código</code>.</p>\n<h2 id=\"primeira-seção\">Primeira seção<a class=\"header-anchor\" href=\"#primeira-seção\" aria-label=\"Permalink\">#</a></h2>\n<h3 id=\"subseção\">Subseção<a class=\"header-anchor\" href=\"#subseção\" aria-label=\"Permalink\">#</a></h3>\n<h2 id=\"primeira-seção-1\">Primeira seção<a class=\"header-anchor\" href=\"#primeira-seção-1\" aria-label=\"Permalink\">#</a></h2>\n<h4 id=\"nível-quatro-fora-do-outline\">Nível quatro (fora do outline)<a class=\"header-anchor\" href=\"#nível-quatro-fora-do-outline\" aria-label=\"Permalink\">#</a></h4>\n<h2 id=\"com-código-e-acentuação-símbolos\">Com <code>código</code> e &lt;acentuação&gt; &amp; símbolos<a class=\"header-anchor\" href=\"#com-código-e-acentuação-símbolos\" aria-label=\"Permalink\">#</a></h2>\n<ul>\n<li>item um</li>\n<li>item dois com <a href=\"https://example.com\" target=\"_blank\" rel=\"noopener noreferrer\" title=\"Título\">link</a></li>\n</ul>\n<ol>\n<li>primeiro</li>\n<li>segundo</li>\n</ol>\n<blockquote>\n<p>citação</p>\n</blockquote>\n<hr>\n<ul>\n<li><input type=\"checkbox\" disabled checked> feito</li>\n<li><input type=\"checkbox\" disabled> pendente</li>\n</ul>\n")
        ],
    )
}
