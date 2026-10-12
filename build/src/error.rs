//! Erros do build, sempre com `arquivo:linha` do `.md` quando se conhece.

use std::fmt;
use std::path::{Path, PathBuf};

/// Erro de build. O `Display` é `arquivo.md:linha: mensagem` (partes ausentes são omitidas).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            file: None,
            line: None,
            message: message.into(),
        }
    }

    /// Completa arquivo e linha **só** onde ainda não há (o erro mais interno vence).
    pub fn at(mut self, file: &Path, line: usize) -> Self {
        if self.file.is_none() {
            self.file = Some(file.to_path_buf());
        }
        if self.line.is_none() {
            self.line = Some(line);
        }
        self
    }

    pub(crate) fn io(path: &Path, source: &std::io::Error) -> Self {
        Self {
            file: Some(path.to_path_buf()),
            line: None,
            message: format!("erro de E/S: {source}"),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.file, self.line) {
            (Some(file), Some(line)) => write!(f, "{}:{line}: {}", file.display(), self.message),
            (Some(file), None) => write!(f, "{}: {}", file.display(), self.message),
            (None, Some(line)) => write!(f, "linha {line}: {}", self.message),
            (None, None) => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for Error {}
