# AST Research — template module (RPC-378)

Tool: AstGrep (rust)

## Public string constants in `src/markdown/template/`

Pattern: `pub const $NAME: &str = $VAL;`

- `template/scripts.rs:11` — `pub const HEAD_SCRIPTS: &str` (mermaid loader + Prism tags). Will be UPDATED to add `fontFamily: 'monospace'`, flowchart opts, and `mermaid.run()`.
- `template/scripts.rs:29` — `pub const INTERACTION_SCRIPT: &str` (copy/badge/theme/font). Unchanged.
- `template/styles.rs:9` — `pub const STYLES: &str` (themed CSS). Unchanged; modal CSS goes to a NEW `modal_styles::MODAL_STYLES`.

## Public entry point

Pattern: `pub fn viewer_template($$$ARGS) -> String { $$$BODY }`

- `template/mod.rs:19` — `pub fn viewer_template(title: &str, content_html: &str) -> String`.
  Signature MUST remain stable. New modal markup, Panzoom CDN, modal script, and
  `MODAL_STYLES` will be injected through this single assembler.

## Conclusion

The interactivity is emitted purely as static `&str` consts assembled by
`viewer_template`. RPC-378 adds two new submodules (`modal_styles.rs`,
`mermaid_modal.rs`) plus an edit to `HEAD_SCRIPTS`, keeping every file < 300
lines and the public API unchanged.
