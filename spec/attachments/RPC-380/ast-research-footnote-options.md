# AST Research — RPC-380 Footnote Option Alignment

## Scope
`codelet/attachment-viewer/src/markdown/render.rs`

## Query 1: pulldown-cmark Options inserts

Pattern: `options.insert($OPT)` (rust)

Results:
- `render.rs:22` — `options.insert(Options::ENABLE_TABLES)`
- `render.rs:23` — `options.insert(Options::ENABLE_STRIKETHROUGH)`
- `render.rs:24` — `options.insert(Options::ENABLE_TASKLISTS)`
- `render.rs:25` — `options.insert(Options::ENABLE_FOOTNOTES)`  ← TO REMOVE

The Options set is constructed in exactly one place. Removing the
`ENABLE_FOOTNOTES` insert (line 25) is the only behavioural change needed;
tables/strikethrough/tasklists stay.

## Query 2: render entry point

Pattern: `pub fn render_markdown($$$ARGS) -> String { $$$BODY }` (rust)

Results:
- `render.rs:18` — `pub fn render_markdown(markdown: &str) -> String`

There is a single public render entry point. No other call site builds a
pulldown-cmark Options struct, so no axum/HTTP-layer change is required.

## Conclusion
Single-line removal at render.rs:25 plus a module doc-comment update. Add an
integration test asserting footnote syntax emits no footnote-specific markup
and verify tables/strikethrough remain intact.
