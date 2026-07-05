# BLOCK-011 — BlocklistView mouse-wheel scroll support

## Summary

The Rust TUI `/blocklist` view supports keyboard navigation but has **no
mouse-wheel support**. Both canonical reference scroll views
(`model_selector`, `provider_settings`) handle
`MouseEventKind::ScrollUp`/`ScrollDown` via the shared
`scroll_viewport::WheelVelocity` 1×–5× acceleration ramp. This card adds
wheel support to `/blocklist` for parity.

Depends on **BLOCK-010** (keyboard parity) — both touch
`views/blocklist/mod.rs` and `views/navigator_events.rs`, so they are done
sequentially to avoid conflicts.

## Root cause (exact locations)

### 1. No `handle_mouse` on the view — `views/blocklist/mod.rs`

`BlocklistView` (lines 82–204) exposes `handle_key` (150–182) but **no
`handle_mouse` method**. There is no `WheelVelocity` field either.

### 2. Navigator drops mouse events — `views/navigator_events.rs`

`handle_blocklist_event` (lines **148–168**):

```rust
pub(crate) fn handle_blocklist_event(&mut self, event: &Event) -> EventResult {
    let Event::Key(key) = event else {
        return EventResult::ignored();   // line 149–151 — Event::Mouse dropped here
    };
    match self.blocklist.handle_key(*key) { ... }
}
```

Any `Event::Mouse(_)` hits the `else` and returns `ignored()` before reaching
the view.

## Reference / canonical behaviour (what we mirror)

### model_selector

**View** — `views/model_selector/dispatch.rs` `handle_mouse` (lines 190–207):

```rust
pub fn handle_mouse(&mut self, ev: MouseEvent) -> ModelSelectorEvent {
    use crate::components::scroll_viewport::WheelDirection;
    use crossterm::event::MouseEventKind;
    let dir = match ev.kind {
        MouseEventKind::ScrollUp => WheelDirection::Up,
        MouseEventKind::ScrollDown => WheelDirection::Down,
        _ => return ModelSelectorEvent::Ignored,
    };
    let step = self.wheel.step(dir);          // WheelVelocity ramp field `wheel`
    let mover: fn(&mut Self) = match dir {
        WheelDirection::Up => Self::move_up,
        WheelDirection::Down => Self::move_down,
    };
    for _ in 0..step.unsigned_abs() { mover(self); }
    ModelSelectorEvent::Consumed
}
```

**Navigator** — `views/navigator_events.rs` `handle_model_selector_event`
(lines 94–102) routes mouse BEFORE the key guard:

```rust
if let Event::Mouse(mouse) = event {
    return match self.model_selector.handle_mouse(*mouse) {
        ModelSelectorEvent::Consumed => EventResult::consumed(),
        _ => EventResult::ignored(),
    };
}
let Event::Key(key) = event else { return EventResult::ignored(); };
```

### provider_settings

`views/provider_settings/mouse.rs` `handle_mouse` (22–38) uses the same
`WheelVelocity` pattern.

### Shared primitive — `components/scroll_viewport.rs`

- `WheelDirection { Up, Down }`
- `WheelVelocity` — `.step(dir)` returns an accelerating step count (1×–5×) so
  rapid wheel events move multiple rows per event. Default-constructible.

## Fix direction

### A. `views/blocklist/mod.rs`

1. Add a `wheel: WheelVelocity` field to `BlocklistView` (it already
   `#[derive(Default)]`; `WheelVelocity` is `Default`, so no manual ctor
   change needed — but confirm the derive still holds; if `WheelVelocity` is
   not `Clone`/`Debug`/`Default`, adjust derives accordingly). Reference:
   check how `model_selector` declares its `wheel` field and its derives.
2. Add `pub fn handle_mouse(&mut self, ev: MouseEvent) -> BlocklistEvent`
   mirroring the model_selector implementation:
   - `ScrollUp` → repeat `move_up` `step` times → `Consumed`
   - `ScrollDown` → repeat `move_down` `step` times → `Consumed`
   - other kinds → `Ignored`
   - `move_up`/`move_down` already call `adjust_scroll()`, so `scroll_offset`
     reconciles for free.
3. Import `crossterm::event::{MouseEvent, MouseEventKind}` and
   `scroll_viewport::{WheelDirection, WheelVelocity}` as needed.

### B. `views/navigator_events.rs`

In `handle_blocklist_event` (148–168) add a mouse-routing branch BEFORE the
`let Event::Key(key) = event else { ... }` guard, mirroring
`handle_model_selector_event` (94–102):

```rust
if let Event::Mouse(mouse) = event {
    return match self.blocklist.handle_mouse(*mouse) {
        BlocklistEvent::Consumed => EventResult::consumed(),
        _ => EventResult::ignored(),
    };
}
```

### File-size guard
`mod.rs` is currently 236 lines. Adding a field + `handle_mouse` (~18 lines)
keeps it under 300. If it approaches the limit, extract key/mouse dispatch to a
sibling `dispatch.rs` (mirror model_selector) — but only if needed.

## Acceptance criteria (rules)

1. **Wheel down moves selection down.** A `ScrollDown` mouse event advances the
   selection by the `WheelVelocity` step count (clamped to last rule);
   `scroll_offset` reconciles so the selection stays visible.
2. **Wheel up moves selection up.** A `ScrollUp` event retreats the selection by
   the step count (clamped to 0).
3. **Acceleration ramp.** Rapid consecutive wheel events in the same direction
   move more than one row per event (the shared `WheelVelocity` 1×–5× ramp),
   matching model_selector feel.
4. **Non-wheel mouse events ignored.** A mouse move / click / drag event returns
   `Ignored` (no selection change).
5. **Navigator routes wheel to the view.** `handle_blocklist_event` forwards
   `Event::Mouse` into `BlocklistView::handle_mouse` and translates `Consumed`
   → `EventResult::consumed()`, else `ignored()`.

## Examples

- 20 rules, selection at 0 → single `ScrollDown` → selection = 1 (first tick,
  1× ramp).
- Several fast `ScrollDown` events → selection jumps multiple rows per event as
  the ramp accelerates.
- Selection at 19 (last) → `ScrollDown` → selection stays 19 (clamped).
- Selection at 0 → `ScrollUp` → stays 0 (clamped).
- `MouseEventKind::Moved` event → `Ignored`, selection unchanged.

## Test strategy

- **View-level unit tests** (`views/blocklist/tests.rs`): construct a
  `BlocklistView`, `set_rules(...)`, `set_visible_rows(n)`, then call
  `handle_mouse` with synthetic `MouseEvent`s (`MouseEventKind::ScrollDown`
  etc.) and assert `selected_index` and `scroll_offset()`. Assert a `Moved`
  event returns `Ignored`.
- **Navigator-level test** (if a harness exists, mirror the model_selector
  RPC-353 wiring test): drive an `Event::Mouse` through
  `handle_blocklist_event` and assert `EventResult::consumed()` and that the
  view's selection changed.

Confirm tests FAIL first (red): today `handle_mouse` does not exist (compile
error) and the navigator drops mouse events.

## Files

- `codelet/fspec-tui/src/views/blocklist/mod.rs` — `wheel` field + `handle_mouse`
- `codelet/fspec-tui/src/views/navigator_events.rs` — mouse routing branch
- `codelet/fspec-tui/src/views/blocklist/tests.rs` — unit tests (and/or a new
  `tests/blocklist_view_mouse_block011.rs` integration test)
