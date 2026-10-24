//! Fixtures `.md` → código gerado e texto para LLM esperados (snapshots).
//! `UPDATE_EXPECT=1 cargo test -p leptos-press-build` regrava os arquivos esperados.

use std::path::{Path, PathBuf};

use leptos_press_build::{
    Attrs, CompiledPage, Ctx, Error, MdComponent, Output, PageData, Plugin, Site, SiteData,
    compile_page, format_rust,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Tag de teste: o código chama `echo(file, titulo)` e a saída Markdown cita o arquivo.
struct Echo;

impl MdComponent for Echo {
    fn tag(&self) -> &'static str {
        "Echo"
    }
    fn codegen(&self, attrs: &Attrs, ctx: &Ctx) -> Result<TokenStream, Error> {
        let file = attrs.require("file")?;
        let title = attrs.get("title").unwrap_or("");
        let from = ctx.front.str("examples").unwrap_or("");
        Ok(quote! { echo(#from, #file, #title) })
    }
    fn items(&self, attrs: &Attrs, _ctx: &Ctx) -> Result<TokenStream, Error> {
        let name = format_ident!("mod_{}", attrs.require("file")?.to_lowercase());
        Ok(quote! { mod #name; })
    }
    fn markdown(&self, attrs: &Attrs, _ctx: &Ctx) -> Result<String, Error> {
        Ok(format!(
            "```rust\n// exemplo {}\n```",
            attrs.require("file")?
        ))
    }
}

fn components() -> Vec<Box<dyn MdComponent>> {
    vec![Box::new(Echo)]
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn compile(src: &str) -> Result<CompiledPage, Error> {
    compile_page(
        src,
        Path::new("content/teste.md"),
        "/teste",
        Path::new("."),
        &components(),
    )
}

fn snapshot(page: &CompiledPage) -> String {
    let mut out = format!(
        "// title: {}\n// description: {}\n",
        page.title, page.description
    );
    for h in &page.headings {
        out.push_str(&format!("// h{} #{} {:?}\n", h.level, h.id, h.text));
    }
    for item in &page.items {
        out.push_str(&format!("// item: {item}\n"));
    }
    let name = format_ident!("page");
    out + &format_rust(page.page_fn(&name, 0)).expect("código gerado válido")
}

fn expect(path: &Path, actual: &str) {
    if std::env::var_os("UPDATE_EXPECT").is_some() {
        std::fs::write(path, actual).expect("gravar esperado");
        return;
    }
    let expected = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("falta {} — rode com UPDATE_EXPECT=1", path.display()));
    assert_eq!(expected, actual, "snapshot diferente: {}", path.display());
}

fn check_case(case: &str) {
    let dir = fixtures_dir();
    let src = std::fs::read_to_string(dir.join(format!("{case}.md"))).expect("fixture");
    let page = compile(&src).unwrap_or_else(|e| panic!("{case}: {e}"));
    expect(&dir.join(format!("{case}.expected.rs")), &snapshot(&page));
    expect(&dir.join(format!("{case}.expected.md")), &page.markdown);
}

#[test]
fn headings_and_outline() {
    check_case("headings");
    let page =
        compile(&std::fs::read_to_string(fixtures_dir().join("headings.md")).unwrap()).unwrap();
    let ids: Vec<_> = page.headings.iter().map(|h| h.id.as_str()).collect();
    assert!(
        ids.contains(&"primeira-seção") && ids.contains(&"primeira-seção-1"),
        "{ids:?}"
    );
}

#[test]
fn alerts_and_containers() {
    check_case("alerts");
}

#[test]
fn table() {
    check_case("table");
}

#[test]
fn code_blocks() {
    check_case("code");
}

#[test]
fn links() {
    check_case("links");
}

#[test]
fn registered_tag() {
    check_case("component");
    let page =
        compile(&std::fs::read_to_string(fixtures_dir().join("component.md")).unwrap()).unwrap();
    assert!(page.markdown.contains("// exemplo Basic"));
    assert!(!page.markdown.contains("<Echo"));
}

fn err(src: &str) -> String {
    compile(src).err().expect("deveria falhar").to_string()
}

#[test]
fn unknown_tag_reports_line() {
    let e = err("# T\n\ntexto\n\n<Nope x=\"1\" />\n");
    assert_eq!(e, "content/teste.md:5: tag `<Nope>` não registrada");
}

#[test]
fn raw_html_is_an_error() {
    assert!(err("# T\n\n<div>oi</div>\n").contains("HTML cru não é permitido"));
    assert!(err("# T\n\ntexto <b>inline</b>\n").contains("HTML cru não é permitido"));
    assert!(err("# T\n\n<Echo file=\"A\"></Echo>\n").contains("autofechada"));
}

#[test]
fn bad_attributes_are_errors() {
    assert!(err("<Echo file=Basic />\n").contains("aspas"));
    assert!(err("<Echo file=\"A\" nope=\"1\" />\n").contains("atributo desconhecido `nope`"));
    assert!(err("<Echo />\n").contains("atributo obrigatório `file`"));
}

#[test]
fn invalid_front_matter_is_an_error() {
    let e = err("---\ntitle: [x\n---\n# T\n");
    assert!(e.starts_with("content/teste.md:"), "{e}");
}

#[test]
fn two_tags_in_one_block_is_an_error() {
    assert!(err("<Echo file=\"A\" />\n<Echo file=\"B\" />\n").contains("mais de um elemento"));
}

// ── Site ────────────────────────────────────────────────────────────────────

fn temp_site(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    for (rel, text) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

/// `name` = pasta temporária própria de cada teste (os testes rodam em paralelo).
fn two_section_site(name: &str) -> PathBuf {
    temp_site(
        name,
        &[
            (
                "content/index.md",
                "---\nlayout: home\ntitle: Início\nhero:\n  name: Meu site\n  tagline: Um site\n  actions:\n    - text: Começar\n      link: /guide/start\nfeatures:\n  - title: Rápido\n    details: Muito\n---\n",
            ),
            (
                "content/guide/start.md",
                "---\ntitle: Começando\ndescription: Como começar\norder: 1\n---\n\n# Começando\n\nVeja [a API](/components/btn).\n",
            ),
            (
                "content/guide/theming.md",
                "---\ntitle: Temas\ndescription: Tokens\norder: 2\n---\n\n# Temas\n\n<Echo file=\"Basic\" />\n",
            ),
            (
                "content/components/btn.md",
                "---\ntitle: Btn\ndescription: Botão\n---\n\n# Btn\n\n## Props\n",
            ),
        ],
    )
}

fn site(root: &Path) -> Site {
    Site::new("content")
        .root(root)
        .title("leptos-nebula")
        .description("Componentes Leptos")
        .group("guide", "Guia")
        .group("components", "Componentes")
        .nav([("Guia", "/guide/"), ("Componentes", "/components/")])
        .sidebar_from_folders()
        .component(Echo)
}

#[test]
fn press_rs_of_a_two_section_site() {
    let generated = site(&two_section_site("two-sections"))
        .generate()
        .unwrap_or_else(|e| panic!("{e}"));
    expect(&fixtures_dir().join("site.press.rs"), &generated.rust);
}

#[test]
fn broken_internal_link_is_an_error() {
    let root = temp_site(
        "broken-link",
        &[("content/a.md", "# A\n\n[x](/nao-existe)\n")],
    );
    let e = Site::new("content")
        .root(&root)
        .generate()
        .err()
        .unwrap()
        .to_string();
    assert!(
        e.ends_with("a.md:3: link interno quebrado: /nao-existe não é uma página do site"),
        "{e}"
    );
}

#[test]
fn nav_link_must_resolve() {
    let root = temp_site("bad-nav", &[("content/a.md", "# A\n")]);
    let e = Site::new("content")
        .root(&root)
        .nav([("X", "/nada/")])
        .generate()
        .err()
        .unwrap()
        .to_string();
    assert!(e.contains("não resolve para nenhuma página"), "{e}");
}

#[test]
fn duplicate_routes_are_an_error() {
    let root = temp_site(
        "dup-route",
        &[("content/a/index.md", "# A\n"), ("content/a.md", "# A\n")],
    );
    let e = Site::new("content")
        .root(&root)
        .generate()
        .err()
        .unwrap()
        .to_string();
    assert!(e.contains("a rota /a é gerada por"), "{e}");
}

// ── Plugins ─────────────────────────────────────────────────────────────────

/// Emite `path` com um texto fixo; `check_page` falha nas páginas com `fail: true`.
struct Emit(&'static str, &'static str);

impl Plugin for Emit {
    fn name(&self) -> &'static str {
        self.0
    }
    fn check_page(&self, page: &PageData) -> Result<(), Error> {
        if page.front.extra.contains_key("fail") {
            return Err(Error::new("página recusada"));
        }
        Ok(())
    }
    fn emit(&self, site: &SiteData, out: &mut Output) -> Result<(), Error> {
        out.add(self.1, format!("{} páginas", site.pages.len()))
    }
}

#[test]
fn emitted_file_is_a_valid_link_target() {
    let root = temp_site(
        "plugin-link",
        &[("content/a.md", "# A\n\n[x](/extra/dados.txt)\n")],
    );
    let base = || Site::new("content").root(&root);
    let e = base().generate().err().unwrap().to_string();
    assert!(e.contains("link interno quebrado: /extra/dados.txt"), "{e}");
    let generated = base()
        .plugin(Emit("dados", "extra/dados.txt"))
        .generate()
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(generated.files["extra/dados.txt"], "1 páginas");
    // O roteador do cliente deixa o navegador carregar o arquivo (sem o 404 do SPA).
    assert!(
        generated
            .rust
            .contains(r#"<a href=\"/extra/dados.txt\" rel=\"external\">"#),
        "{}",
        generated.rust
    );
}

#[test]
fn page_links_get_no_rel_external() {
    let generated = site(&two_section_site("page-links"))
        .generate()
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(generated.rust.contains(r#"<a href=\"/components/btn\">"#));
    assert!(!generated.rust.contains("rel=\\\"external"));
}

#[test]
fn no_plugins_no_static_files() {
    let root = temp_site("no-plugins", &[("content/a.md", "# A\n")]);
    let generated = Site::new("content").root(&root).generate().unwrap();
    assert!(generated.files.is_empty(), "{:?}", generated.files.keys());
}

#[test]
fn same_path_from_two_plugins_is_an_error() {
    let root = temp_site("plugin-dup", &[("content/a.md", "# A\n")]);
    let e = Site::new("content")
        .root(&root)
        .plugin(Emit("um", "x.txt"))
        .plugin(Emit("outro", "x.txt"))
        .generate()
        .err()
        .unwrap()
        .to_string();
    assert_eq!(
        e,
        "plugin `outro`: `x.txt` emitido por dois plugins: `um` e `outro`"
    );
}

#[test]
fn check_page_error_points_to_the_file() {
    let root = temp_site(
        "plugin-check",
        &[
            ("content/a.md", "# A\n"),
            ("content/b.md", "---\nfail: true\n---\n# B\n"),
        ],
    );
    let e = Site::new("content")
        .root(&root)
        .plugin(Emit("revisor", "x.txt"))
        .generate()
        .err()
        .unwrap()
        .to_string();
    assert!(
        e.ends_with("content/b.md:1: plugin `revisor`: página recusada"),
        "{e}"
    );
}

#[test]
fn site_data_order_and_groups() {
    struct Spy;
    impl Plugin for Spy {
        fn name(&self) -> &'static str {
            "spy"
        }
        fn emit(&self, site: &SiteData, out: &mut Output) -> Result<(), Error> {
            let routes: Vec<&str> = site.pages.iter().map(|p| p.route.as_str()).collect();
            let groups: Vec<String> = site
                .groups
                .iter()
                .map(|g| format!("{}={}:{}", g.folder, g.text, g.routes.join(",")))
                .collect();
            out.add(
                "spy.txt",
                format!("{}\n{}", routes.join(" "), groups.join(" ")),
            )
        }
    }
    let generated = site(&two_section_site("site-data"))
        .plugin(Spy)
        .generate()
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        generated.files["spy.txt"],
        "/guide/start /guide/theming /components/btn /\n\
         guide=Guia:/guide/start,/guide/theming components=Componentes:/components/btn"
    );
}
