//! Utilidades de HTML: escape e slug de âncoras.

pub(crate) fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    push_escaped(&mut out, s);
    out
}

pub(crate) fn push_escaped(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// Slug de âncora no estilo do GitHub/VitePress: minúsculas, alfanuméricos (Unicode) mantidos,
/// espaços e `-`/`_` viram `-`, o resto some.
pub(crate) fn slugify(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if (c.is_whitespace() || c == '-' || c == '_') && !out.ends_with('-') {
            out.push('-');
        }
    }
    let slug = out.trim_matches('-');
    if slug.is_empty() {
        "section".to_string()
    } else {
        slug.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slugify("Começando agora!"), "começando-agora");
        assert_eq!(slugify("  A  B-C_D "), "a-b-c-d");
        assert_eq!(slugify("???"), "section");
    }

    #[test]
    fn escapes() {
        assert_eq!(escape("a<b>&\"c"), "a&lt;b&gt;&amp;&quot;c");
    }
}
