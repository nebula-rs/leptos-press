// UnoCSS do leptos-press: gera `style/dist/press.css` a partir dos `.rs` do crate.
// Reusa o preset do leptos-nebula (tema → tokens, ícones lucide) e soma os shortcuts `lxp-*`.
import { defineConfig, presetWind3 } from 'unocss'
import { presetUi } from 'leptos-nebula/preset'
import { shortcuts } from './uno/shortcuts.ts'

export default defineConfig({
  presets: [presetWind3(), presetUi()],
  shortcuts,
})
