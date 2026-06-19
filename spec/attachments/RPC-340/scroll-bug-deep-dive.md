# RPC-340 — Model selector list does not scroll to follow the cursor

**Severity: CRITICAL** — this is the single highest-impact defect making the
full-screen model selector unusable on realistic data.

## Summary

In `codelet/fspec-tui/src/views/model_selector/mod.rs`, `scroll_offset` is
**declared and read but NEVER written** after construction. It is initialized to
`0` and only read at render time. Cursor movement (`move_up`/`move_down`,
expand/collapse, filter rebuild, `set_providers`) updates `selected_index` but
never adjusts `scroll_offset` to keep the selection inside the viewport. Once the
list overflows the viewport, the highlight moves off-screen while the rendered
window stays frozen at the top — lower models become invisible/unreachable.

A ready-made `ensure_visible` helper already exists in
`components::scroll_viewport` and is already used by `provider_settings`, so the
fix is to wire it in (mirror `provider_settings::adjust_scroll`).

---

## PART 1 — TypeScript original (how scroll is kept in sync)

- `scrollOffset` state: `src/tui/hooks/useModelSelectorState.ts:151`
- `visibleHeight` state: `useModelSelectorState.ts:152` (returned `:322-323`,
  setters `:332-333`)
- `visibleHeight = height - 6` (flat 6-row chrome approximation):
  `ModelSelectorScreen.tsx:82-84`
- Slice expression windowing the flat list:
  `ModelSelectorView.tsx:144-146` →
  `flatItems.slice(scrollOffset, scrollOffset + visibleHeight)`
- `navigateDown` clamp+shift: `useModelSelectorState.ts:218-231`
  - `if (newIdx >= scrollOffset + visibleHeight) setScrollOffset(newIdx - visibleHeight + 1)`
- `navigateUp` clamp+shift: `useModelSelectorState.ts:233-246`
  - `if (newIdx < scrollOffset) setScrollOffset(newIdx)`
- Reset on open: `useModelSelectorState.ts:286-293` (`setScrollOffset(0)`)
- Reset on filter: `useModelSelectorState.ts:295-305` (selection→0, `setScrollOffset(0)`)
- Scrollbar thumb (shown when `flatItems.length > visibleHeight`):
  `ModelSelectorView.tsx:237-255`
  - thumb height: `max(1, floor((visibleHeight / flatItems.length) * visibleHeight))`
  - thumb pos: `floor((scrollOffset / flatItems.length) * visibleHeight)`

---

## PART 2 — Rust side (current state)

- `scroll_offset` declared `mod.rs:47`, initialized `mod.rs:70`.
- **Read sites only**:
  - `mod.rs:326` → passed into `rows::render_body(...)`
  - `rows.rs:153` param; `rows.rs:194` `let so = scroll_offset.min(total.saturating_sub(1));`
  - `rows.rs:307`/`:316` `render_scrollbar`, `thumb_pos = (scroll_offset * h) / total`
- **Write sites: NONE.** `grep` for `self.scroll_offset =` in
  `views/model_selector/` returns empty. The only "movement" is the render-time
  local clamp `so` at `rows.rs:194` (not persisted).
- `render_body` windowing: `rows.rs:192-197`
  - `visible_rows = list_height` where `list_height = area.height - 1` (legend
    row reserved, `rows.rs:173`)
- `self.visible_rows` is assigned during render (`mod.rs:320`,
  `body_area.height - 1`) but **never used for scroll math** — currently dead.
- `move_up` (`mod.rs:146-153`), `move_down` (`mod.rs:155-164`), `Home`/`End`
  (`mod.rs:253-261`), `toggle_expansion` (`mod.rs:166-194`), `handle_filter_key`
  (`mod.rs:196-223`), `set_providers` (`mod.rs:92-101`) — **all** update
  `selected_index` but never `scroll_offset`. Root cause.

> Note: `visible_rows_for(area)` (`mod.rs:334-337`) subtracts `CHROME_ROWS` (=3,
> `full_screen_shell.rs:22`) — that is the OUTER chrome, the WRONG quantity for
> the body window. The body handed to the closure is already chrome-stripped, so
> the correct viewport row count is `body_area.height - 1` (the legend).

---

## PART 3 — Existing reusable helper (no changes needed)

`components::scroll_viewport::ensure_visible` — `components/scroll_viewport.rs:46-66`,
exported via `components/mod.rs:31`:

```rust
pub fn ensure_visible(
    scroll_offset: &mut usize,
    selected: usize,
    visible_rows: usize,
    total: usize,
)
```

Behavior (`:52-65`): resets to 0 when `visible_rows == 0 || total == 0`; scrolls
up (`*scroll_offset = selected`) when `selected < *scroll_offset`; scrolls down
(`*scroll_offset = selected + 1 - visible_rows`) when
`selected >= *scroll_offset + visible_rows`; clamps to
`max_offset = total - visible_rows`. **Exact match of the TS navigate clamp
logic** plus a max-offset clamp.

`wrap_index(current, delta, total)` (`:29-36`) also exists but is not needed (the
model selector uses header-skipping helpers, not wrap-by-delta).

**Parity precedent:** `provider_settings/mod.rs:18` imports it and wraps it in
`adjust_scroll()` (`:240-248`), called from `move_clamped()` (`:259`). Mirror this.

---

## PART 4 — Proposed precise Rust changes

### 1. Add a private `adjust_scroll` wrapper (mirror `provider_settings::adjust_scroll`)

In `mod.rs`, after `move_down` (~`:164`):

```rust
fn adjust_scroll(&mut self) {
    crate::components::scroll_viewport::ensure_visible(
        &mut self.scroll_offset,
        self.selected_index,
        self.visible_rows,   // body rows minus the legend (= body_area.height - 1)
        self.rows.len(),
    );
}
```

### 2. `visible_rows` value to pass
Pass `self.visible_rows` (already assigned `body_area.height - 1` at `mod.rs:320`).
Keep the `visible_rows: 12` default (`mod.rs:76`) for the pre-first-render case;
`ensure_visible` re-clamps every call so a stale value self-corrects.
Do NOT pass `visible_rows_for(area)` (wrong chrome quantity).

### 3. Call sites for `adjust_scroll()`
| Call site | Location | Reason |
|---|---|---|
| end of `move_up` | `mod.rs:153` | arrow/wheel up |
| end of `move_down` | `mod.rs:163` | arrow/wheel down |
| `Home` | `mod.rs:254` | jump to top |
| `End` | `mod.rs:258-259` | jump to bottom |
| end of `set_providers` | `mod.rs:100` | (re)load / refresh resync |
| `toggle_expansion` (in `if changed`) | `mod.rs:189-192` | row count changes |
| `handle_filter_key` Esc/Backspace/Char | `mod.rs:202,212,218` | filter rebuild |

`move_up`/`move_down` are called from BOTH `handle_key` (`mod.rs:246,250`) and
`handle_mouse` (`mod.rs:298,302`) — wiring `adjust_scroll()` inside those two
covers keyboard AND wheel in one place (preferred).

### 4. Defensive render-time reconcile (recommended in addition)
At the top of the render body closure, right after
`self.visible_rows = body_area.height.saturating_sub(1) as usize;` (`mod.rs:320`),
call `self.adjust_scroll()` so the offset is reconciled against the now-known
viewport height (covers initial-draw / post-resize). `ensure_visible` is
idempotent so this is harmless.

### No changes needed
- `ensure_visible` itself (`scroll_viewport.rs:46-66`)
- `render_body` windowing (`rows.rs:192-197`) — keep the render-time `min` safety net
- Scrollbar math (`rows.rs:304-332`) — follows automatically once offset tracks cursor

---

## Interaction with sibling cards
- **RPC-341 (open-on-current-model)** makes this MORE important: an auto-seeded
  cursor deep in the list will render off-screen until scroll tracks it. Seed
  `scroll_offset` in `set_providers` right after computing `selected_index`.
- **RPC-342 (collapse-by-default)** amplifies overflow scenarios; orthogonal but
  related.
