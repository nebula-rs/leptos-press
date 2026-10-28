//! Metadados do site, gerados pelo `leptos-press-build` (`SITE` em `press.rs`).
//!
//! Tudo `&'static`: o gerador emite `static`s, sem alocação nem estado de execução.

/// Link do cabeçalho. `link` é uma página real; `prefix` é a rota que acende o item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavLink {
    pub text: &'static str,
    pub link: &'static str,
    pub prefix: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarItem {
    pub text: &'static str,
    pub link: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarGroup {
    pub text: &'static str,
    /// Semântica do `collapsed` do VitePress: `None` = não recolhível; `Some(false)` =
    /// recolhível, aberto; `Some(true)` = recolhível, fechado.
    pub collapsed: Option<bool>,
    pub items: &'static [SidebarItem],
}

impl SidebarGroup {
    /// O grupo tem um item com a rota `path`.
    pub fn contains(&self, path: &str) -> bool {
        let path = normalize_path(path);
        self.items.iter().any(|i| i.link == path)
    }

    /// Estado inicial de um grupo recolhível para a página `path`: aberto se `collapsed` é
    /// `Some(false)` ou se a página atual está no grupo. `None` (não recolhível) = sempre aberto.
    pub fn open_for(&self, path: &str) -> bool {
        match self.collapsed {
            None | Some(false) => true,
            Some(true) => self.contains(path),
        }
    }
}

/// Layout da página (`layout:` do front-matter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLayout {
    Doc,
    Home,
}

/// Cabeçalho `h2`/`h3` da página, para o outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub id: &'static str,
    pub text: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroAction {
    pub text: &'static str,
    pub link: &'static str,
    /// `theme: brand` (ou a primeira ação): botão cheio; senão, contornado.
    pub primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Feature {
    pub title: &'static str,
    pub details: &'static str,
    /// Emoji ou texto curto; vazio = sem ícone.
    pub icon: &'static str,
}

/// Conteúdo do `layout: home`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeMeta {
    pub name: Option<&'static str>,
    pub text: Option<&'static str>,
    pub tagline: Option<&'static str>,
    pub actions: &'static [HeroAction],
    pub features: &'static [Feature],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMeta {
    /// Rota (`/components/btn`; `/` para a home).
    pub path: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub layout: PageLayout,
    /// Só `h2` e `h3`.
    pub headings: &'static [Heading],
    pub home: Option<&'static HomeMeta>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiteMeta {
    pub title: &'static str,
    pub description: &'static str,
    pub nav: &'static [NavLink],
    pub sidebar: &'static [SidebarGroup],
    pub pages: &'static [PageMeta],
    /// Caminho do texto que a busca lê (`/llms-full.txt`) quando o site liga a busca
    /// (`Site::search(true)` no build); `None` = sem busca (nem o botão "Buscar").
    pub search: Option<&'static str>,
}

/// Remove a barra final (`/guide/` → `/guide`); a raiz continua `/`.
pub fn normalize_path(path: &str) -> &str {
    match path.trim_end_matches('/') {
        "" => "/",
        trimmed => trimmed,
    }
}

impl SiteMeta {
    pub fn page(&self, path: &str) -> Option<&'static PageMeta> {
        let path = normalize_path(path);
        self.pages.iter().find(|p| p.path == path)
    }

    /// Itens da sidebar na ordem de leitura (base de anterior/próxima).
    pub fn reading_order(&self) -> impl Iterator<Item = &'static SidebarItem> + '_ {
        self.sidebar.iter().flat_map(|g| g.items.iter())
    }

    /// (anterior, próxima) da página `path` na ordem da sidebar.
    pub fn neighbors(
        &self,
        path: &str,
    ) -> (Option<&'static SidebarItem>, Option<&'static SidebarItem>) {
        let path = normalize_path(path);
        let items: Vec<&'static SidebarItem> = self.reading_order().collect();
        let Some(i) = items.iter().position(|item| item.link == path) else {
            return (None, None);
        };
        (
            i.checked_sub(1).map(|j| items[j]),
            items.get(i + 1).copied(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static ITEMS: [SidebarItem; 3] = [
        SidebarItem {
            text: "A",
            link: "/g/a",
        },
        SidebarItem {
            text: "B",
            link: "/g/b",
        },
        SidebarItem {
            text: "C",
            link: "/g/c",
        },
    ];
    static GROUPS: [SidebarGroup; 1] = [SidebarGroup {
        text: "G",
        collapsed: None,
        items: &ITEMS,
    }];
    static SITE: SiteMeta = SiteMeta {
        title: "t",
        description: "",
        nav: &[],
        sidebar: &GROUPS,
        pages: &[],
        search: None,
    };

    #[test]
    fn normalizes() {
        assert_eq!(normalize_path("/g/"), "/g");
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path(""), "/");
    }

    #[test]
    fn collapsed_group_opens_on_current_page() {
        let mut g = GROUPS[0];
        assert!(g.open_for("/x"));
        g.collapsed = Some(false);
        assert!(g.open_for("/x"));
        g.collapsed = Some(true);
        assert!(!g.open_for("/x"));
        assert!(g.open_for("/g/b/"));
        assert!(g.contains("/g/a"));
    }

    #[test]
    fn neighbors_follow_sidebar_order() {
        let (prev, next) = SITE.neighbors("/g/b/");
        assert_eq!(prev.map(|i| i.text), Some("A"));
        assert_eq!(next.map(|i| i.text), Some("C"));
        assert_eq!(SITE.neighbors("/g/a").0.map(|i| i.text), None);
        assert_eq!(SITE.neighbors("/g/c").1.map(|i| i.text), None);
        assert!(SITE.neighbors("/x").0.is_none() && SITE.neighbors("/x").1.is_none());
    }
}
