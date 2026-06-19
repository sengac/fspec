# RPC-339 — Refit SearchHistoryView onto the shared full-screen shell

**Split out of RPC-337.** RPC-337 extracts the shared full-screen shell
(`render_full_screen_scaffold` in `views/full_screen_shell.rs`) and
refits **ProviderSettingsView** and **ResumeSessionView** onto it. The
third hand-rolled scaffold — **SearchHistoryView** — was deferred here
because its title region is structurally incompatible with the shell's
current title contract.

## Why this was deferred from RPC-337

The shell renders the title via `render_title_with_count(area, buf,
title, count, suffix)` → `"Search History (N matches)"`.

But `SearchHistoryView`'s title is an **editable query input**, not a
count label (`views/agent/search_history_view_render.rs:28-36`):

```text
(search): <live query><inverse-cursor block>
```

Forcing SearchHistory through `render_title_with_count` would **remove
the visible query editor** — a functional UX regression, not a mere
snapshot re-baseline. So RPC-337's rule [5] (snapshot parity + refit)
was scoped to Provider + Resume only.

## Scope of this card

### 1. Generalize the shell title region
Add a title-renderer variant to `views/full_screen_shell.rs` so the
title region can be painted by a caller-supplied closure, e.g.:

```rust
pub(crate) fn render_full_screen_scaffold_with_title<T, B>(
    area: Rect,
    buf: &mut Buffer,
    title_fn: T,          // FnOnce(Rect, &mut Buffer) — paints title row
    footer_hint: &str,
    body_fn: B,           // FnOnce(Rect, &mut Buffer) — paints body
    overlay: Option<&ConfirmDialog>,
) where T: FnOnce(Rect, &mut Buffer), B: FnOnce(Rect, &mut Buffer)
```

Keep the existing `render_full_screen_scaffold(... title, count,
suffix ...)` convenience wrapper for Provider/Resume/ModelSelector
(implement it in terms of the title-closure variant to avoid
duplication). This satisfies RPC-337 rule [4] ("title row MUST support
the `{title} ({count} {suffix})` format") — the capability is preserved
as the common-case wrapper.

### 2. Refit SearchHistoryView
- Replace `SearchHistoryView::render`'s hand-rolled `Clear` +
  4-constraint `Layout` (`search_history_view.rs:243-257`) with a call
  to the title-closure shell variant.
- The title closure calls the existing `render_title` (editable query
  + cursor); the body closure calls `render_body`; footer stays the
  static hint string.
- SearchHistoryView has no destructive action → `overlay = None`.
- Keep `search_history_view.rs` and `search_history_view_render.rs`
  under 300 LoC (source-shape rule pinned by
  `tests/rpc026_source_shape.rs`).

### 3. Re-baseline snapshots
Any insta snapshots covering SearchHistory render output get reviewed +
re-accepted (the surrounding Clear/split structure is unchanged, so
output should stay byte-identical; confirm and re-baseline if the
extraction shifts anything).

## Acceptance criteria to capture during Example Mapping (seed)

- **Rule:** The shell exposes a title-renderer variant accepting a
  caller-supplied title closure; the count-title wrapper is implemented
  on top of it.
- **Rule:** SearchHistoryView renders via the shell's title-closure
  variant, preserving its editable-query title + inverse cursor.
- **Rule:** SearchHistoryView's `render` no longer hand-rolls `Clear` +
  the 4-constraint Layout (it delegates to the shell).
- **Example:** Typing into the search palette still shows the live
  query in the title row after the refit.
- **Example:** SearchHistory body + footer render identically to the
  pre-refit baseline.

## References
- Shell: `codelet/fspec-tui/src/views/full_screen_shell.rs` (RPC-337).
- SearchHistory render: `views/agent/search_history_view.rs:243-257`,
  `views/agent/search_history_view_render.rs`.
- Parent card: RPC-337. This card `dependsOn` RPC-337.
