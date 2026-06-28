# RPC-376 — Heading anchor IDs + GFM render-option parity (Rust markdown viewer)

## Problem

`codelet/attachment-viewer/src/markdown/render.rs` renders GFM markdown via
`pulldown-cmark` but **does not emit `id` attributes on headings**. The original
TypeScript viewer (`src/server/utils/markdown-renderer.ts`) registers the
`marked-gfm-heading-id` plugin (`marked.use(gfmHeadingId())`), which slugifies
every `h1`–`h6` into a GitHub-compatible anchor ID. Author-written links such as
`[Jump to summary](#summary)` therefore **resolve in the TS viewer but silently
break in the Rust viewer**.

This was already filed against the TS viewer as
`spec/features/anchor-links-not-working-in-markdown-attachment-viewer.feature`.
RPC-376 brings the **Rust port** to the same behavior.

## Source of truth (TypeScript behavior to replicate)

- `src/server/utils/markdown-renderer.ts:30` — `marked.use(gfmHeadingId())`
- Tests: `src/server/utils/__tests__/markdown-renderer.test.ts:36-127`

### Slug algorithm (GitHub-style, must match `marked-gfm-heading-id`)

Given a heading's **text content** (after inline markdown is stripped to plain
text):

1. Lowercase the whole string.
2. Remove characters that are not alphanumeric, space, or hyphen
   (e.g. `?`, `!`, `'`, `.`, `:`, `()` are dropped). Apostrophe in `What's` →
   `whats`.
3. Replace runs of whitespace with a single hyphen `-`.
4. De-duplicate: the first occurrence of a slug is used as-is; subsequent
   identical slugs get `-1`, `-2`, … suffixes (per-document counter).

### Verified examples (from the TS tests)

| Heading text                     | Generated id                      |
| -------------------------------- | --------------------------------- |
| `Summary`                        | `summary`                         |
| `Summary` (2nd occurrence)       | `summary-1`                       |
| `Summary` (3rd occurrence)       | `summary-2`                       |
| `Domain-to-Tag Mapping Rules`    | `domain-to-tag-mapping-rules`     |
| `What's New?`                    | `whats-new`                       |

Anchor link round-trip: `[Jump to summary](#summary)` must keep `href="#summary"`
AND a heading `## Summary` must render `id="summary"`, so the browser anchor jump
works.

## Implementation approach (Rust)

- In `render.rs`, headings currently flow through the `passthrough` buffer into
  `pulldown_cmark::html::push_html`. That path does **not** add ids.
- Intercept `Tag::Heading` start/end the same way code blocks are intercepted:
  capture the heading level + accumulate its text, compute the slug, and emit
  `<h{level} id="{slug}">…inner…</h{level}>` yourself, OR run a post-process step.
- Prefer a dedicated `slug.rs` helper (`fn slugify(text: &str, seen: &mut HashMap<String,u32>) -> String`)
  so it is independently unit-testable and matches the table above exactly.
- Keep all files **< 300 lines**. Add `mod slug;` under `markdown/`.

## GFM render-option reconciliation (secondary)

The two renderers currently diverge on options:

- **TS**: `marked({ gfm: true, breaks: true })` → single `\n` becomes `<br>`.
- **Rust**: enables `ENABLE_SMART_PUNCTUATION` (curly quotes/dashes) which TS does
  **not**, and does **not** emulate `breaks: true`.

Decision for this card:

- **Remove `ENABLE_SMART_PUNCTUATION`** from `render.rs` — TS does not smart-quote,
  and code/identifiers must stay literal. This is the higher-risk divergence.
- `breaks: true` (soft-break → `<br>`) — pulldown-cmark has no direct option;
  replicate by mapping `Event::SoftBreak` to a hard break (`<br>\n`) in the
  passthrough handling. Confirm against a fixture (`line1\nline2` → `line1<br>line2`).
- Keep `ENABLE_TABLES`, `ENABLE_STRIKETHROUGH`, `ENABLE_TASKLISTS`,
  `ENABLE_FOOTNOTES` (footnotes are a Rust bonus and stay).

## Acceptance criteria (for Example Mapping → scenarios)

1. A heading `## Summary` renders with `id="summary"`.
2. Multi-word heading slugs hyphenate and lowercase
   (`Domain-to-Tag Mapping Rules` → `id="domain-to-tag-mapping-rules"`).
3. Special characters are stripped (`What's New?` → `id="whats-new"`).
4. Duplicate headings get numbered suffixes (`summary`, `summary-1`, `summary-2`).
5. An anchor link `[Jump to summary](#summary)` keeps `href="#summary"` and the
   target heading id matches, enabling in-page navigation.
6. A soft line break inside a paragraph renders as `<br>` (breaks parity).
7. Smart punctuation is NOT applied (a straight apostrophe stays `&#39;`/`'`, not a
   curly quote).

## Out of scope

- Auto-generated table-of-contents list (TS doesn't generate one either — only
  per-heading ids).
- Prism, theme, fonts, mermaid modal (RPC-377 / RPC-378).

## Constraints

- Preserve the fspec.pro axum architecture and existing public API of the crate.
- No `unwrap()`/`expect()` in request-path code; pure helpers may use total logic.
- All files < 300 lines; add focused unit tests for `slugify`.
