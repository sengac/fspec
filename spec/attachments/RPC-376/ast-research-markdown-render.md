# AST research — RPC-376 markdown render path

Scope: `codelet/attachment-viewer/src/markdown/`

## Existing functions returning String (render path)

- `render.rs:69` `fn render_code_block(lang: &str, code: &str) -> String` — emits
  mermaid / code-block HTML. Confirms the "intercept a tag, render it ourselves"
  pattern already used for `Tag::CodeBlock`.

## Event interception pattern (render.rs:32-54)

`render_markdown` iterates `Parser::new_ext` events and matches:
- `Event::Start(Tag::CodeBlock(kind))` → flush passthrough, capture lang.
- `Event::End(TagEnd::CodeBlock)` → emit `render_code_block`.
- `Event::Text|Code` while inside a code block → accumulate into `code_buf`.
- everything else → `passthrough` buffer flushed via `pulldown_cmark::html::push_html`.

This is the exact hook point to add `Tag::Heading` interception (accumulate inner
events + plain text, emit `<hN id="slug">…</hN>`), and to map `Event::SoftBreak`
to a hard `<br>` for breaks parity.

## Options (render.rs:16-21)

Currently enables TABLES, STRIKETHROUGH, TASKLISTS, FOOTNOTES, SMART_PUNCTUATION.
RPC-376 removes SMART_PUNCTUATION (TS marked does not smart-quote).

## Conclusion

No public API change to the crate; `mod slug;` added under `markdown/`, and
`render.rs` heading/softbreak handling extends the existing event loop. `slug.rs`
is independently unit-testable per the verified examples table.
