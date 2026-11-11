//! Multilingual sites: a synthetic `SiteData` with two locales (`en`, the default, and `pt`),
//! built with `SiteData::builder` (the core does not discover locales yet).

use std::collections::BTreeMap;

use leptos_press_build::{
    Error, FrontMatter, GroupData, Layout, Output, PageData, Plugin, SiteData, SiteDataBuilder,
};
use leptos_press_llms::Llms;

fn home(route: &str, file: &str, locale: &str, title: &str) -> PageData {
    PageData::new(route, file)
        .with_locale(locale)
        .with_title(title)
        .with_front(FrontMatter {
            layout: Layout::Home,
            ..FrontMatter::default()
        })
        .with_markdown(format!("# {title}\n"))
}

fn page(route: &str, file: &str, locale: &str, title: &str, description: &str) -> PageData {
    PageData::new(route, file)
        .with_locale(locale)
        .with_title(title)
        .with_description(description)
        .with_markdown(format!("# {title}\n\nText.\n"))
}

/// `en` (default) and `pt`, with prefixes `en_prefix` and `/pt`. Pages in sidebar order, then
/// the home (as the core orders them).
fn two_locales(en_prefix: &str) -> SiteDataBuilder {
    let en = |r: &str| format!("{en_prefix}{r}");
    let en_home = if en_prefix.is_empty() {
        "/".to_string()
    } else {
        en_prefix.to_string()
    };
    SiteData::builder("leptos-nebula")
        .description("Leptos components")
        .locale("en", "English", en_prefix)
        .locale("pt", "Português", "/pt")
        .page(page(
            &en("/guide/start"),
            "content/en/guide/start.md",
            "en",
            "Getting started",
            "How to start",
        ))
        .page(page(
            &en("/guide/theming"),
            "content/en/guide/theming.md",
            "en",
            "Theming",
            "Tokens",
        ))
        .page(page(
            &en("/components/btn"),
            "content/en/components/btn.md",
            "en",
            "Btn",
            "Button",
        ))
        .page(home(&en_home, "content/en/index.md", "en", "Home"))
        .page(page(
            "/pt/guide/start",
            "content/pt/guide/start.md",
            "pt",
            "Começando",
            "Como começar",
        ))
        .page(page(
            "/pt/components/btn",
            "content/pt/components/btn.md",
            "pt",
            "Btn",
            "Botão",
        ))
        .page(home("/pt", "content/pt/index.md", "pt", "Início"))
        .group(GroupData::new(
            "en",
            "guide",
            "Guide",
            [en("/guide/start"), en("/guide/theming")],
        ))
        .group(GroupData::new(
            "en",
            "components",
            "Components",
            [en("/components/btn")],
        ))
        .group(GroupData::new("pt", "guide", "Guia", ["/pt/guide/start"]))
        .group(GroupData::new(
            "pt",
            "components",
            "Componentes",
            ["/pt/components/btn"],
        ))
}

fn emit(site: &SiteData, llms: Llms) -> Result<BTreeMap<String, String>, Error> {
    let mut out = Output::default();
    llms.emit(site, &mut out)?;
    Ok(out
        .iter()
        .map(|(p, c)| (p.to_string(), c.to_string()))
        .collect())
}

fn files(site: &SiteData, llms: Llms) -> BTreeMap<String, String> {
    emit(site, llms).unwrap_or_else(|e| panic!("{e}"))
}

fn site(en_prefix: &str) -> SiteData {
    two_locales(en_prefix)
        .build()
        .unwrap_or_else(|e| panic!("{e}"))
}

fn names(files: &BTreeMap<String, String>) -> Vec<&str> {
    files.keys().map(String::as_str).collect()
}

#[test]
fn every_locale_gets_its_files_under_its_prefix() {
    let files = files(&site("/en"), Llms::default());
    assert_eq!(
        names(&files),
        [
            "en/components/btn.md",
            "en/guide/start.md",
            "en/guide/theming.md",
            "en/index.md",
            "en/llms-full.txt",
            "en/llms.txt",
            "llms-full.txt",
            "llms.txt",
            "pt/components/btn.md",
            "pt/guide/start.md",
            "pt/index.md",
            "pt/llms-full.txt",
            "pt/llms.txt",
        ]
    );
}

#[test]
fn each_llms_txt_lists_only_its_locale_and_links_its_full_file() {
    let files = files(&site("/en"), Llms::default());
    assert_eq!(
        files["pt/llms.txt"],
        "# leptos-nebula\n\n> Leptos components\n\n\
         Full documentation in a single file: [llms-full.txt](/pt/llms-full.txt)\n\n\
         ## Guia\n\n- [Começando](/pt/guide/start.md): Como começar\n\n\
         ## Componentes\n\n- [Btn](/pt/components/btn.md): Botão\n"
    );
    assert_eq!(
        files["en/llms.txt"],
        "# leptos-nebula\n\n> Leptos components\n\n\
         Full documentation in a single file: [llms-full.txt](/en/llms-full.txt)\n\n\
         ## Guide\n\n- [Getting started](/en/guide/start.md): How to start\n\
         - [Theming](/en/guide/theming.md): Tokens\n\n\
         ## Components\n\n- [Btn](/en/components/btn.md): Button\n"
    );
}

#[test]
fn root_llms_txt_is_the_default_locale_plus_other_languages() {
    let files = files(&site("/en"), Llms::default().optional_group("components"));
    assert_eq!(
        files["llms.txt"],
        "# leptos-nebula\n\n> Leptos components\n\n\
         Full documentation in a single file: [llms-full.txt](/llms-full.txt)\n\n\
         ## Guide\n\n- [Getting started](/en/guide/start.md): How to start\n\
         - [Theming](/en/guide/theming.md): Tokens\n\n\
         ## Other languages\n\n- [Português](/pt/llms.txt): pt\n\n\
         ## Optional\n\n- [Btn](/en/components/btn.md): Button\n"
    );
    assert_eq!(files["llms-full.txt"], files["en/llms-full.txt"]);
    assert!(!files["en/llms.txt"].contains("Other languages"));
    assert!(!files["pt/llms.txt"].contains("Other languages"));
}

#[test]
fn default_locale_at_the_root_has_a_single_root_pair() {
    let files = files(&site(""), Llms::default());
    assert_eq!(
        names(&files),
        [
            "components/btn.md",
            "guide/start.md",
            "guide/theming.md",
            "index.md",
            "llms-full.txt",
            "llms.txt",
            "pt/components/btn.md",
            "pt/guide/start.md",
            "pt/index.md",
            "pt/llms-full.txt",
            "pt/llms.txt",
        ]
    );
    assert!(files["llms.txt"].contains("[llms-full.txt](/llms-full.txt)"));
    assert!(files["llms.txt"].contains("## Other languages\n\n- [Português](/pt/llms.txt): pt\n"));
    assert!(files["llms-full.txt"].starts_with("---\nurl: /index.md\n---\n\n# Home\n"));
}

#[test]
fn llms_full_per_locale_keeps_the_contract() {
    let files = files(&site("/en"), Llms::default());
    let urls = |f: &str| -> Vec<String> {
        files[f]
            .lines()
            .filter_map(|l| l.strip_prefix("url: "))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(
        urls("pt/llms-full.txt"),
        [
            "/pt/index.md",
            "/pt/guide/start.md",
            "/pt/components/btn.md"
        ]
    );
    assert_eq!(
        urls("en/llms-full.txt"),
        [
            "/en/index.md",
            "/en/guide/start.md",
            "/en/guide/theming.md",
            "/en/components/btn.md"
        ]
    );
    assert_eq!(
        files["pt/llms-full.txt"],
        "---\nurl: /pt/index.md\n---\n\n# Início\n\n\
         ---\nurl: /pt/guide/start.md\n---\n\n# Começando\n\nText.\n\n\
         ---\nurl: /pt/components/btn.md\n---\n\n# Btn\n\nText.\n"
    );

    let files = self::files(&site("/en"), Llms::new().page_copies(false));
    assert!(files["pt/llms-full.txt"].starts_with("---\nurl: /pt\n---\n\n# Início"));
    assert!(files["pt/llms.txt"].contains("- [Btn](/pt/components/btn): Botão"));
}

#[test]
fn exclude_is_relative_to_each_locale_root() {
    let files = files(&site("/en"), Llms::new().exclude("/guide").exclude("/"));
    assert_eq!(
        names(&files),
        [
            "en/components/btn.md",
            "en/llms-full.txt",
            "en/llms.txt",
            "llms-full.txt",
            "llms.txt",
            "pt/components/btn.md",
            "pt/llms-full.txt",
            "pt/llms.txt",
        ]
    );
    assert!(!files["pt/llms.txt"].contains("## Guia"));
    assert!(!files["en/llms.txt"].contains("## Guide"));
    assert!(files["pt/llms-full.txt"].starts_with("---\nurl: /pt/components/btn.md\n"));
    // A prefix is not matched against the full route: `/pt` is not a locale-relative prefix.
    let files = self::files(&site("/en"), Llms::new().exclude("/pt"));
    assert!(files.contains_key("pt/guide/start.md"));
}

#[test]
fn optional_group_applies_to_every_locale() {
    let files = files(&site("/en"), Llms::new().optional_group("guide"));
    assert!(files["pt/llms.txt"].ends_with(
        "## Componentes\n\n- [Btn](/pt/components/btn.md): Botão\n\n\
         ## Optional\n\n- [Começando](/pt/guide/start.md): Como começar\n"
    ));
    assert!(
        files["en/llms.txt"].ends_with("## Optional\n\n- [Getting started](/en/guide/start.md): How to start\n- [Theming](/en/guide/theming.md): Tokens\n")
    );
    // A folder that is a group in some locale is accepted; in none, an error.
    let site = two_locales("/en")
        .group(GroupData::new("pt", "extra", "Extra", Vec::<String>::new()))
        .build()
        .unwrap();
    assert!(emit(&site, Llms::new().optional_group("extra")).is_ok());
    let e = emit(&site, Llms::new().optional_group("nada"))
        .unwrap_err()
        .to_string();
    assert!(e.contains("`optional_group(\"nada\")`"), "{e}");
}

#[test]
fn without_full_there_is_no_full_file_nor_link() {
    let files = files(&site("/en"), Llms::new().full(false));
    assert!(!files.keys().any(|k| k.ends_with("llms-full.txt")));
    assert!(!files.values().any(|t| t.contains("llms-full.txt")));
}

#[test]
fn a_non_default_locale_without_prefix_is_an_error() {
    let site = SiteData::builder("t")
        .locale("en", "English", "/en")
        .locale("pt", "Português", "")
        .build()
        .unwrap();
    let e = emit(&site, Llms::default()).unwrap_err().to_string();
    assert!(e.contains("o idioma `pt` não tem prefixo"), "{e}");
}
