//! Realce de código no build (syntect → HTML com classes; as cores vêm do CSS do press).

use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::html::escape;

pub(crate) struct Highlighter {
    syntaxes: SyntaxSet,
}

impl Highlighter {
    pub(crate) fn new() -> Self {
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
        }
    }

    /// HTML com `<span class="…">`. Linguagem desconhecida → texto escapado, sem erro.
    pub(crate) fn highlight(&self, lang: &str, code: &str) -> String {
        self.try_highlight(lang, code)
            .unwrap_or_else(|| escape(code))
    }

    fn try_highlight(&self, lang: &str, code: &str) -> Option<String> {
        let token = match lang.to_ascii_lowercase().as_str() {
            "" | "text" | "txt" | "plain" | "plaintext" => return None,
            // O conjunto padrão do syntect não tem TypeScript nem TOML.
            "ts" | "typescript" | "tsx" | "jsx" | "mjs" => "js".to_string(),
            "shell" | "console" | "zsh" => "sh".to_string(),
            "rust,ignore" | "rust,no_run" => "rust".to_string(),
            other => other.to_string(),
        };
        let syntax = self.syntaxes.find_syntax_by_token(&token)?;
        let mut generator =
            ClassedHTMLGenerator::new_with_class_style(syntax, &self.syntaxes, ClassStyle::Spaced);
        for line in LinesWithEndings::from(code) {
            generator
                .parse_html_for_line_which_includes_newline(line)
                .ok()?;
        }
        Some(generator.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_gets_spans() {
        let html = Highlighter::new().highlight("rust", "fn main() {}\n");
        assert!(html.contains("<span class=\""), "{html}");
        assert!(html.contains("main"));
    }

    #[test]
    fn unknown_language_is_escaped_text() {
        assert_eq!(Highlighter::new().highlight("nope", "a < b"), "a &lt; b");
    }
}
