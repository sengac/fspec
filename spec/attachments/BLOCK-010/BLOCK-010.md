# BLOCK-010 — BlocklistView keyboard parity (remove vim j/k; add PageUp/PageDown/Home/End)

## Summary

The Rust TUI `/blocklist` view (`codelet/fspec-tui/src/views/blocklist/`) is the
**only** ported full-screen scroll view that:

1. Accepts **vim-style `j`/`k`** navigation keys, and
2. **Advertises them** in its footer (`↑↓/jk: Navigate | ...`), while
3. Having **NO `PageUp`/`PageDown`/`Home`/`End`** support at all — navigation is
   strictly single-step (`move_up`/`move_down`).

The two canonical reference scroll views use **arrow keys + Page/Home/End only**
and never bind vim keys. This card brings `/blocklist` to parity.

## Root cause (exact locations)

### 1. Vim `j`/`k` bindings — `views/blocklist/mod.rs`

`BlocklistView::handle_key` (lines **150–182**) contains a `KeyCode::Char(c)`
arm that maps vim keys onto navigation:

```rust
KeyCode::Char(c) => match c {
    'j' | 'J' => { self.move_down(); BlocklistEvent::Consumed }   // lines 168–171 — VIM
    'k' | 'K' => { self.move_up();   BlocklistEvent::Consumed }   // lines 172–175 — VIM
    ' '       => self.toggle_focused(),                            // line 176 — KEEP (Space toggle)
    _         => BlocklistEvent::Consumed,                          // line 177
},
```

There are **no** `KeyCode::PageUp`, `KeyCode::PageDown`, `KeyCode::Home`, or
`KeyCode::End` arms — those key events fall through to the `_ => Consumed`
catch-all at line 180 and do nothing.

Movement helpers today (lines **184–196**) are single-step only:
`move_down` (184–189), `move_up` (191–196). There is no paging helper.

### 2. Footer advertises vim — `views/blocklist/render.rs`

```rust
// line 24
const FOOTER_HINT: &str = "↑↓/jk: Navigate | Enter/Space: Toggle Rule | Esc: Close";
```

Referenced at line **51** (passed into `render_full_screen_scaffold`).

## Reference / canonical behaviour (what we mirror)

### model_selector — `views/model_selector/dispatch.rs` `handle_key` (lines 32–182)

| Key | Line(s) | Action |
|-----|---------|--------|
| `Up` | 69–72 | `move_up()` |
| `Down` | 73–76 | `move_down()` |
| `Home` | 77–81 | anchor first + `adjust_scroll()` |
| `PageDown` | 82–85 | `page_down()` |
| `PageUp` | 86–89 | `page_up()` |
| `End` | 90–96 | last selectable + `adjust_scroll()` |

**No `Char('j')` / `Char('k')` arms exist.**

Paging helpers — `views/model_selector/navigation.rs` `page_down` (50–63) /
`page_up` (65–80): step = `visible_rows.max(1)`, loop the per-row clamp mover
`step` times, then `adjust_scroll()`.

### provider_settings — `views/provider_settings/list.rs` `handle_list_key`

`PageDown` (56) / `PageUp` (57) / `Home` (58) / `End` (59) / `Up` (69–81) /
`Down` (82–91). No vim keys. Footer strings never mention `jk`.

### Shared viewport primitive — `components/scroll_viewport.rs`

`ensure_visible(&mut offset, selected, visible_rows, len)` keeps the selection
inside the window. Blocklist already calls it via `adjust_scroll()`
(`mod.rs:128–135`). Paging must call `adjust_scroll()` after mutating
`selected_index` so `scroll_offset` reconciles.

## Fix direction

All changes are confined to the blocklist view (2 files):

### A. `views/blocklist/mod.rs`

1. **Remove** the `'j' | 'J'` and `'k' | 'K'` arms from the `KeyCode::Char(c)`
   match (lines 168–175).
2. **Preserve** the Space-toggle. Cleanest: replace the whole
   `KeyCode::Char(c) => match c { ... }` block with an explicit
   `KeyCode::Char(' ') => self.toggle_focused()` arm plus a
   `KeyCode::Char(_) => BlocklistEvent::Consumed` no-op arm.
3. **Add** four new arms after `Up`:
   - `KeyCode::PageDown => { self.page_down(); Consumed }`
   - `KeyCode::PageUp   => { self.page_up();   Consumed }`
   - `KeyCode::Home     => { self.jump_top();  Consumed }`
   - `KeyCode::End      => { self.jump_bottom(); Consumed }`
4. **Add** the four helpers (mirror model_selector semantics, but blocklist rows
   are all selectable — no header-skipping needed):
   - `page_down`: `let step = self.visible_rows.max(1); for _ in 0..step { self.move_down(); }`
     (or clamp `selected_index` by `step` then `adjust_scroll()` — either is fine
     since every row is selectable). Must not overrun `rules.len()-1`.
   - `page_up`: symmetric toward 0.
   - `jump_top`: `self.selected_index = 0; self.adjust_scroll();`
   - `jump_bottom`: `self.selected_index = self.rules.len().saturating_sub(1); self.adjust_scroll();`
   - All must be no-ops on an empty list (`rules.is_empty()`).
5. Update the module doc-comment (lines 8–11) that currently says
   "Pressing `j`/`k` (or arrows) navigates" to reflect arrows + Page/Home/End.

### B. `views/blocklist/render.rs`

Change `FOOTER_HINT` (line 24) to drop `/jk` and surface the new keys, e.g.:

```rust
const FOOTER_HINT: &str =
    "↑↓ Navigate | PgUp/PgDn/Home/End: Scroll | Enter/Space: Toggle Rule | Esc: Close";
```

### Out of scope
- Mouse-wheel support is tracked separately in **BLOCK-011** (dependent card).
- `panes.rs` (render-only) and `dispatch_blocklist.rs` (action plumbing) — untouched.

## Acceptance criteria (rules)

1. **Vim keys are inert.** Pressing `j`, `J`, `k`, or `K` in the blocklist view
   does NOT move the selection (returns `Consumed` no-op, `selected_index`
   unchanged).
2. **Arrow keys still navigate.** `Down` moves selection down one; `Up` moves up
   one (unchanged behaviour).
3. **PageDown/PageUp page by a viewport.** `PageDown` advances `selected_index`
   by up to `visible_rows` (clamped to last rule); `PageUp` retreats by up to
   `visible_rows` (clamped to 0). `scroll_offset` reconciles so the selection
   stays visible.
4. **Home/End jump to ends.** `Home` selects the first rule (index 0); `End`
   selects the last rule (`len-1`). `scroll_offset` reconciles.
5. **Toggle preserved.** `Space` and `Enter` still emit
   `Action::ToggleBlocklistRule(id)` for the focused rule.
6. **Footer parity.** `FOOTER_HINT` no longer contains `jk`; it advertises the
   arrow + Page/Home/End keys.

## Examples

- List of 20 rules, `visible_rows = 8`, selection at 0 → press `PageDown` →
  selection = 8, list scrolled so row 8 is visible.
- Selection at 8 → press `PageUp` → selection = 0.
- Selection anywhere → press `End` → selection = 19 (last), window shows the tail.
- Selection at 19 → press `Home` → selection = 0, window at top.
- Selection at 5 → press `j` → selection **stays 5** (vim inert).
- Focused rule → press `Space` → emits `ToggleBlocklistRule(focused.id)`.

## Test strategy

Extend the existing blocklist unit tests (`views/blocklist/tests.rs`, in-module
`#[cfg(test)]` so it can read private `selected_index` / `scroll_offset()` via
the `scroll_offset()` accessor + `set_visible_rows()`), plus/or an external
integration test under `tests/`. Use the existing pattern from the BLOCK-008
scroll tests. Drive `handle_key` with synthetic `KeyEvent`s and assert
`selected_index` / `scroll_offset()`. For footer parity, render into a
`TestBackend`/`Buffer` and assert the footer row does not contain `jk`.

Confirm tests FAIL first (red): today `j`/`k` DO move (so the "vim inert" test
fails), and Page/Home/End are no-ops (so those tests fail).

## Files

- `codelet/fspec-tui/src/views/blocklist/mod.rs` — key handler + new helpers
- `codelet/fspec-tui/src/views/blocklist/render.rs` — `FOOTER_HINT`
- `codelet/fspec-tui/src/views/blocklist/tests.rs` — unit tests (and/or a new
  `tests/blocklist_view_keybindings_block010.rs` integration test)
