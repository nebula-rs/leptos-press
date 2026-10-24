//! Conformidade de um bundle com a OKF v0.2 (§11 da especificação).

use std::collections::BTreeMap;
use std::path::Path;

use leptos_press_build::yaml::Value;

/// Verifica as regras de conformidade da OKF v0.2 (§11) num diretório de bundle:
///
/// 1. todo `.md` não reservado tem front-matter YAML (um mapa) entre `---`;
/// 2. com `type` texto não vazio;
/// 3. os reservados seguem a estrutura: `index.md` sem front-matter (o da raiz pode ter só
///    `okf_version`), com ao menos um cabeçalho e itens de lista que começam por um link;
///    `log.md` sem front-matter e com cabeçalhos `##` no formato `AAAA-MM-DD`.
///
/// Devolve todas as violações (`caminho: mensagem`). O que a especificação manda **tolerar**
/// (links quebrados, `type` desconhecido, chaves extras, `index.md` ausente) não é erro.
pub fn check_bundle(dir: &Path) -> Result<(), Vec<String>> {
    let mut files = BTreeMap::new();
    let mut errors = Vec::new();
    collect(dir, dir, &mut files, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    check_files(&files)
}

fn collect(
    root: &Path,
    dir: &Path,
    files: &mut BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            errors.push(format!("{}: {e}", dir.display()));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files, errors);
        } else if path.extension().is_some_and(|e| e == "md") {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    files.insert(rel, text);
                }
                Err(e) => errors.push(format!("{rel}: {e}")),
            }
        }
    }
}

/// [`check_bundle`] sobre arquivos em memória (caminho relativo ao bundle → conteúdo).
pub(crate) fn check_files(files: &BTreeMap<String, String>) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for (path, text) in files {
        let name = path.rsplit('/').next().unwrap_or(path);
        let at_root = !path.contains('/');
        let result = match name {
            "index.md" => check_index(text, at_root),
            "log.md" => check_log(text),
            _ => check_concept(text),
        };
        if let Err(message) = result {
            errors.push(format!("{path}: {message}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// `Some((yaml, corpo))` se o texto abre com front-matter; `Err` se abre e não fecha.
fn split(text: &str) -> Result<Option<(&str, &str)>, String> {
    let Some(rest) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    else {
        return Ok(None);
    };
    if let Some(body) = rest.strip_prefix("---") {
        return Ok(Some(("", body)));
    }
    let end = rest
        .find("\n---\n")
        .map(|i| (i, i + 5))
        .or_else(|| rest.strip_suffix("\n---").map(|y| (y.len(), rest.len())))
        .ok_or("front-matter aberto com `---` e nunca fechado")?;
    Ok(Some((&rest[..end.0], &rest[end.1..])))
}

fn mapping(yaml: &str) -> Result<leptos_press_build::yaml::Mapping, String> {
    match leptos_press_build::yaml::from_str::<Value>(yaml) {
        Ok(Value::Mapping(m)) => Ok(m),
        Ok(Value::Null) => Ok(Default::default()),
        Ok(_) => Err("o front-matter precisa ser um mapa YAML".into()),
        Err(e) => Err(format!("front-matter inválido: {e}")),
    }
}

fn check_concept(text: &str) -> Result<(), String> {
    let (yaml, _) = split(text)?.ok_or("conceito sem front-matter (`type` é obrigatório)")?;
    let front = mapping(yaml)?;
    match front.get("type") {
        Some(Value::String(t)) if !t.trim().is_empty() => Ok(()),
        Some(Value::String(_)) => Err("`type` vazio".into()),
        Some(_) => Err("`type` precisa ser texto".into()),
        None => Err("front-matter sem `type`".into()),
    }
}

fn check_index(text: &str, at_root: bool) -> Result<(), String> {
    let body = match split(text)? {
        None => text,
        Some((yaml, body)) => {
            if !at_root {
                return Err("`index.md` fora da raiz não pode ter front-matter".into());
            }
            let front = mapping(yaml)?;
            if let Some((key, _)) = front
                .iter()
                .find(|(k, _)| k.as_str() != Some("okf_version"))
            {
                return Err(format!(
                    "o front-matter do `index.md` da raiz só pode ter `okf_version` (tem `{}`)",
                    key.as_str().unwrap_or("?")
                ));
            }
            if front
                .get("okf_version")
                .is_some_and(|v| !matches!(v, Value::String(_)))
            {
                return Err("`okf_version` precisa ser texto (`\"0.2\"`)".into());
            }
            body
        }
    };
    let mut fenced = false;
    let mut headings = 0;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if line.starts_with('#') {
            headings += 1;
        } else if let Some(item) = line.strip_prefix("* ").or_else(|| line.strip_prefix("- "))
            && (!item.starts_with('[') || !item.contains("]("))
        {
            return Err(format!("item de listagem sem link: `{line}`"));
        }
    }
    if headings == 0 {
        return Err("`index.md` sem nenhum cabeçalho de seção".into());
    }
    Ok(())
}

fn check_log(text: &str) -> Result<(), String> {
    if split(text)?.is_some() {
        return Err("`log.md` não pode ter front-matter".into());
    }
    for line in text.lines() {
        if let Some(date) = line.strip_prefix("## ")
            && !is_iso_date(date.trim())
        {
            return Err(format!(
                "cabeçalho de data fora do formato AAAA-MM-DD: `{line}`"
            ));
        }
    }
    Ok(())
}

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(files: &[(&str, &str)]) -> Result<(), Vec<String>> {
        check_files(
            &files
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
        )
    }

    #[test]
    fn minimal_concept_is_conformant() {
        assert_eq!(check(&[("a.md", "---\ntype: Reference\n---\n")]), Ok(()));
        assert_eq!(
            check(&[("a/b.md", "---\ntype: X\ntitle: Y\n---\n\n# Y\n")]),
            Ok(())
        );
    }

    #[test]
    fn concept_rules() {
        let bad = [
            ("sem front-matter", "# A\n"),
            ("sem type", "---\ntitle: A\n---\n"),
            ("type vazio", "---\ntype: ''\n---\n"),
            ("type não texto", "---\ntype: [a]\n---\n"),
            ("não fechado", "---\ntype: A\n"),
            ("não é mapa", "---\n- a\n---\n"),
        ];
        for (case, text) in bad {
            assert!(check(&[("a.md", text)]).is_err(), "{case}");
        }
    }

    #[test]
    fn index_rules() {
        let ok_root = "---\nokf_version: \"0.2\"\n---\n\n# S\n\n* [A](/a.md) - a\n";
        assert_eq!(check(&[("index.md", ok_root)]), Ok(()));
        assert_eq!(check(&[("x/index.md", "# S\n\n- [A](/x/a.md)\n")]), Ok(()));
        let e = check(&[(
            "index.md",
            "---\nokf_version: \"0.2\"\ntitle: X\n---\n# S\n",
        )]);
        assert!(e.unwrap_err()[0].contains("só pode ter `okf_version`"));
        let e = check(&[("x/index.md", "---\nokf_version: \"0.2\"\n---\n# S\n")]);
        assert!(e.unwrap_err()[0].contains("fora da raiz"));
        assert!(check(&[("index.md", "* [A](/a.md)\n")]).is_err());
        assert!(check(&[("index.md", "# S\n\n* A sem link\n")]).is_err());
    }

    #[test]
    fn log_rules() {
        assert_eq!(
            check(&[("log.md", "# Log\n\n## 2026-05-22\n* x\n")]),
            Ok(())
        );
        assert!(check(&[("log.md", "# Log\n\n## 22/05/2026\n")]).is_err());
        assert!(check(&[("log.md", "---\ntype: X\n---\n")]).is_err());
    }
}
