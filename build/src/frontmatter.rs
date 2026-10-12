//! Front-matter YAML das páginas.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::error::Error;

/// Layout da página.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    #[default]
    Doc,
    Home,
}

/// Ação do hero (`layout: home`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HeroAction {
    pub text: String,
    pub link: String,
    /// `brand` (padrão para a primeira ação) ou `alt`.
    #[serde(default)]
    pub theme: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Hero {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub tagline: Option<String>,
    #[serde(default)]
    pub actions: Vec<HeroAction>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Feature {
    pub title: String,
    #[serde(default)]
    pub details: Option<String>,
    /// Emoji ou texto curto exibido acima do título.
    #[serde(default)]
    pub icon: Option<String>,
}

/// Front-matter de uma página. Chaves desconhecidas ficam em `extra` (ex.: `examples: Btn`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FrontMatter {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, alias = "desc")]
    pub description: Option<String>,
    #[serde(default)]
    pub layout: Layout,
    #[serde(default)]
    pub order: Option<i64>,
    #[serde(default)]
    pub hero: Option<Hero>,
    #[serde(default)]
    pub features: Vec<Feature>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}

impl FrontMatter {
    /// Valor textual de uma chave extra (`examples: Btn` → `Some("Btn")`).
    pub fn str(&self, key: &str) -> Option<&str> {
        self.extra.get(key).and_then(serde_yaml_ng::Value::as_str)
    }
}

/// Separa o front-matter do corpo. O corpo devolvido mantém o **mesmo número de linhas**
/// (o front-matter vira linhas em branco), para que as linhas do comrak sejam as do arquivo.
pub(crate) fn split(src: &str, file: &Path) -> Result<(FrontMatter, String), Error> {
    let first = src.lines().next().unwrap_or("");
    if first.trim_end() != "---" {
        return Ok((FrontMatter::default(), src.to_string()));
    }
    let lines: Vec<&str> = src.split('\n').collect();
    let Some(end) = lines
        .iter()
        .skip(1)
        .position(|l| l.trim_end() == "---")
        .map(|i| i + 1)
    else {
        return Err(Error::new("front-matter aberto com `---` e nunca fechado").at(file, 1));
    };
    let yaml = lines[1..end].join("\n");
    let front: FrontMatter = if yaml.trim().is_empty() {
        FrontMatter::default()
    } else {
        serde_yaml_ng::from_str(&yaml).map_err(|e| {
            // A linha do YAML é relativa ao bloco; +1 pela linha do `---` de abertura.
            let line = e.location().map_or(1, |l| l.line() + 1);
            Error::new(format!("front-matter inválido: {e}")).at(file, line)
        })?
    };
    let mut body = "\n".repeat(end + 1);
    body.push_str(&lines[end + 1..].join("\n"));
    Ok((front, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_front_matter() {
        let (f, body) = split("# Oi\n", Path::new("a.md")).unwrap();
        assert!(f.title.is_none());
        assert_eq!(body, "# Oi\n");
    }

    #[test]
    fn keeps_line_count() {
        let (f, body) = split(
            "---\ntitle: A\norder: 2\nexamples: Btn\n---\n# Oi\n",
            Path::new("a.md"),
        )
        .unwrap();
        assert_eq!(f.title.as_deref(), Some("A"));
        assert_eq!(f.order, Some(2));
        assert_eq!(f.str("examples"), Some("Btn"));
        assert_eq!(body.lines().position(|l| l == "# Oi"), Some(5));
    }

    #[test]
    fn invalid_reports_line() {
        let err = split("---\ntitle: A\norder: [x\n---\n", Path::new("a.md")).unwrap_err();
        assert!(err.to_string().starts_with("a.md:"), "{err}");
    }

    #[test]
    fn unclosed() {
        assert!(split("---\ntitle: A\n", Path::new("a.md")).is_err());
    }
}
