// title: Código
// description: 
// h1 #código "Código"
/// /teste (`content/teste.md`)
fn page() -> ::leptos::prelude::AnyView {
    ::leptos_press::render_page(
        &SITE.pages[0],
        ::std::vec![
            ::leptos_press::html("<h1 id=\"código\">Código<a class=\"header-anchor\" href=\"#código\" aria-label=\"Permalink\">#</a></h1>\n"),
            ::leptos_press::code_block("rust",
            "fn main() {\n    println!(\"oi <mundo> & cia\");\n}",
            "<span class=\"source rust\"><span class=\"meta function rust\"><span class=\"meta function rust\"><span class=\"storage type function rust\">fn</span> </span><span class=\"entity name function rust\">main</span></span><span class=\"meta function rust\"><span class=\"meta function parameters rust\"><span class=\"punctuation section parameters begin rust\">(</span></span><span class=\"meta function rust\"><span class=\"meta function parameters rust\"><span class=\"punctuation section parameters end rust\">)</span></span></span></span><span class=\"meta function rust\"> </span><span class=\"meta function rust\"><span class=\"meta block rust\"><span class=\"punctuation section block begin rust\">{</span>\n    <span class=\"support macro rust\">println!</span><span class=\"meta group rust\"><span class=\"punctuation section group begin rust\">(</span></span><span class=\"meta group rust\"><span class=\"string quoted double rust\"><span class=\"punctuation definition string begin rust\">&quot;</span>oi &lt;mundo&gt; &amp; cia<span class=\"punctuation definition string end rust\">&quot;</span></span></span><span class=\"meta group rust\"><span class=\"punctuation section group end rust\">)</span></span><span class=\"punctuation terminator rust\">;</span>\n</span><span class=\"meta block rust\"><span class=\"punctuation section block end rust\">}</span></span></span></span>"),
            ::leptos_press::code_block("linguagem-desconhecida", "a < b", "a &lt; b"),
            ::leptos_press::code_block("", "sem linguagem", "sem linguagem"),
            ::leptos_press::code_block("", "bloco indentado", "bloco indentado"),
            ::leptos_press::html("<p>Dentro de lista:</p>\n<ul>\n<li><p>item</p>\n<pre><code class=\"language-rust\"><span class=\"source rust\"><span class=\"storage type rust\">let</span> x <span class=\"keyword operator rust\">=</span> <span class=\"constant numeric integer decimal rust\">1</span><span class=\"punctuation terminator rust\">;</span></span></code></pre>\n</li>\n</ul>\n")
        ],
    )
}
