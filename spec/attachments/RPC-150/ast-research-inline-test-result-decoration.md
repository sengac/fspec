# RPC-150 — Inline test_result decoration on focused/matching Provider row (AST Research)

## Scope

Regression-shape coverage card. Mirrors the RPC-089 / RPC-090 / RPC-151
/ RPC-152 / RPC-153 / RPC-155 pattern: pin the canonical render
invocation point so the RPC-072 stub state (test result hidden inside
the now-removed `Detail::Summary` view) cannot silently re-emerge in
list-mode rendering.

Implementation already exists in
`codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs`.
This card creates the regression-shape tests + feature file that pin
the call-site structure in source.

## Canonical structures

### `codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs`

**`render_nav_items` per-row decoration call (lines 60-73):**
```rust
// RPC-158: paint the inline test-result decoration on Provider
// header rows whose provider_id matches `view.test_result`. The
// decoration is appended after the label with a single ASCII
// space separator; foreground comes from the status, background
// matches the row's existing colour band.
if matches!(kind, RowKind::Provider { .. }) {
    if let Some(test_result) = view.test_result.as_ref() {
        if test_result.provider_id == item.provider_id {
            paint_test_result_decoration(
                kind, selected, &test_result.status, row_area, end_x, buf,
            );
        }
    }
}
```

**Invariants:**
- `render_nav_items` body reads `view.test_result.as_ref()` inside the
  per-row loop (`for (row_idx, item) in nav_items[...]`).
- Decoration paint is gated by `matches!(kind, RowKind::Provider { .. })`
  — only Provider rows can ever carry the ladder.
- Decoration paint is gated by `test_result.provider_id == item.provider_id`
  — non-matching Provider rows do not show the ladder.
- The `matches!(kind, ...)` gate appears textually BEFORE the
  `test_result.provider_id == item.provider_id` gate.

**`paint_test_result_decoration` definition (lines 77-106):**
```rust
fn paint_test_result_decoration(
    kind: RowKind,
    selected: bool,
    status: &super::ProviderTestStatus,
    row_area: Rect,
    end_x: u16,
    buf: &mut Buffer,
) {
    let right_bound = row_area.x.saturating_add(row_area.width);
    if end_x >= right_bound {
        return;
    }
    let separator_x = end_x;
    let decoration_x = end_x.saturating_add(1);
    if decoration_x >= right_bound {
        return;
    }
    let (text, fg) = status.decoration();
    let bg = row_band_bg(kind, selected);
    let style = Style::default().fg(fg).bg(bg);
    let remaining = (right_bound - decoration_x) as usize;
    buf[(separator_x, row_area.y)].set_symbol(" ");
    buf[(separator_x, row_area.y)].set_style(Style::default().bg(bg));
    buf.set_stringn(decoration_x, row_area.y, &text, remaining, style);
}
```

**Invariants:**
- Exactly one `fn paint_test_result_decoration(` definition in the
  file (canonical home: list_nav_render.rs).
- Exactly one call site `paint_test_result_decoration(\n` (multi-line
  call inside the per-row decoration gate).
- Signature has the canonical 6 parameter declarations: `kind: RowKind,`,
  `selected: bool,`, `status: &super::ProviderTestStatus,`,
  `row_area: Rect,`, `end_x: u16,`, `buf: &mut Buffer,`.
- Body computes `right_bound = row_area.x.saturating_add(row_area.width)`,
  early-returns on `end_x >= right_bound`.
- Reserves the separator cell (`separator_x = end_x`), advances by one
  (`decoration_x = end_x.saturating_add(1)`), early-returns again on
  `decoration_x >= right_bound`.
- Foreground from `status.decoration()`, background from
  `row_band_bg(kind, selected)`, composed via
  `Style::default().fg(fg).bg(bg)`.

### Exclusion check

`paint_test_result_decoration` must be exclusively owned by
`list_nav_render.rs`. It must NOT appear in:

- `codelet/fspec-tui/src/views/provider_settings/detail.rs` — Detail
  surface is being removed (RPC-103 done), and test result must not
  reappear there as a recovery path.
- `codelet/fspec-tui/src/views/provider_settings/row_render.rs` —
  pure row painter; decorations belong one layer up at the paint loop.

## AST queries used

- `pattern='fn $NAME($$$ARGS) { $$$BODY }'` on
  `list_nav_render.rs` confirmed `paint_test_result_decoration` as a
  free function with a multi-line signature.
- `pattern='paint_test_result_decoration($$$ARGS)'` on the same file
  confirmed exactly one call site at line 68.

## Source-string assertions (no runtime render needed)

The card pins the canonical structure via byte-level grep over the
single source file plus exclusion checks against `detail.rs` /
`row_render.rs`. No ratatui `Buffer` / `Frame` needs to be constructed
— we are not running the renderer, we are pinning that the inline
ladder paint site remains wired into the list-mode row loop.

## Tests

- `codelet/fspec-tui/tests/rpc150_inline_test_result_render_shape.rs`
  — ~7 sub-millisecond source-string tests.

## Feature

- `spec/features/provider-settings-list-inline-testresult-rendering-on-focused-row.feature`
  — 7 scenarios pinning the invariants above.
