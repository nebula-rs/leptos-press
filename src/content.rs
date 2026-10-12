//! O que o código gerado chama: `html`, `table`, `code_block`, `alert`, `render_page`.
//!
//! Estas funções são o contrato com o `leptos-press-build`: mudar assinatura aqui exige mudar
//! o gerador (e os snapshots dele).

use leptos::prelude::*;
use leptos_meta::{Meta, Title};
use leptos_nebula::{
    Banner, BannerAvatar, Card, CardSection, Color, Icon, MarkupSeparator, MarkupTable,
};

use crate::{CodeBlock, HomeLayout, PageLayout, PageMeta, PrevNext, SiteMeta};

/// Container com a tipografia do Markdown (estilos por elemento, em tokens).
#[component]
pub fn Prose(children: Children) -> impl IntoView {
    view! { <div class="lxp-prose">{children()}</div> }
}

/// Trecho de HTML estático gerado no build a partir do Markdown (parágrafos, listas,
/// cabeçalhos…).
///
/// `inner_html` é seguro aqui: o conteúdo vem do `.md` do próprio site, escapado pelo gerador
/// em tempo de build — nunca de entrada do usuário em execução.
pub fn html(html: &'static str) -> AnyView {
    view! { <div class="lxp-html" inner_html=html></div> }.into_any()
}

/// Tabela GFM: `inner` é o conteúdo do `<table>` (`thead`/`tbody`), gerado no build.
pub fn table(inner: &'static str) -> AnyView {
    view! {
        <MarkupTable bordered=true flat=true separator=MarkupSeparator::Horizontal wrap_cells=true>
            <table inner_html=inner></table>
        </MarkupTable>
    }
    .into_any()
}

/// Bloco de código com realce feito no build. Ver [`CodeBlock`].
pub fn code_block(lang: &'static str, code: &'static str, highlighted: &'static str) -> AnyView {
    view! { <CodeBlock lang=lang code=code highlighted=highlighted /> }.into_any()
}

/// Como [`code_block`], encostado na base da caixa (para o [`ExampleBox`]).
pub fn code_block_attached(
    lang: &'static str,
    code: &'static str,
    highlighted: &'static str,
) -> AnyView {
    view! { <CodeBlock lang=lang code=code highlighted=highlighted attached=true /> }.into_any()
}

/// Tipo de alerta (`> [!NOTE]` / `::: tip`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl AlertKind {
    fn color(self) -> Color {
        match self {
            AlertKind::Note => Color::Info,
            AlertKind::Tip => Color::Positive,
            AlertKind::Important => Color::Primary,
            AlertKind::Warning => Color::Warning,
            AlertKind::Caution => Color::Negative,
        }
    }

    fn icon(self) -> AnyView {
        match self {
            AlertKind::Note => view! { <Icon name="i-lucide-info" /> }.into_any(),
            AlertKind::Tip => view! { <Icon name="i-lucide-lightbulb" /> }.into_any(),
            AlertKind::Important => {
                view! { <Icon name="i-lucide-message-square-warning" /> }.into_any()
            }
            AlertKind::Warning => view! { <Icon name="i-lucide-triangle-alert" /> }.into_any(),
            AlertKind::Caution => view! { <Icon name="i-lucide-octagon-alert" /> }.into_any(),
        }
    }
}

/// Alerta/container: um [`Banner`] com ícone, título e o conteúdo.
pub fn alert(kind: AlertKind, title: &'static str, content: Vec<AnyView>) -> AnyView {
    view! {
        <div class="lxp-alert">
            <Banner color=kind.color() rounded=true>
                <BannerAvatar slot:avatar>{kind.icon()}</BannerAvatar>
                <div class="lxp-alert-title">{title}</div>
                {content}
            </Banner>
        </div>
    }
    .into_any()
}

/// Monta uma página: título/descrição no `<head>` e o layout (`doc` ou `home`).
pub fn render_page(meta: &'static PageMeta, blocks: Vec<AnyView>) -> AnyView {
    let site = use_context::<&'static SiteMeta>();
    let title = match site {
        Some(site) if meta.path != "/" && !site.title.is_empty() => {
            format!("{} | {}", meta.title, site.title)
        }
        Some(site) if !site.title.is_empty() => site.title.to_string(),
        _ => meta.title.to_string(),
    };
    let description = if meta.description.is_empty() {
        site.map(|s| s.description).unwrap_or_default()
    } else {
        meta.description
    };
    let body = match (meta.layout, meta.home) {
        (PageLayout::Home, Some(home)) => {
            view! { <HomeLayout home=home><Prose>{blocks}</Prose></HomeLayout> }.into_any()
        }
        _ => view! {
            <Prose>{blocks}</Prose>
            <PrevNext path=meta.path />
        }
        .into_any(),
    };
    view! {
        <Title text=title />
        <Meta name="description" content=description />
        {body}
    }
    .into_any()
}

/// Exemplo vivo: o componente renderizado (`children`) numa caixa com o código-fonte
/// encostado na base.
///
/// `title` é opcional (sem ele não há barra de título; use quando o texto acima não basta).
/// `column` empilha os filhos (componentes de bloco, como `Banner`); o padrão põe lado a lado
/// (componentes inline, como `Btn`). `code` deve ser um [`CodeBlock`] em modo `attached`
/// (ver [`code_block_attached`]).
#[component]
pub fn ExampleBox(
    #[prop(optional_no_strip)] title: Option<&'static str>,
    #[prop(optional)] column: bool,
    code: AnyView,
    children: Children,
) -> impl IntoView {
    let stage = if column {
        "lxp-example-stage-column"
    } else {
        "lxp-example-stage-row"
    };
    view! {
        <div class="lxp-example">
            <Card bordered=true flat=true>
                <CardSection>
                    {title.map(|t| view! { <span class="lxp-example-title">{t}</span> })}
                    <div class=stage>{children()}</div>
                </CardSection>
                {code}
            </Card>
        </div>
    }
}
