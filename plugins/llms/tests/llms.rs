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
        "# leptos-nebula\n\n> Componentes Leptos\n\n## Componentes\n\n- [Btn](/components/btn.md): Botão\n\n## Optional\n\n- [Começando](/guide/start.md): Como começar\n- [Temas](/guide/theming.md): Tokens\n"
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
    assert!(files["llms.txt"].starts_with("# leptos-nebula\n\n> Outro resumo\n\n## Guia"));
    let files = self::files("summary-empty", Llms::new().summary(""));
    assert!(files["llms.txt"].starts_with("# leptos-nebula\n\n## Guia"));
}
