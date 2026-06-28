# AST research — RPC-377 viewer template

Scope: `codelet/attachment-viewer/src/markdown/`

## Current public surface

- `template.rs:25` `pub fn viewer_template(title: &str, content_html: &str) -> String`
  — single function, re-exported from `markdown/mod.rs` as
  `pub use template::viewer_template;`. RPC-377 must preserve this exact
  signature while splitting the module into `markdown/template/{mod,styles,scripts}.rs`.

## How it is called

- `mod.rs` re-exports `viewer_template`; the HTTP view handler renders `.md`
  files via `render_markdown` + `viewer_template`. No call-site change needed —
  only the internals of the template grow (Prism scripts, themed CSS, theme +
  font controls JS).

## Escaping

- Only the dynamic `<title>` flows through `html_escape`. The Prism/theme/font
  scripts and styles are static server-emitted strings, so they are written as
  Rust `const`/`format!` blocks verbatim (matching the TS output).

## Tests assert on the emitted string

- `tests/markdown_and_path.rs` already asserts on `viewer_template(...)` output
  (mermaid script, `.markdown-content`, escaped title). New tests extend this:
  Prism 1.29 tags, alias map, copy/badge JS, `fspec-theme` /
  `fspec-base-font-size` keys, clamp bounds 10/24, `:root` + `:root.light-theme`.

## Conclusion

No public-API or axum-architecture change. Module split keeps every file < 300
lines; `viewer_template` remains the single assembler entry point.
