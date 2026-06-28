# RPC-377 — Client-side viewer parity: Prism, copy/badge, theme toggle, font controls

## Problem

The Rust `viewer_template` (`codelet/attachment-viewer/src/markdown/template.rs`)
emits only a minimal static HTML page: a single hardcoded light stylesheet and
`mermaid.initialize({ startOnLoad: true })`. The original TypeScript viewer ships
a rich client-side layer the Rust port is missing:

- **Prism syntax highlighting** of `<pre class="code-block">` blocks
- **Copy button** + **uppercase language badge** per code block
- **Dark/Light theme toggle** persisted to `localStorage`
- **Font-size controls** (−/+) persisted to `localStorage`
- The themed CSS (CSS custom properties for both themes) those features need

## Source of truth (TypeScript)

- `src/server/templates/viewer-template.ts:33` — `getViewerTemplate()` page shell
- `src/server/templates/viewer-scripts.ts`
  - `getPrismScripts()` (lines ~57-63) — Prism core + autoloader CDN
  - DOMContentLoaded block (lines ~88-117) — per-block language class,
    `Prism.highlightAll()`, Copy button (→ "Copied!" 2s via `navigator.clipboard`),
    language badge
  - theme toggle + localStorage (`fspec-theme`, lines ~119-140)
  - font-size controls 10–24px step 2 default 16 (`fspec-base-font-size`,
    lines ~142-189), updates `--base-font-size` / `--font-scale`, disables
    buttons at the limits
- `src/server/templates/viewer-styles.ts` — `getViewerStyles()`: dark (default)
  + light CSS variables (~19-45), headings/links/inline-code, code-block cards,
  copy button + language badge (~204-217), light-theme Prism token colors
  (~464-518)

### CDN dependencies (match TS exactly)

- Prism core `1.29.0` + `autoloader` plugin; theme CSS `prism-vsc-dark-plus`
- (Mermaid stays as today; its theme should follow the selected light/dark theme —
  the deeper mermaid modal is RPC-378, but the theme variable wiring lands here.)

### Prism language aliasing map (from viewer-scripts.ts ~77-86)

`sh|shell|console → bash`, `js → javascript`, `ts → typescript`, `py → python`,
`rb → ruby`, `yml → yaml`, `text → plaintext`. Apply alias to the `<code>` element's
`language-*` class before `Prism.highlightAll()`.

## Behavior to replicate

1. **Highlighting**: each `<pre class="code-block" data-language="LANG"><code>` gets
   `code.className = "language-" + alias(LANG)`, then `Prism.highlightAll()` runs on
   `DOMContentLoaded`.
2. **Copy button**: top-right of each code block; copies the raw code text via
   `navigator.clipboard.writeText`; label flips to "Copied!" for 2s.
3. **Language badge**: uppercase `data-language` shown as a small badge.
4. **Theme toggle**: 🌙/☀️ button toggles `data-theme`/class on `<html>` or
   `<body>`; persists key `fspec-theme` (`dark`|`light`); applied on initial load
   before first paint; mermaid theme initialized to `dark`/`default` accordingly.
5. **Font controls**: `−` / value / `+`; clamps 10–24px (step 2, default 16);
   persists `fspec-base-font-size`; sets `--base-font-size` and a `--font-scale`;
   disables `−` at 10 and `+` at 24.

## Implementation approach (Rust)

- `template.rs` must stay **< 300 lines**. Split into submodules under
  `markdown/template/`:
  - `mod.rs` — `viewer_template(title, content_html)` assembling head/body
  - `styles.rs` — `const STYLES` (themed CSS variables, code-block cards, badge,
    copy button, light-theme Prism token overrides)
  - `scripts.rs` — Prism loader, highlight+copy+badge, theme toggle, font controls
- Emit the scripts/styles as Rust string constants/`format!` blocks that reproduce
  the TS output. Escaping of dynamic values (only the `<title>`) continues through
  `html_escape`.
- The page body needs the chrome the TS page has: theme-toggle button + font-size
  controls + `.markdown-content` wrapper.

## Acceptance criteria (for Example Mapping → scenarios)

1. The served HTML includes Prism core + autoloader script tags (v1.29) and the
   `prism-vsc-dark-plus` theme stylesheet link.
2. The page includes a DOMContentLoaded script that sets `language-*` classes with
   the alias map and calls `Prism.highlightAll()`.
3. Each rendered code block is accompanied by a Copy button and an uppercase
   language badge in the emitted markup/script.
4. The page includes a theme-toggle control and JS that reads/writes
   `localStorage['fspec-theme']` and applies the theme on load.
5. The page includes font-size controls and JS that reads/writes
   `localStorage['fspec-base-font-size']`, clamps to 10–24, and disables the
   buttons at the bounds.
6. The stylesheet defines both dark (default) and light theme CSS variables.
7. `template.rs` and each new submodule remain under 300 lines.

## Notes / constraints

- This is server-emitted client JS; the Rust tests assert on the **emitted HTML
  string** (presence of script/style/markup, correct localStorage keys, alias map,
  clamp bounds), not on browser execution.
- Do NOT regress existing scenarios in
  `rust-attachment-viewer-server.feature` (markdown render, mermaid `<pre>`, raw
  files, traversal 403, 404, health, start/stop).
- Preserve the fspec.pro axum architecture; no public-API breakage.
- Depends on RPC-376 (anchor ids) landing first.
