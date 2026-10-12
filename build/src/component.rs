//! Extensão: tags registradas no Markdown (`<DocExample file="Basic" />`).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;

use crate::error::Error;
use crate::frontmatter::FrontMatter;

/// Atributos `nome="valor"` de uma tag. Atributo nunca lido por `get`/`require` vira erro de
/// build (`atributo desconhecido`).
#[derive(Debug, Default)]
pub struct Attrs {
    map: BTreeMap<String, String>,
    used: RefCell<BTreeSet<String>>,
}

impl Attrs {
    pub(crate) fn new(map: BTreeMap<String, String>) -> Self {
        Self {
            map,
            used: RefCell::default(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        let v = self.map.get(name)?;
        self.used.borrow_mut().insert(name.to_string());
        Some(v)
    }

    pub fn require(&self, name: &str) -> Result<&str, Error> {
        self.get(name)
            .ok_or_else(|| Error::new(format!("atributo obrigatório `{name}` ausente")))
    }

    pub(crate) fn unused(&self) -> Vec<&str> {
        let used = self.used.borrow();
        self.map
            .keys()
            .filter(|k| !used.contains(*k))
            .map(String::as_str)
            .collect()
    }
}

/// Contexto de uma tag.
pub struct Ctx<'a> {
    /// Caminho do `.md` da página (relativo à raiz do site).
    pub page_file: &'a Path,
    /// Rota da página (`/components/btn`).
    pub route: &'a str,
    /// Front-matter da página.
    pub front: &'a FrontMatter,
    /// Raiz do site (diretório do `Cargo.toml` que tem o `build.rs`).
    pub root: &'a Path,
    pub(crate) reads: &'a RefCell<Vec<PathBuf>>,
}

impl Ctx<'_> {
    /// Registra um arquivo lido, para o `cargo:rerun-if-changed`. Devolve o caminho absoluto.
    pub fn track(&self, relative: impl AsRef<Path>) -> PathBuf {
        let path = self.root.join(relative);
        self.reads.borrow_mut().push(path.clone());
        path
    }

    /// Lê um arquivo texto relativo à raiz do site e o registra como entrada do build.
    pub fn read_to_string(&self, relative: impl AsRef<Path>) -> Result<String, Error> {
        let path = self.track(relative);
        // Sem `file`: o erro ganha o `.md:linha` da tag que pediu o arquivo.
        std::fs::read_to_string(&path)
            .map_err(|e| Error::new(format!("não foi possível ler {}: {e}", path.display())))
    }
}

/// Uma tag aceita no Markdown, com **duas** saídas: código e texto para LLM.
pub trait MdComponent {
    /// Nome da tag (`"DocExample"`).
    fn tag(&self) -> &'static str;

    /// Expressão Rust que substitui a tag na página. Precisa avaliar para uma view
    /// (o gerador aplica `into_any()`).
    fn codegen(&self, attrs: &Attrs, ctx: &Ctx) -> Result<TokenStream, Error>;

    /// Itens no nível do módulo gerado (ex.: `#[path = "…"] mod exemplo;`). Itens idênticos
    /// pedidos por várias tags são emitidos uma vez.
    fn items(&self, _attrs: &Attrs, _ctx: &Ctx) -> Result<TokenStream, Error> {
        Ok(TokenStream::new())
    }

    /// Markdown que substitui a tag no `llms-full.txt` e na cópia `.md` da página.
    fn llms(&self, attrs: &Attrs, ctx: &Ctx) -> Result<String, Error>;
}
