# RPC-380 — GFM Footnote-Option Alignment with marked

## Problem

The Rust attachment-viewer enables `Options::ENABLE_FOOTNOTES` in
`codelet/attachment-viewer/src/markdown/render.rs`:

```rust
let mut options = Options::empty();
options.insert(Options::ENABLE_TABLES);
options.insert(Options::ENABLE_STRIKETHROUGH);
options.insert(Options::ENABLE_TASKLISTS);
options.insert(Options::ENABLE_FOOTNOTES);   // <-- this one
```

With this flag on, pulldown-cmark renders GitHub-style footnote markup for
`Text[^1]` + `[^1]: a note` (a `<sup>` reference plus a footnote-definition section).

The TypeScript viewer uses **marked 16.4.2**, which has **no footnote support** — no
`marked-footnote` plugin is installed in `package.json`. marked treats `[^1]` as a plain
link reference and **never emits footnote markup**.

This is a **reverse divergence**: the Rust viewer produces footnote HTML the TS viewer never
generates. To align the Rust renderer's capability set with marked's, footnotes should be
disabled.

## Goal

Remove `Options::ENABLE_FOOTNOTES` so footnote syntax is not given dedicated footnote
rendering, matching marked's lack of footnote support — while keeping the other GFM
extensions (tables, strikethrough, task lists) working exactly as before.

## Approach

1. In `render.rs`, delete the `options.insert(Options::ENABLE_FOOTNOTES);` line.
2. Keep `ENABLE_TABLES`, `ENABLE_STRIKETHROUGH`, `ENABLE_TASKLISTS`.
3. Update the module doc comment to reflect the enabled extension set (remove the footnotes
   mention).
4. Add a test asserting footnote syntax does **not** produce footnote-specific markup
   (no footnote reference element, no footnote-definition section), and confirm the existing
   table/strikethrough/tasklist behavior is unchanged.

## Expected behavior after the change

| Input | Expectation |
| --- | --- |
| `Text[^1]\n\n[^1]: a note` | No `<sup>` footnote ref, no footnote-definition section |
| `\| a \| b \|\n\|---\|---\|\n\| 1 \| 2 \|` | still renders a `<table>` |
| `~~gone~~` | still renders a `<del>` |

> **Note on exact `[^1]` output:** marked's own footnote-less behavior falls back to a
> (somewhat quirky) link-reference. We do NOT attempt to replicate that exact fallback string;
> the acceptance bar for this card is simply that the Rust renderer emits **no footnote-specific
> markup**. Footnote syntax is essentially absent from the real attachment corpus (design docs,
> review findings), so this is a low-risk alignment.

## Files

- **Edit:** `codelet/attachment-viewer/src/markdown/render.rs` (remove one option insert,
  update doc comment).
- **New/extended tests:** `codelet/attachment-viewer/tests/` with `@step` comments mapping to
  `spec/features/markdown-footnote-option-parity.feature`.

## Constraints

- Files stay **under 300 lines**.
- `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` all clean.
- No changes to the axum HTTP layer.

## References

- pulldown-cmark Options docs (ENABLE_FOOTNOTES): https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html
- marked 16 — no built-in footnotes (requires the separate `marked-footnote` plugin, not
  installed here).
