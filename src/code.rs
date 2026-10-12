use leptos::prelude::*;
use leptos_nebula::{Btn, BtnDesign, Color, Icon};
use wasm_bindgen::JsCast;

/// Bloco de código: HTML já realçado no build, linguagem e botão de copiar no canto superior
/// direito (sem barra, como o VitePress).
///
/// O botão aparece no hover do bloco e no foco por teclado, e fica sempre visível em telas
/// sem hover. `attached` encosta o bloco na base de uma caixa (ex.: [`ExampleBox`](crate::ExampleBox)):
/// sem raio, margem nem borda própria além da de cima.
///
/// `highlighted` vem do `leptos-press-build` (syntect, escapado no build) e entra por
/// `inner_html`. A Clipboard API só é tocada no handler do clique, nunca na renderização
/// (ADR-0007).
#[component]
pub fn CodeBlock(
    lang: &'static str,
    code: &'static str,
    highlighted: &'static str,
    #[prop(optional)] attached: bool,
) -> impl IntoView {
    let copied = RwSignal::new(false);
    let on_copy = move |_| {
        copy_to_clipboard(code);
        copied.set(true);
        set_timeout(
            move || copied.set(false),
            std::time::Duration::from_millis(1500),
        );
    };
    // `group`: o botão de copiar reage ao hover/foco do bloco (`group-hover:` nos shortcuts).
    let wrapper = if attached {
        "group lxp-code-attached"
    } else {
        "group lxp-code"
    };
    view! {
        <div class=wrapper>
            <div class="lxp-code-tools">
                {(!lang.is_empty()).then(|| view! { <span class="lxp-code-lang">{lang}</span> })}
                <span class="lxp-code-copy">
                    <Btn
                        round=true
                        dense=true
                        design=BtnDesign::Flat
                        color=Color::Neutral
                        aria_label="Copiar código"
                        on_click=on_copy
                    >
                        {move || {
                            if copied.get() {
                                view! { <Icon name="i-lucide-check" /> }.into_any()
                            } else {
                                view! { <Icon name="i-lucide-copy" /> }.into_any()
                            }
                        }}
                    </Btn>
                </span>
            </div>
            <pre class="lxp-pre">
                <code inner_html=highlighted></code>
            </pre>
        </div>
    }
}

/// `navigator.clipboard.writeText(text)` sem as APIs instáveis do `web-sys`.
fn copy_to_clipboard(text: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let navigator = window.navigator();
    let Ok(clipboard) = js_sys::Reflect::get(&navigator, &"clipboard".into()) else {
        return;
    };
    let Ok(write) = js_sys::Reflect::get(&clipboard, &"writeText".into()) else {
        return;
    };
    if let Ok(write) = write.dyn_into::<js_sys::Function>() {
        // Falha (permissão, contexto inseguro) é ignorada: copiar é conveniência.
        let _ = write.call1(&clipboard, &text.into());
    }
}
