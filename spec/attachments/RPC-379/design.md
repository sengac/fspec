# RPC-379 — Bare-URL and Email Autolink Rendering Parity

## Problem

The TypeScript viewer (`src/server/utils/markdown-renderer.ts`) renders markdown with
**marked 16.4.2** configured with `gfm: true`. GFM autolink-literal support means bare URLs
and email addresses written as plain text become clickable links automatically:

| Input (plain text) | marked output |
| --- | --- |
| `See https://example.com for details` | `See <a href="https://example.com">https://example.com</a> for details` |
| `Email a@b.com please` | `Email <a href="mailto:a@b.com">a@b.com</a> please` |

The Rust port (`codelet/attachment-viewer/src/markdown/render.rs`) uses **pulldown-cmark
0.12.2**, which has **no autolink-literal support at all**. Its `Options::ENABLE_GFM` flag —
despite the name — only enables GitHub blockquote **alert tags** (`[!NOTE]`, `[!WARNING]`, …),
NOT autolinks. The maintainer confirms in
[issue #494](https://github.com/pulldown-cmark/pulldown-cmark/issues/494):

> "This is not supported by pulldown directly, but since this is essentially an exercise in
> text search, it's relatively straightforward to build directly on top of the library."

Result: bare URLs in attachment markdown render as **inert plain text** in the Rust viewer.
This is a genuine functional rendering divergence — a clickable link in the TS viewer is dead
text in the Rust viewer.

## Goal

Make the Rust `render_markdown` autolink bare `http://` / `https://` URLs and bare email
addresses, matching marked's GFM autolink-literal output for the common cases, **without
changing the fspec.pro axum HTTP architecture** (this card touches only markdown rendering).

## Approach (architecture)

pulldown-cmark is a pull parser emitting an `Event` stream. The current `render.rs` already
intercepts the stream (code blocks, headings, soft breaks) and flushes non-intercepted events
through `pulldown_cmark::html::push_html`. We layer autolinking as an **event transform** on
the passthrough buffer:

1. **New module `markdown/autolink.rs`** (< 300 lines) exposing a function that takes the
   passthrough `Vec<Event>` and returns a new `Vec<Event>` where each eligible `Event::Text`
   is split into a sequence of `Text` + `Start(Tag::Link)` + `Text` + `End(TagEnd::Link)` +
   `Text …` events for each detected URL/email.

2. **Apply inside `flush_passthrough` BEFORE `push_html`** so the synthesized link events are
   serialized by pulldown's own HTML writer. This keeps HTML escaping of the link text and
   destination identical to surrounding text (no second escaper to drift).

3. **Skip-context tracking** so we never autolink where we shouldn't:
   - Track `Tag::Link` nesting depth across the buffer; while depth > 0, leave `Event::Text`
     untouched (rule: never autolink inside an existing link → no nested `<a>`).
   - Only ever transform `Event::Text`. Inline code arrives as a single `Event::Code`
     (a non-`Text` event) and is therefore naturally skipped — a URL inside `` `…` `` stays
     literal.

4. **Headings**: heading inner text is rendered separately via `HeadingState::render`. Keep
   headings as-is (do NOT autolink inside headings) for simplicity; the slug text stays plain.
   Document this as the chosen behavior.

## URL / email detection rules (match marked / GFM autolink literal)

- Schemes: `http://` and `https://`. (`www.`-prefixed and `ftp://` are out of scope for this
  card; document as a known, acceptable narrowing.)
- Email: `local-part@domain` where the domain contains at least one dot.
- **Trailing-punctuation trimming** (GFM autolink literal): characters in the set
  `< > ? ! . , : ; * _ ~` immediately trailing the URL are excluded from the link.
- **Unbalanced trailing `)`**: if the URL ends with `)` and the parentheses within the URL are
  unbalanced, trim the trailing `)` out of the link (GFM paren rule).
- Emit `Tag::Link { link_type: LinkType::Autolink (or Inline), dest_url, title: "", id: "" }`
  with `dest_url` = the URL (for emails, prefix `mailto:`).

### Worked expectations

| Input | Anchor href | Anchor text | Notes |
| --- | --- | --- | --- |
| `https://example.com` | `https://example.com` | `https://example.com` | base case |
| `http://example.com` | `http://example.com` | `http://example.com` | http scheme |
| `a@b.com` | `mailto:a@b.com` | `a@b.com` | email → mailto |
| `https://example.com.` | `https://example.com` | `https://example.com` | trailing `.` excluded |
| `[label](https://example.com)` | `https://example.com` | `label` | already a link → not double-linked |
| `` `https://x.com` `` | — | — | inside code span → not linked |

## Files

- **New:** `codelet/attachment-viewer/src/markdown/autolink.rs`
- **Edit:** `codelet/attachment-viewer/src/markdown/render.rs` (call transform in
  `flush_passthrough`)
- **Edit:** `codelet/attachment-viewer/src/markdown/mod.rs` (declare `mod autolink;` if needed)
- **New tests:** add to `codelet/attachment-viewer/tests/` (e.g. `markdown_autolink.rs`) with
  `@step` comments mapping to `spec/features/markdown-autolink-literals.feature`.

## Constraints

- Every Rust source file stays **under 300 lines**.
- `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` all clean.
- No changes to the axum HTTP layer (`build_router`, `ViewerState`, handlers, Cors/Trace).
- No `unwrap()`/`expect()` on fallible paths in production code; use `?`/pattern matching.

## Out of scope (documented narrowings)

- `www.`-prefixed autolinks and non-http(s) schemes (marked supports `www.`; we narrow to
  http/https for this card — note as a follow-up if needed).
- Byte-for-byte serialization differences that render identically in a browser
  (e.g. void-element self-closing slash) are NOT part of this card.

## References

- pulldown-cmark issue #494 (autolinks not built in): https://github.com/pulldown-cmark/pulldown-cmark/issues/494
- GFM spec, autolink literals: https://github.github.com/gfm/#autolinks-extension-
- marked 16.4.2 (TS dependency producing the target output)
