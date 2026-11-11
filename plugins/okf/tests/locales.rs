//! Multilingual sites: a synthetic `SiteData` with two locales (`en`, the default, and `pt`),
//! built with `SiteData::builder` (the core does not discover locales yet).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use leptos_press_build::{
    Error, FrontMatter, GroupData, Layout, Output, PageData, Plugin, SiteData,
};
use leptos_press_okf::{Okf, check_bundle};

fn home(route: &str, file: &str, locale: &str, title: &str, markdown: &str) -> PageData {
    PageData::new(route, file)
        .with_locale(locale)
        .with_title(title)
        .with_front(FrontMatter {
            layout: Layout::Home,
            ..FrontMatter::default()
        })
        .with_markdown(markdown)
}

fn page(route: &str, file: &str, locale: &str, title: &str, markdown: &str) -> PageData {
    PageData::new(route, file)
        .with_locale(locale)
        .with_title(title)
        .with_description(format!("About {title}"))
        .with_markdown(markdown)
}

/// `en` (default, `/en`) and `pt` (`/pt`), with links inside each locale and across them.
fn site() -> SiteData {
    SiteData::builder("leptos-nebula")
        .description("Leptos components")
        .locale("en", "English", "/en")
        .locale("pt", "Português", "/pt")
        .page(page(
            "/en/guide/start",
            "content/en/guide/start.md",
            "en",
            "Start",
            "# Start\n\nSee [Btn](/en/components/btn#props), [home](/en), \
             [theming](./theming.md#tokens) and [em português](/pt/guide/start).\n",
        ))
        .page(page(
            "/en/guide/theming",
            "content/en/guide/theming.md",
            "en",
            "Theming",
            "# Theming\n\n## Tokens\n",
        ))
        .page(page(
            "/en/components/btn",
            "content/en/components/btn.md",
            "en",
            "Btn",
            "# Btn\n\n## Props\n",
        ))
        .page(home(
            "/en",
            "content/en/index.md",
            "en",
            "Home",
            "Start with the [guide](/en/guide/start).\n",
        ))
        .page(page(
            "/pt/guide/start",
            "content/pt/guide/start.md",
            "pt",
            "Começando",
            "# Começando\n\nVeja o [Btn](/pt/components/btn) e [in English](/en/guide/start#x).\n",
        ))
        .page(page(
            "/pt/components/btn",
            "content/pt/components/btn.md",
            "pt",
            "Btn",
            "# Btn\n",
        ))
        .page(home(
            "/pt",
            "content/pt/index.md",
            "pt",
            "Início",
            "Comece pelo [guia](/pt/guide/start).\n",
        ))
        .group(GroupData::new(
            "en",
            "guide",
            "Guide",
            ["/en/guide/start", "/en/guide/theming"],
        ))
        .group(GroupData::new(
            "en",
            "components",
            "Components",
            ["/en/components/btn"],
        ))
        .group(GroupData::new("pt", "guide", "Guia", ["/pt/guide/start"]))
        .group(GroupData::new(
            "pt",
            "components",
            "Componentes",
            ["/pt/components/btn"],
        ))
        .build()
        .unwrap_or_else(|e| panic!("{e}"))
}

/// As the core runs a plugin: `check_page` on every page, then `emit`.
fn emit(site: &SiteData, okf: Okf) -> Result<BTreeMap<String, String>, Error> {
    for page in &site.pages {
        okf.check_page(page)?;
    }
    let mut out = Output::default();
    okf.emit(site, &mut out)?;
    Ok(out
        .iter()
        .map(|(p, c)| (p.to_string(), c.to_string()))
        .collect())
}

fn bundle(okf: Okf) -> BTreeMap<String, String> {
    emit(&site(), okf).unwrap_or_else(|e| panic!("{e}"))
}

fn configured() -> Okf {
    Okf::new().group_type("components", "Leptos Component")
}

fn names(files: &BTreeMap<String, String>) -> Vec<&str> {
    files.keys().map(String::as_str).collect()
}

fn front(text: &str) -> &str {
    let rest = text.strip_prefix("---\n").expect("front-matter");
    &rest[..rest.find("\n---\n").expect("end of front-matter")]
}

/// Writes the bundle to disk and runs `check_bundle` on it.
fn check_on_disk(name: &str, files: &BTreeMap<String, String>) {
    let dir: PathBuf = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("okf-locales")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    assert_eq!(check_bundle(&dir.join("okf")), Ok(()));
}

#[test]
fn one_selected_locale_has_ids_without_the_prefix() {
    let files = bundle(configured().locales(["pt"]));
    assert_eq!(
        names(&files),
        [
            "okf/components/btn.md",
            "okf/components/index.md",
            "okf/guide/index.md",
            "okf/guide/start.md",
            "okf/index.md",
            "okf/overview.md",
        ]
    );
    let start = &files["okf/guide/start.md"];
    assert!(start.contains("[Btn](/components/btn.md)"), "{start}");
    // A page of a locale left out: its site URL.
    assert!(start.contains("[in English](/en/guide/start#x)"), "{start}");
    assert!(files["okf/overview.md"].contains("[guia](/guide/start.md)"));
    assert_eq!(
        files["okf/index.md"],
        "---\nokf_version: \"0.2\"\n---\n\n# leptos-nebula\n\nLeptos components\n\n\
         * [Início](/overview.md)\n\n\
         ## Guia\n\n* [Começando](/guide/start.md) - About Começando\n\n\
         ## Componentes\n\n* [Btn](/components/btn.md) - About Btn\n"
    );
    // `group_type` folders are relative to the locale root.
    assert!(front(&files["okf/components/btn.md"]).starts_with("type: Leptos Component\n"));
    check_on_disk("one", &files);
}

#[test]
fn links_to_left_out_locales_are_absolute_with_base_url() {
    let files = bundle(
        configured()
            .locales(["en"])
            .base_url("https://example.dev/docs"),
    );
    let start = &files["okf/guide/start.md"];
    assert!(
        start.contains("[em português](https://example.dev/docs/pt/guide/start)"),
        "{start}"
    );
    assert!(start.contains("[Btn](/components/btn.md#props)"), "{start}");
    assert!(start.contains("[home](/overview.md)"), "{start}");
    assert!(
        start.contains("[theming](/guide/theming.md#tokens)"),
        "{start}"
    );
    // `resource` is the page's real URL, with the locale prefix.
    assert!(
        front(start)
            .lines()
            .any(|l| l == "resource: https://example.dev/docs/en/guide/start"),
        "{start}"
    );
    assert!(
        front(&files["okf/overview.md"])
            .lines()
            .any(|l| l == "resource: https://example.dev/docs/en")
    );
}

#[test]
fn several_locales_get_one_directory_each() {
    let files = bundle(configured());
    assert_eq!(
        names(&files),
        [
            "okf/en/components/btn.md",
            "okf/en/components/index.md",
            "okf/en/guide/index.md",
            "okf/en/guide/start.md",
            "okf/en/guide/theming.md",
            "okf/en/index.md",
            "okf/en/overview.md",
            "okf/index.md",
            "okf/pt/components/btn.md",
            "okf/pt/components/index.md",
            "okf/pt/guide/index.md",
            "okf/pt/guide/start.md",
            "okf/pt/index.md",
            "okf/pt/overview.md",
        ]
    );
    assert_eq!(
        files["okf/index.md"],
        "---\nokf_version: \"0.2\"\n---\n\n# leptos-nebula\n\nLeptos components\n\n\
         * [English](/en/index.md) - en\n\
         * [Português](/pt/index.md) - pt\n"
    );
    assert_eq!(
        files["okf/pt/index.md"],
        "# leptos-nebula (Português)\n\nLeptos components\n\n\
         * [Início](/pt/overview.md)\n\n\
         ## Guia\n\n* [Começando](/pt/guide/start.md) - About Começando\n\n\
         ## Componentes\n\n* [Btn](/pt/components/btn.md) - About Btn\n"
    );
    assert_eq!(
        files["okf/en/guide/index.md"],
        "# Guide\n\n* [Start](/en/guide/start.md) - About Start\n\
         * [Theming](/en/guide/theming.md) - About Theming\n"
    );
    let start = &files["okf/en/guide/start.md"];
    for link in [
        "[Btn](/en/components/btn.md#props)",
        "[home](/en/overview.md)",
        "[theming](/en/guide/theming.md#tokens)",
        "[em português](/pt/guide/start.md)",
    ] {
        assert!(start.contains(link), "missing {link:?} in:\n{start}");
    }
    assert!(files["okf/pt/guide/start.md"].contains("[in English](/en/guide/start.md#x)"));
    assert!(front(&files["okf/pt/components/btn.md"]).starts_with("type: Leptos Component\n"));
    check_on_disk("two", &files);

    // Selecting both explicitly is the same bundle.
    assert_eq!(bundle(configured().locales(["pt", "en"])), files);
}

#[test]
fn unknown_or_empty_locales_are_an_error() {
    let e = emit(&site(), Okf::new().locales(["en", "de"]))
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("o site não tem o idioma `de` (idiomas do site: `en`, `pt`)"),
        "{e}"
    );
    let e = emit(&site(), Okf::new().locales(Vec::<String>::new()))
        .unwrap_err()
        .to_string();
    assert!(e.contains("nenhum idioma selecionado"), "{e}");
}

#[test]
fn pages_of_left_out_locales_are_not_checked() {
    let bad = PageData::new("/pt/x", "content/pt/x.md")
        .with_locale("pt")
        .with_title("X")
        .with_front(FrontMatter {
            extra: [("okf".to_string(), "draft".into())].into_iter().collect(),
            ..FrontMatter::default()
        });
    let site = SiteData::builder("t")
        .locale("en", "English", "/en")
        .locale("pt", "Português", "/pt")
        .page(
            PageData::new("/en/a", "content/en/a.md")
                .with_locale("en")
                .with_title("A"),
        )
        .page(bad)
        .build()
        .unwrap();
    assert!(emit(&site, Okf::new().locales(["en"])).is_ok());
    assert!(emit(&site, Okf::new()).is_err());
}

#[test]
fn overview_collision_is_per_locale() {
    let site = SiteData::builder("t")
        .locale("en", "English", "/en")
        .locale("pt", "Português", "/pt")
        .page(home("/en", "content/en/index.md", "en", "Home", ""))
        .page(PageData::new("/pt/overview", "content/pt/overview.md").with_locale("pt"))
        .page(home("/pt", "content/pt/index.md", "pt", "Início", ""))
        .build()
        .unwrap();
    assert!(emit(&site, Okf::new().locales(["en"])).is_ok());
    let e = emit(&site, Okf::new()).unwrap_err().to_string();
    assert!(e.contains("`okf/pt/overview.md`"), "{e}");
}
