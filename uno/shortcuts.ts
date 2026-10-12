// Shortcuts do leptos-press (prefixo `lxp-`), escritos como literais nos `.rs`.
//
// Mesmas regras do ui (docs/guide/conventions.md): nome nunca começa com prefixo de variante
// do UnoCSS (todos começam com `lxp-`); só tokens (`var(--lx-…)`); a base não define o que uma
// variante também define. A tipografia do Markdown (seletores de elemento) vive em
// `style/prose.css`, não aqui.

type Shortcuts = Record<string, string>

const v = (name: string) => `var(--lx-${name})`
/** Propriedade CSS arbitrária do UnoCSS: `[prop:valor]`. Espaços viram `_`. */
const p = (prop: string, value: string) => `[${prop}:${value.replaceAll(' ', '_')}]`
const sp = (n: string) => v(`space-${n}`)
const fs = (n: string) => p('font-size', v(`font-size-${n}`))
const color = (n: string) => p('color', v(`color-${n}`))
const border = (side: string, token = 'color-border') =>
  p(`border-${side}`, `${v('border-width')} solid ${v(token)}`)

export const shortcuts: Shortcuts = {
  // ── Cabeçalho ─────────────────────────────────────────────────────────────
  // `pl-[sm]` = o padding horizontal da sidebar: o título alinha com o texto dos itens.
  'lxp-brand': `no-underline whitespace-nowrap pl-[${sp('sm')}] ${fs('lg')} ${color('on-surface')} ${p('font-weight', v('font-weight-strong'))}`,
  'lxp-nav': `hidden md:flex items-center gap-[${sp('lg')}]`,
  // Dois shortcuts completos (nunca base + modificador): mesmo peso e tamanho nos dois estados.
  'lxp-nav-link': `no-underline whitespace-nowrap ${fs('sm')} ${p('font-weight', v('font-weight-strong'))} ${color('on-surface')} hover:${color('primary')}`,
  'lxp-nav-link-active': `no-underline whitespace-nowrap ${fs('sm')} ${p('font-weight', v('font-weight-strong'))} ${color('primary')}`,
  // Botão de menu só em tela estreita (a sidebar vira gaveta abaixo de 1024px).
  'lxp-menu-btn': 'flex lg:hidden',

  // ── Página ────────────────────────────────────────────────────────────────
  // Área da página: padding; dentro dela, `lxp-doc-*` (centralizado, largura máxima).
  'lxp-page': `block box-border px-[${sp('lg')}] py-[${sp('xl')}]`,
  // Como o VPDoc do VitePress: coluna de texto ~688px (43rem), aside 224px (14rem), gap 64px.
  'lxp-doc-aside': `grid mx-auto max-w-[61rem] gap-x-[4rem] ${p('grid-template-columns', 'minmax(0,1fr)')} xl:${p('grid-template-columns', 'minmax(0,43rem)_14rem')}`,
  'lxp-doc-plain': 'block mx-auto max-w-[43rem]',
  'lxp-doc-wide': `block mx-auto max-w-[${v('content-max-width')}]`,
  'lxp-content': 'block min-w-0',
  // Outline: some abaixo de 1280px (v0); sticky abaixo do cabeçalho, com rolagem própria.
  'lxp-aside': `hidden xl:block sticky self-start overflow-y-auto box-border top-[calc(${v('header-height')}+${sp('xl')})] max-h-[calc(100vh-${v('header-height')}-${sp('2xl')})]`,
  // Padding horizontal: o fundo do item ativo/hover não toca as bordas da gaveta.
  'lxp-sidebar': `block box-border px-[${sp('sm')}] py-[${sp('sm')}]`,
  'lxp-outline': 'block box-border',
  'lxp-prose': 'block min-w-0',
  'lxp-alert': `block my-[${sp('md')}]`,
  'lxp-alert-title': `${p('font-weight', v('font-weight-strong'))} mb-[${sp('xs')}]`,

  // ── Bloco de código ───────────────────────────────────────────────────────
  // Sem barra: linguagem e botão de copiar flutuam no canto (como o VitePress).
  'lxp-code': [
    'relative block overflow-hidden box-border',
    `my-[${sp('md')}] rounded-[${v('radius-md')}]`,
    p('background', v('code-bg')),
    p('border', `${v('border-width')} solid ${v('color-border')}`),
  ].join(' '),
  // Dentro do ExampleBox: encostado na base da caixa, só com a borda de cima.
  'lxp-code-attached': [
    'relative block overflow-hidden box-border m-0 rounded-none',
    p('background', v('code-bg')),
    border('top'),
  ].join(' '),
  'lxp-code-tools': `absolute z-10 flex items-center top-[${sp('xs')}] right-[${sp('xs')}] gap-[${sp('xs')}]`,
  'lxp-code-lang': `select-none ${fs('xs')} ${color('on-surface-muted')}`,
  // Aparece no hover do bloco e no foco por teclado; sem hover (toque), sempre visível.
  'lxp-code-copy': [
    'opacity-0 transition-opacity',
    'group-hover:opacity-100 group-focus-within:opacity-100 focus-within:opacity-100',
    '[@media(hover:none)]:opacity-100',
  ].join(' '),
  'lxp-pre': [
    'block m-0 box-border overflow-x-auto',
    `py-[${sp('md')}] pl-[${sp('md')}] pr-[4rem]`,
    p('font-family', v('font-mono')),
    fs('sm'),
    p('line-height', '1.6'),
    color('code-text'),
  ].join(' '),

  // ── Exemplo vivo (ExampleBox) ─────────────────────────────────────────────
  'lxp-example': `block my-[${sp('md')}]`,
  'lxp-example-title': `block mb-[${sp('sm')}] ${fs('sm')} ${color('on-surface-muted')} ${p('font-weight', v('font-weight-strong'))}`,
  // Palco: `row` (inline, ex.: Btn) ou `column` (bloco, ex.: Banner) — escolhido por exemplo.
  'lxp-example-stage-row': `flex flex-wrap items-center gap-[${sp('sm')}]`,
  'lxp-example-stage-column': `flex flex-col items-stretch gap-[${sp('sm')}]`,

  // ── Anterior / próxima ────────────────────────────────────────────────────
  'lxp-prev-next': `grid grid-cols-2 gap-[${sp('md')}] mt-[${sp('xl')}]`,
  'lxp-prev': 'flex justify-start',
  'lxp-next': 'flex justify-end',

  // ── Home ──────────────────────────────────────────────────────────────────
  'lxp-hero': `flex flex-col items-center text-center box-border gap-[${sp('md')}] py-[${sp('2xl')}]`,
  'lxp-hero-name': `${p('font-size', `calc(${v('font-size-xl')}*2)`)} ${color('primary')} ${p('line-height', '1.1')}`,
  'lxp-hero-text': fs('xl'),
  'lxp-hero-tagline': `${fs('lg')} ${color('on-surface-muted')}`,
  'lxp-hero-actions': `flex flex-wrap justify-center gap-[${sp('sm')}] mt-[${sp('md')}]`,
  'lxp-features': `grid grid-cols-1 md:grid-cols-3 gap-[${sp('md')}] my-[${sp('xl')}]`,
  'lxp-feature-icon': `${fs('xl')} mb-[${sp('sm')}]`,
  'lxp-feature-title': `${fs('md')} ${p('font-weight', v('font-weight-strong'))} mb-[${sp('xs')}]`,
  'lxp-feature-details': `m-0 ${fs('sm')} ${color('on-surface-muted')}`,

  // ── 404 ───────────────────────────────────────────────────────────────────
  'lxp-not-found': `flex flex-col items-center text-center gap-[${sp('md')}] py-[${sp('2xl')}]`,
}
