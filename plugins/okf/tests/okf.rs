//! Site de fixture → árvore OKF esperada (snapshot), regras de `type`, mescla do `okf:`,
//! reescrita de links e `check_bundle`.
//! `UPDATE_EXPECT=1 cargo test -p leptos-press-okf` regrava os arquivos esperados.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use leptos_press_build::{Attrs, Ctx, Error, MdComponent, Site};
use leptos_press_okf::{Okf, check_bundle};
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

fn temp_dir(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("okf")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    root
}

fn write(root: &Path, files: &[(&str, &str)]) {
    for (rel, text) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

const HOME: (&str, &str) = (
    "content/index.md",
    "---\nlayout: home\ntitle: Início\ndescription: Página inicial\nhero:\n  name: Meu site\n---\n\nComece pelo [guia](/guide/start).\n",
);

const START: (&str, &str) = (
    "content/guide/start.md",
    "---\ntitle: Começando\ndescription: Como começar\norder: 1\ntags: [intro, setup]\n---\n\n# Começando\n\nVeja [a API](/components/btn), as [props](/components/btn#props), a [home](/),\nos [tokens](./theming.md#tokens), o [site](https://example.com) e os [passos](#passos).\n\n## Passos\n\n```md\n[não mexe](/components/btn)\n```\n\nTexto com `[código](/components/btn)` inline.\n",
);

fn site_files() -> Vec<(&'static str, &'static str)> {
    vec![
        HOME,
        START,
        (
            "content/guide/theming.md",
            "---\ntitle: Temas\ndescription: Tokens\norder: 2\ntype: Tutorial\n---\n\n# Temas\n\n## Tokens\n\n<Echo file=\"Basic\" />\n",
        ),
        (
            "content/components/btn.md",
            "---\ntitle: Btn\ndescription: Botão\nokf:\n  title: Btn (QBtn)\n  status: draft\n  sources:\n    - resource: https://quasar.dev/vue-components/button\n---\n\n# Btn\n\n## Props\n\nDe volta ao [guia](../guide/start.md).\n",
        ),
        (
            "content/components/extra/deep.md",
            "---\ntitle: Fundo\n---\n\n# Fundo\n",
        ),
        (
            "content/about.md",
            "# Sobre\n\nUma página fora da sidebar.\n",
        ),
    ]
}

fn site(root: &Path, okf: Okf) -> Site {
    Site::new("content")
        .root(root)
        .title("leptos-nebula")
        .description("Componentes Leptos")
        .group("guide", "Guia")
        .group("components", "Componentes")
        .sidebar_from_folders()
        .component(Echo)
        .plugin(okf)
}

fn configured() -> Okf {
    Okf::new()
        .group_type("components", "Leptos Component")
        .group_type("guide", "Guide")
}

fn generate(
    name: &str,
    files: &[(&str, &str)],
    okf: Okf,
) -> Result<BTreeMap<String, String>, Error> {
    let root = temp_dir(name);
    write(&root, files);
    site(&root, okf).generate().map(|g| g.files)
}

fn bundle(name: &str, okf: Okf) -> BTreeMap<String, String> {
    generate(name, &site_files(), okf).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn site_to_okf_tree() {
    let files = bundle("tree", configured());
    let snapshot: String = files
        .iter()
        .map(|(path, text)| format!("=== {path} ===\n{text}"))
        .collect();
    expect(&fixtures_dir().join("site.okf.txt"), &snapshot);
    assert!(
        files.keys().all(|k| k.starts_with("okf/")),
        "{:?}",
        files.keys()
    );
}

fn front(text: &str) -> &str {
    let rest = text.strip_prefix("---\n").expect("front-matter");
    &rest[..rest.find("\n---\n").expect("fim do front-matter")]
}

#[test]
fn type_comes_from_page_then_group_then_default() {
    let files = bundle("types", configured());
    assert!(front(&files["okf/guide/theming.md"]).starts_with("type: Tutorial\n"));
    assert!(front(&files["okf/guide/start.md"]).starts_with("type: Guide\n"));
    assert!(front(&files["okf/components/extra/deep.md"]).starts_with("type: Leptos Component\n"));
    assert!(front(&files["okf/about.md"]).starts_with("type: Documentation Page\n"));
    assert!(front(&files["okf/overview.md"]).starts_with("type: Documentation Page\n"));

    let files = bundle("types-default", Okf::new().default_type("Page"));
    assert!(front(&files["okf/guide/start.md"]).starts_with("type: Page\n"));
}

#[test]
fn okf_map_is_merged_over_derived_keys() {
    let files = bundle("merge", configured());
    assert_eq!(
        front(&files["okf/components/btn.md"]),
        "type: Leptos Component\ntitle: Btn (QBtn)\ndescription: Botão\nstatus: draft\nsources:\n- resource: https://quasar.dev/vue-components/button"
    );
    assert_eq!(
        front(&files["okf/guide/start.md"]),
        "type: Guide\ntitle: Começando\ndescription: Como começar\ntags:\n- intro\n- setup"
    );
}

#[test]
fn okf_map_errors_point_to_the_page() {
    for (case, okf) in [
        ("type vazio", "okf:\n  type: ''"),
        ("não é mapa", "okf: draft"),
    ] {
        let page = format!("---\ntitle: X\n{okf}\n---\n# X\n");
        let e = generate(
            "merge-bad",
            &[("content/guide/x.md", page.as_str())],
            Okf::new(),
        )
        .err()
        .unwrap_or_else(|| panic!("{case}: deveria falhar"))
        .to_string();
        assert!(
            e.contains("content/guide/x.md:1: plugin `okf`: `okf"),
            "{case}: {e}"
        );
    }
}

#[test]
fn links_are_rewritten_to_bundle_paths() {
    let files = bundle("links", configured());
    let start = &files["okf/guide/start.md"];
    for link in [
        "[a API](/components/btn.md)",
        "[props](/components/btn.md#props)",
        "[home](/overview.md)",
        "[tokens](/guide/theming.md#tokens)",
        "[site](https://example.com)",
        "[passos](#passos)",
        "[não mexe](/components/btn)\n```",
        "`[código](/components/btn)`",
    ] {
        assert!(start.contains(link), "falta {link:?} em:\n{start}");
    }
    assert!(files["okf/components/btn.md"].contains("[guia](/guide/start.md)"));
    assert!(files["okf/overview.md"].contains("[guia](/guide/start.md)"));
}

#[test]
fn resource_only_with_base_url() {
    let files = bundle(
        "resource",
        configured().base_url("https://exemplo.dev/docs/"),
    );
    assert!(
        front(&files["okf/guide/start.md"])
            .contains("resource: https://exemplo.dev/docs/guide/start\n"),
        "{}",
        files["okf/guide/start.md"]
    );
    assert!(
        front(&files["okf/overview.md"])
            .lines()
            .any(|l| l == "resource: https://exemplo.dev/docs/")
    );
    let files = bundle("no-resource", configured());
    assert!(!files["okf/guide/start.md"].contains("resource:"));
}

#[test]
fn reserved_names_are_an_error() {
    let e = generate(
        "reserved",
        &[("content/guide/log.md", "# Log\n")],
        Okf::new(),
    )
    .err()
    .unwrap()
    .to_string();
    assert!(
        e.contains("content/guide/log.md:1: plugin `okf`: a rota /guide/log vira `okf/guide/log.md`, nome reservado"),
        "{e}"
    );
    let e = generate(
        "overview",
        &[
            HOME,
            ("content/overview.md", "# Visão geral\n"),
            START_TARGETS,
        ],
        Okf::new(),
    )
    .err()
    .unwrap()
    .to_string();
    assert!(e.contains("okf/overview.md"), "{e}");
}

/// Páginas que os links da home precisam (o link quebrado falharia antes do teste).
const START_TARGETS: (&str, &str) = ("content/guide/start.md", "# Começando\n");

#[test]
fn check_bundle_accepts_the_generated_bundle() {
    let files = bundle("check-ok", configured());
    let dir = temp_dir("check-ok-out");
    for (path, text) in &files {
        write(&dir, &[(path.as_str(), text.as_str())]);
    }
    assert_eq!(check_bundle(&dir.join("okf")), Ok(()));
}

#[test]
fn check_bundle_rejects_invalid_bundles() {
    let dir = temp_dir("check-bad");
    write(
        &dir,
        &[
            (
                "index.md",
                "---\nokf_version: \"0.2\"\ntitle: X\n---\n\n# X\n",
            ),
            ("a.md", "---\ntitle: A\n---\n"),
            ("b.md", "---\ntype: B\n---\n"),
            ("sub/index.md", "---\nokf_version: \"0.2\"\n---\n# Sub\n"),
            ("sub/c.md", "# C\n"),
        ],
    );
    let errors = check_bundle(&dir).unwrap_err();
    assert_eq!(errors.len(), 4, "{errors:?}");
    assert!(
        errors[0].starts_with("a.md: front-matter sem `type`"),
        "{errors:?}"
    );
    assert!(
        errors[1].starts_with(
            "index.md: o front-matter do `index.md` da raiz só pode ter `okf_version`"
        )
    );
    assert!(errors[2].starts_with("sub/c.md: conceito sem front-matter"));
    assert!(errors[3].starts_with("sub/index.md: `index.md` fora da raiz"));
}
