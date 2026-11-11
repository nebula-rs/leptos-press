//! Site de fixture → `llms.txt` / `llms-full.txt` esperados (snapshots) e opções do builder.
//! `UPDATE_EXPECT=1 cargo test -p leptos-press-llms` regrava os arquivos esperados.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use leptos_press_build::{Attrs, Ctx, Error, MdComponent, Site};
use leptos_press_llms::Llms;
use proc_macro2::TokenStream;
use quote::quote;

/// Tag de teste: a saída Markdown cita o arquivo.
struct Echo;

impl MdComponent for Echo {
    fn tag(&self) -> &'static str {
        "Echo"
    }
    fn codegen(&self, attrs: &Attrs, _ctx: &Ctx) -> Result<TokenStream, Error> {
        let file = attrs.require("file")?;
        Ok(quote! { echo(#file) })
    }
    fn markdown(&self, attrs: &Attrs, _ctx: &Ctx) -> Result<String, Error> {
        Ok(format!(
            "```rust\n// exemplo {}\n```",
            attrs.require("file")?
        ))
    }
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
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

/// `name` = pasta temporária própria de cada teste (os testes rodam em paralelo).
fn two_section_site(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    let files = [
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
    ];
    for (rel, text) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

fn generate(name: &str, llms: Llms) -> Result<BTreeMap<String, String>, Error> {
    Site::new("content")
        .root(two_section_site(name))
        .title("leptos-nebula")
        .description("Componentes Leptos")
        .group("guide", "Guia")
        .group("components", "Componentes")
        .nav([("Guia", "/guide/"), ("Componentes", "/components/")])
        .sidebar_from_folders()
        .component(Echo)
        .plugin(llms)
        .generate()
        .map(|g| g.files)
}

fn files(name: &str, llms: Llms) -> BTreeMap<String, String> {
    generate(name, llms).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn default_output() {
    let files = files("default", Llms::default());
    let dir = fixtures_dir();
    expect(&dir.join("site.llms.txt"), &files["llms.txt"]);
    expect(&dir.join("site.llms-full.txt"), &files["llms-full.txt"]);
    let names: Vec<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "components/btn.md",
            "guide/start.md",
            "guide/theming.md",
            "index.md",
            "llms-full.txt",
            "llms.txt",
        ]
    );
    assert!(files["guide/theming.md"].contains("// exemplo Basic"));
}

#[test]
fn without_full() {
    let files = files("no-full", Llms::new().full(false));
    assert!(!files.contains_key("llms-full.txt"));
    assert!(files.contains_key("llms.txt") && files.contains_key("index.md"));
}

#[test]
fn without_page_copies_links_go_to_routes() {
    let files = files("no-copies", Llms::new().page_copies(false));
    let names: Vec<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(names, ["llms-full.txt", "llms.txt"]);
    assert!(
        files["llms.txt"].contains("- [Btn](/components/btn): Botão"),
        "{}",
        files["llms.txt"]
    );
    assert!(files["llms-full.txt"].starts_with("---\nurl: /\n---"));
    assert!(files["llms-full.txt"].contains("url: /guide/start\n"));
}

#[test]
fn excluded_prefixes_leave_every_file() {
    let files = files("exclude", Llms::new().exclude("/guide/").exclude("/"));
    let names: Vec<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(names, ["components/btn.md", "llms-full.txt", "llms.txt"]);
    assert!(
        !files["llms.txt"].contains("## Guia"),
        "{}",
        files["llms.txt"]
    );
    assert!(!files["llms-full.txt"].contains("Começando"));
    assert!(files["llms-full.txt"].starts_with("---\nurl: /components/btn.md\n"));
}

#[test]
fn exclude_is_by_path_segment() {
    // `/guide/sta` não é prefixo de segmento de `/guide/start`.
    let files = files("exclude-segment", Llms::new().exclude("/guide/sta"));
    assert!(files.contains_key("guide/start.md"));
}

#[test]
fn optional_group_goes_last() {
    let files = files("optional", Llms::new().optional_group("guide"));
    assert_eq!(
        files["llms.txt"],
        "# leptos-nebula\n\n> Componentes Leptos\n\nFull documentation in a single file: [llms-full.txt](/llms-full.txt)\n\n## Componentes\n\n- [Btn](/components/btn.md): Botão\n\n## Optional\n\n- [Começando](/guide/start.md): Como começar\n- [Temas](/guide/theming.md): Tokens\n"
    );
}

#[test]
fn unknown_optional_group_is_an_error() {
    let e = generate("optional-bad", Llms::new().optional_group("nada"))
        .err()
        .unwrap()
        .to_string();
    assert!(
        e.contains("plugin `llms`: `optional_group(\"nada\")`"),
        "{e}"
    );
}

#[test]
fn summary_replaces_the_description() {
    let files = files("summary", Llms::new().summary("Outro resumo"));
    assert!(
        files["llms.txt"].starts_with("# leptos-nebula\n\n> Outro resumo\n\nFull documentation")
    );
    let files = self::files("summary-empty", Llms::new().summary(""));
    assert!(files["llms.txt"].starts_with("# leptos-nebula\n\nFull documentation"));
}

/// Contrato do `llms-full.txt` (doc comment do crate; lido pela busca do `leptos-press`):
/// separador `---` / `url: …` / `---` no início de linha, home primeiro e depois a ordem do
/// site, `url` da cópia `.md` (ou da rota sem cópias), uma linha em branco entre as partes.
#[test]
fn full_format_contract() {
    let full = &files("contract", Llms::default())["llms-full.txt"];
    let expected = [
        ("/index.md", "# Início"),
        (
            "/guide/start.md",
            "# Começando\n\nVeja [a API](/components/btn).",
        ),
        (
            "/guide/theming.md",
            "# Temas\n\n```rust\n// exemplo Basic\n```",
        ),
        ("/components/btn.md", "# Btn\n\n## Props"),
    ];
    let built = expected
        .iter()
        .map(|(url, md)| format!("---\nurl: {url}\n---\n\n{md}"))
        .collect::<Vec<_>>()
        .join("\n\n")
        + "\n";
    assert_eq!(full, &built);

    // Sem cópias: a `url` é a rota (home = `/`), na mesma ordem.
    let full = &files("contract-routes", Llms::new().page_copies(false))["llms-full.txt"];
    let urls: Vec<&str> = full
        .lines()
        .filter_map(|l| l.strip_prefix("url: "))
        .collect();
    assert_eq!(
        urls,
        ["/", "/guide/start", "/guide/theming", "/components/btn"]
    );
    assert!(full.starts_with("---\nurl: /\n---\n\n# Início\n\n---\n"));
    assert!(full.ends_with("## Props\n"));
}

/// Uma página com `index.md` de pasta sai como `/pasta/index.md`; um `---` dentro de um bloco
/// de código fica no texto da página (o leitor ignora blocos de código).
#[test]
fn full_format_folder_index_and_fenced_separator() {
    let root = two_section_site("contract-index");
    std::fs::write(
        root.join("content/guide/index.md"),
        "---\ntitle: Guia\n---\n\n# Guia\n\n```text\n---\nurl: /falso.md\n---\n```\n",
    )
    .unwrap();
    let files = Site::new("content")
        .root(&root)
        .title("t")
        .sidebar_from_folders()
        .component(Echo)
        .plugin(Llms::default())
        .generate()
        .unwrap_or_else(|e| panic!("{e}"))
        .files;
    let full = &files["llms-full.txt"];
    assert!(
        full.contains(
            "---\nurl: /guide/index.md\n---\n\n# Guia\n\n```text\n---\nurl: /falso.md\n---\n```"
        ),
        "{full}"
    );
    let routes = Site::new("content")
        .root(&root)
        .title("t")
        .sidebar_from_folders()
        .component(Echo)
        .plugin(Llms::new().page_copies(false))
        .generate()
        .unwrap_or_else(|e| panic!("{e}"))
        .files;
    assert!(routes["llms-full.txt"].contains("---\nurl: /guide\n---\n\n# Guia"));
}
