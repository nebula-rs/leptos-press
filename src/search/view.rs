//! Interface da busca: o botão "Buscar" do cabeçalho, os atalhos (`Ctrl/⌘+K` e `/`) e o
//! diálogo com o campo e a lista de resultados.
//!
//! Seguro para SSR: o servidor renderiza o botão e o `<dialog>` fechado; o `fetch` do arquivo,
//! os atalhos, a largura da janela e os atributos ARIA do campo só existem em `Effect`s e
//! handlers.

use std::sync::Arc;

use leptos::ev;
use leptos::leptos_dom::helpers::{request_animation_frame, window_event_listener};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_nebula::{
    Btn, BtnDesign, Color, Dialog, FieldDesign, FieldPrepend, Icon, Input, InputHandle, Spinner,
};
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use super::index::SearchIndex;
use super::query::{self, HitGroup, Segment};
use crate::SiteMeta;

const INPUT_ID: &str = "lxp-search-input";
const LIST_ID: &str = "lxp-search-list";
/// Abaixo disto (o `max-sm` do UnoCSS) o diálogo abre em tela cheia.
const NARROW_QUERY: &str = "(max-width: 639.98px)";

/// Estado do índice: carregado uma vez por sessão, na primeira abertura.
#[derive(Clone)]
enum Load {
    Idle,
    Loading,
    Ready(Arc<SearchIndex>),
    Failed,
}

/// Botão "Buscar", atalhos e diálogo. Só é usado quando `site.search` é `Some(caminho)`.
#[component]
pub(crate) fn Search(site: &'static SiteMeta, path: &'static str) -> impl IntoView {
    let show = RwSignal::new(false);
    // No celular o diálogo é `maximized`; a prop do `Dialog` não é reativa, então o diálogo é
    // recriado (fechado) quando a largura muda entre duas aberturas.
    let maximized = RwSignal::new(false);
    let load = RwSignal::new(Load::Idle);
    let query_text = RwSignal::new(String::new());
    let active = RwSignal::new(None::<usize>);
    let input = InputHandle::new();
    let kbd = RwSignal::new("Ctrl K");
    let navigate = StoredValue::new_local(use_navigate());
    // Âncora do resultado escolhido: a rolagem até ela é refeita depois que o diálogo fecha
    // (a do roteador acontece com a página ainda bloqueada pelo diálogo e se perde).
    // `Some(None)` = topo da página.
    let pending = StoredValue::new(None::<Option<&'static str>>);

    let terms = Memo::new(move |_| query::terms(&query_text.get()));
    let groups: Memo<Vec<HitGroup>> = Memo::new(move |_| {
        let Load::Ready(index) = load.get() else {
            return Vec::new();
        };
        terms.with(|t| query::group(&index, &query::search(&index, t)))
    });
    // Ordem de navegação (a da tela): as seções, grupo a grupo.
    let flat = Memo::new(move |_| {
        groups.with(|g| {
            g.iter()
                .flat_map(|g| g.sections.iter().copied())
                .collect::<Vec<usize>>()
        })
    });

    let fetch = move || {
        if matches!(load.get_untracked(), Load::Loading | Load::Ready(_)) {
            return;
        }
        load.set(Load::Loading);
        spawn_local(async move {
            let next = match fetch_text(path).await {
                Ok(text) => Load::Ready(Arc::new(build_index(&text, site))),
                Err(_) => Load::Failed,
            };
            load.set(next);
        });
    };

    let open = move || {
        if show.get_untracked() {
            input.focus();
            return;
        }
        fetch();
        let narrow = web_sys::window()
            .and_then(|w| w.match_media(NARROW_QUERY).ok().flatten())
            .is_some_and(|m| m.matches());
        if narrow != maximized.get_untracked() {
            maximized.set(narrow);
            // O diálogo novo nasce no próximo quadro; só então abre.
            request_animation_frame(move || show.set(true));
        } else {
            show.set(true);
        }
    };

    let go = move |section: usize| {
        let Load::Ready(index) = load.get_untracked() else {
            return;
        };
        let href = index.href(section);
        pending.set_value(Some(index.sections[section].anchor));
        show.set(false);
        navigate.with_value(|nav| nav(&href, NavigateOptions::default()));
    };

    // Atalhos globais e o rótulo da tecla (⌘ no Mac): só no navegador.
    Effect::new(move |_| {
        if is_mac() {
            kbd.set("⌘ K");
        }
        let handle = window_event_listener(ev::keydown, move |e: ev::KeyboardEvent| {
            if e.default_prevented() || e.is_composing() {
                return;
            }
            let key = e.key();
            let mods = e.ctrl_key() || e.meta_key();
            let ctrl_k = mods && !e.alt_key() && !e.shift_key() && key.eq_ignore_ascii_case("k");
            let slash = key == "/" && !mods && !e.alt_key() && !in_text_field(&e);
            if ctrl_k || slash {
                e.prevent_default();
                open();
            }
        });
        on_cleanup(move || handle.remove());
    });

    // Resultados novos: a primeira fica ativa (`Enter` vai direto para ela).
    Effect::new(move |_| {
        let n = flat.with(Vec::len);
        active.set((n > 0).then_some(0));
    });

    // ARIA no `<input>` do `Input` (que não tem essas props): papel de combobox, lista
    // controlada e opção ativa. Refeito ao abrir (o diálogo pode ter sido recriado).
    Effect::new(move |_| {
        let open = show.get();
        let expanded = flat.with(|f| !f.is_empty());
        let current = active.get();
        if !open {
            return;
        }
        let Some(el) = document_element(INPUT_ID) else {
            return;
        };
        let _ = el.set_attribute("role", "combobox");
        let _ = el.set_attribute("aria-autocomplete", "list");
        let _ = el.set_attribute("aria-controls", LIST_ID);
        let _ = el.set_attribute("aria-label", "Buscar na documentação");
        let _ = el.set_attribute("autocomplete", "off");
        let _ = el.set_attribute("spellcheck", "false");
        let _ = el.set_attribute("aria-expanded", if expanded { "true" } else { "false" });
        match current.filter(|_| expanded) {
            Some(k) => {
                let _ = el.set_attribute("aria-activedescendant", &option_id(k));
                scroll_into_view(&option_id(k));
            }
            None => {
                let _ = el.remove_attribute("aria-activedescendant");
            }
        }
    });

    let on_keydown = Callback::new(move |e: ev::KeyboardEvent| {
        if e.is_composing() {
            return;
        }
        let n = flat.with_untracked(Vec::len);
        match e.key().as_str() {
            "ArrowDown" if n > 0 => {
                e.prevent_default();
                active.update(|a| *a = Some(a.map_or(0, |k| (k + 1).min(n - 1))));
            }
            "ArrowUp" if n > 0 => {
                e.prevent_default();
                active.update(|a| *a = Some(a.map_or(0, |k| k.saturating_sub(1))));
            }
            "Enter" => {
                e.prevent_default();
                let target = active
                    .get_untracked()
                    .and_then(|k| flat.with_untracked(|f| f.get(k).copied()));
                if let Some(section) = target {
                    go(section);
                }
            }
            _ => {}
        }
    });

    let on_hide = Callback::new(move |()| {
        query_text.set(String::new());
        if let Some(anchor) = pending.try_update_value(Option::take).flatten() {
            request_animation_frame(move || match anchor.and_then(document_element) {
                Some(el) => el.scroll_into_view(),
                None => {
                    if let Some(w) = web_sys::window() {
                        w.scroll_to_with_x_and_y(0.0, 0.0);
                    }
                }
            });
        }
    });
    let on_show = Callback::new(move |()| input.focus());

    let panel = move |max: bool| {
        let panel_class = if max { "lxp-search-max" } else { "lxp-search" };
        view! {
            <div class=panel_class>
                <div class="lxp-search-bar">
                    <div class="lxp-search-field">
                        <Input
                            value=query_text
                            id=INPUT_ID
                            placeholder="Buscar na documentação"
                            design=FieldDesign::Outlined
                            dense=true
                            clearable=true
                            hide_bottom_space=true
                            handle=input
                            on_keydown=on_keydown
                        >
                            <FieldPrepend slot:prepend>
                                <Icon name="i-lucide-search" />
                            </FieldPrepend>
                        </Input>
                    </div>
                    <Btn
                        round=true
                        dense=true
                        design=BtnDesign::Flat
                        color=Color::Neutral
                        icon="i-lucide-x"
                        aria_label="Fechar a busca"
                        on_click=move |_| show.set(false)
                    />
                </div>
                <div class="lxp-search-body">
                    {move || {
                        // "Tentar de novo" devolve o foco ao campo (o botão some com o erro).
                        let retry = move || {
                            fetch();
                            input.focus();
                        };
                        body(load, terms, groups, active, go, retry)
                    }}
                </div>
                <div class="lxp-search-footer" aria-hidden="true">
                    <span class="lxp-search-hint">
                        <kbd class="lxp-search-kbd">"↑"</kbd>
                        <kbd class="lxp-search-kbd">"↓"</kbd>
                        "navegar"
                    </span>
                    <span class="lxp-search-hint">
                        <kbd class="lxp-search-kbd">"Enter"</kbd>
                        "abrir"
                    </span>
                    <span class="lxp-search-hint">
                        <kbd class="lxp-search-kbd">"Esc"</kbd>
                        "fechar"
                    </span>
                </div>
            </div>
        }
    };

    view! {
        <button
            type="button"
            class="lxp-search-btn"
            aria-label="Buscar"
            aria-haspopup="dialog"
            aria-keyshortcuts="Control+K Meta+K /"
            on:click=move |_| open()
        >
            <span class="i-lucide-search lxp-search-btn-icon" aria-hidden="true"></span>
            <span class="lxp-search-btn-label">"Buscar"</span>
            <kbd class="lxp-search-kbd lxp-search-btn-kbd" aria-hidden="true">
                {move || kbd.get()}
            </kbd>
        </button>
        {move || {
            let max = maximized.get();
            view! {
                <Dialog
                    show=show
                    maximized=max
                    aria_label="Buscar"
                    on_show=on_show
                    on_hide=on_hide
                >
                    {panel(max)}
                </Dialog>
            }
        }}
    }
}

/// Corpo do diálogo conforme o estado do índice e da busca.
fn body(
    load: RwSignal<Load>,
    terms: Memo<Vec<String>>,
    groups: Memo<Vec<HitGroup>>,
    active: RwSignal<Option<usize>>,
    go: impl Fn(usize) + Copy + Send + Sync + 'static,
    retry: impl Fn() + Copy + Send + Sync + 'static,
) -> AnyView {
    match load.get() {
        Load::Idle | Load::Loading => view! {
            <div class="lxp-search-state">
                <Spinner size="2em" aria_label="Carregando o índice da busca" />
                <span>"Carregando o índice…"</span>
            </div>
        }
        .into_any(),
        Load::Failed => view! {
            <div class="lxp-search-state" role="alert">
                <span>"Não foi possível carregar o índice da busca."</span>
                <Btn
                    label="Tentar de novo"
                    icon="i-lucide-rotate-ccw"
                    design=BtnDesign::Outline
                    on_click=move |_| retry()
                />
            </div>
        }
        .into_any(),
        Load::Ready(index) => {
            let none = move || {
                let t = terms.get();
                if !t.is_empty() && groups.with(Vec::is_empty) {
                    "Nenhum resultado para a busca.".to_string()
                } else {
                    String::new()
                }
            };
            view! {
                <Show when=move || terms.with(Vec::is_empty)>
                    <div class="lxp-search-state">"Digite para buscar nas páginas."</div>
                </Show>
                <div class="lxp-search-live" aria-live="polite" aria-atomic="true">
                    {none}
                </div>
                {move || results(&index, terms.get(), groups.get(), active, go)}
            }
            .into_any()
        }
    }
}

/// A lista de resultados agrupada por página (`listbox` > `group` > `option`).
fn results(
    index: &SearchIndex,
    terms: Vec<String>,
    groups: Vec<HitGroup>,
    active: RwSignal<Option<usize>>,
    go: impl Fn(usize) + Copy + Send + Sync + 'static,
) -> AnyView {
    if groups.is_empty() {
        return ().into_any();
    }
    let mut k = 0;
    let groups = groups
        .into_iter()
        .enumerate()
        .map(|(g, group)| {
            let page = &index.pages[group.page];
            let title_id = format!("lxp-search-g{g}");
            let labelledby = title_id.clone();
            let options = group
                .sections
                .iter()
                .map(|&section| {
                    let flat = k;
                    k += 1;
                    option(index, section, flat, &terms, active, go)
                })
                .collect_view();
            view! {
                <div class="lxp-search-group" role="group" aria-labelledby=labelledby>
                    <div class="lxp-search-group-title" id=title_id>
                        <span>{page.title}</span>
                        {page.group.map(|g| view! { <span class="lxp-search-group-name">{g}</span> })}
                    </div>
                    {options}
                </div>
            }
        })
        .collect_view();
    view! {
        <div class="lxp-search-list" role="listbox" id=LIST_ID aria-label="Resultados da busca">
            {groups}
        </div>
    }
    .into_any()
}

fn option(
    index: &SearchIndex,
    section: usize,
    flat: usize,
    terms: &[String],
    active: RwSignal<Option<usize>>,
    go: impl Fn(usize) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let s = &index.sections[section];
    let page = &index.pages[s.page];
    // Caminho: os cabeçalhos (o pai mais apagado) ou, no topo, o título da página.
    let (parent, name) = match s.headings.as_slice() {
        [] => (None, page.title.to_string()),
        [one] => (None, one.clone()),
        [first, .., last] => (Some(format!("{first} › ")), last.clone()),
    };
    let name = marked(query::highlight(&name, terms));
    let has = |norm: &str| terms.iter().any(|t| norm.contains(t.as_str()));
    // Trecho do texto corrido; do código só se o termo está lá e não no texto.
    let (snippet, code) = if !s.text.is_empty() && (has(&s.norm_text) || !has(&s.norm_code)) {
        (query::snippet(&s.text, terms), false)
    } else {
        (query::snippet(&s.code, terms), true)
    };
    let has_snippet = snippet.iter().any(|seg| !seg.text.trim().is_empty());
    let snippet_class = if code {
        "lxp-search-snippet-code"
    } else {
        "lxp-search-snippet-text"
    };
    let snippet = has_snippet.then(|| view! { <span class=snippet_class>{marked(snippet)}</span> });
    let href = index.href(section);
    let id = option_id(flat);
    let is_active = move || active.get() == Some(flat);
    view! {
        <a
            class=move || if is_active() { "lxp-search-hit-active" } else { "lxp-search-hit" }
            href=href
            id=id
            role="option"
            tabindex="-1"
            aria-selected=move || if is_active() { "true" } else { "false" }
            on:mousemove=move |_| {
                if active.get_untracked() != Some(flat) {
                    active.set(Some(flat));
                }
            }
            on:click=move |e: ev::MouseEvent| {
                // Clique simples: navega pelo roteador e fecha; com modificador, o navegador
                // abre em outra aba como um link qualquer.
                if e.button() == 0 && !(e.ctrl_key() || e.meta_key() || e.shift_key() || e.alt_key()) {
                    e.prevent_default();
                    go(section);
                }
            }
        >
            <span class="lxp-search-hit-path">
                {parent.map(|p| view! { <span class="lxp-search-hit-parent">{p}</span> })}
                {name}
            </span>
            {snippet}
        </a>
    }
}

/// Pedaços de texto com os termos em `<mark>`.
fn marked(segments: Vec<Segment>) -> impl IntoView {
    segments
        .into_iter()
        .map(|s| {
            if s.mark {
                view! { <mark class="lxp-search-mark">{s.text}</mark> }.into_any()
            } else {
                s.text.into_any()
            }
        })
        .collect_view()
}

fn option_id(k: usize) -> String {
    format!("lxp-search-opt-{k}")
}

/// Monta o índice, medindo o tempo (`performance.measure("lxp-search-index")`, visível nas
/// ferramentas do navegador).
fn build_index(text: &str, site: &'static SiteMeta) -> SearchIndex {
    let perf = web_sys::window().and_then(|w| w.performance());
    if let Some(p) = &perf {
        let _ = p.mark("lxp-search-index-start");
    }
    let index = SearchIndex::build(text, site);
    if let Some(p) = &perf {
        let _ = p.mark("lxp-search-index-end");
        let _ = p.measure_with_start_mark_and_end_mark(
            "lxp-search-index",
            "lxp-search-index-start",
            "lxp-search-index-end",
        );
    }
    index
}

async fn fetch_text(path: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or("sem window")?;
    let resp = JsFuture::from(window.fetch_with_str(path))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: web_sys::Response = resp.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    text.as_string()
        .ok_or_else(|| "resposta sem texto".to_string())
}

fn document_element(id: &str) -> Option<web_sys::Element> {
    web_sys::window()?.document()?.get_element_by_id(id)
}

fn scroll_into_view(id: &str) {
    if let Some(el) = document_element(id) {
        let opts = web_sys::ScrollIntoViewOptions::new();
        opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
        el.scroll_into_view_with_scroll_into_view_options(&opts);
    }
}

/// O alvo da tecla é um campo de texto (`/` digita, não abre a busca).
fn in_text_field(e: &ev::KeyboardEvent) -> bool {
    let Some(el) = e
        .target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return false;
    };
    matches!(
        el.tag_name().to_ascii_lowercase().as_str(),
        "input" | "textarea" | "select"
    ) || el.is_content_editable()
}

fn is_mac() -> bool {
    web_sys::window()
        .and_then(|w| w.navigator().platform().ok())
        .is_some_and(|p| p.starts_with("Mac") || p.starts_with("iP"))
}
