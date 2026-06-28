# AST Research — RPC-379 Autolink Literals

## Scope
`codelet/attachment-viewer/src/markdown/` (render.rs, escape.rs, mod.rs)

## Query 1: passthrough flush function

Pattern: `fn flush_passthrough($$$ARGS) { $$$BODY }` (rust)

Result:
- `render.rs:123` — `fn flush_passthrough(buffer: &mut Vec<Event>, out: &mut String)`

This is the single place non-intercepted events are serialized to HTML. The
autolink transform must run here on the `buffer` BEFORE `push_html`, so the
synthesized link events are serialized by pulldown's own writer (consistent
escaping).

## Query 2: push_html call sites

Pattern: `pulldown_cmark::html::push_html($$$ARGS)` (rust)

Results:
- `render.rs:116` — `push_html(&mut inner, self.events.into_iter())` — HeadingState::render (separate path; headings left as-is, NOT autolinked)
- `render.rs:128` — `push_html(out, events.into_iter())` — flush_passthrough (the path we transform)

## Conclusion
- New module `markdown/autolink.rs`: `fn autolink_events(Vec<Event>) -> Vec<Event>`.
- Wire it into `flush_passthrough` right before the `push_html` at line 128.
- Heading inner text uses a separate push_html (line 116) → headings stay plain.
- Inline code arrives as `Event::Code` (non-Text) → naturally skipped.
- Track `Tag::Link` nesting depth → never autolink inside existing links.
- escape.rs `html_escape` unchanged; pulldown's writer handles link-text escaping.
- No axum/HTTP-layer code is touched.
