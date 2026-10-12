// Gera style/dist/press.css (versionado). Ordem:
// prose.css (tipografia do Markdown + realce) → utilitários/shortcuts usados em src/**/*.rs.
// Sem reset nem tokens: isso vem do ui.css (UiStyles carrega antes de PressStyles).
import { createGenerator } from 'unocss'
import { existsSync, readFileSync, writeFileSync, mkdirSync, readdirSync, statSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import config from '../uno.config.ts'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const read = (p) => readFileSync(join(root, p), 'utf8').trim()

function walk(dir) {
  return readdirSync(dir).flatMap((n) => {
    const f = join(dir, n)
    return statSync(f).isDirectory() ? walk(f) : f.endsWith('.rs') ? [f] : []
  })
}

const uno = await createGenerator(config)
const code = walk(join(root, 'src')).map((f) => readFileSync(f, 'utf8')).join('\n')
const utilities = (await uno.generate(code, { preflights: false })).css

const out = [
  '/* GERADO por scripts/build-css.mjs — não editar à mão. */',
  read('style/prose.css'),
  utilities,
].join('\n\n') + '\n'

const target = join(root, 'style/dist/press.css')
if (process.argv.includes('--check')) {
  // css-fresh: falha se o arquivo versionado não é o que o build gera agora.
  const current = existsSync(target) ? readFileSync(target, 'utf8') : ''
  if (current !== out) {
    console.error('✗ style/dist/press.css desatualizado — rode `moon run leptos-press:css`')
    process.exit(1)
  }
  console.log('✓ style/dist/press.css em dia')
} else {
  mkdirSync(join(root, 'style/dist'), { recursive: true })
  writeFileSync(target, out)
  console.log(`✓ style/dist/press.css (${out.length} bytes)`)
}
