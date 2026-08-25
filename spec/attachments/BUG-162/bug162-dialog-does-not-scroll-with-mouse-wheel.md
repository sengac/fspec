# Research — BUG-161: Search dialog does not scroll with the mouse wheel

Research date: 2026-08-25. Scope: `rust/fspec-tui` mouse subsystem + the
BOARD-022 `WorkUnitSearchDialog`.

## Observed symptom

The board `/` search dialog list does not respond to the mouse wheel,
unlike the board columns and the AgentView content. When there are more
matches than fit in the viewport, the user can only navigate with
Up/Down/PageUp/PageDown/Home/End — they cannot scroll with the mouse.

## Why it happens

`WorkUnitSearchDialog` implements the `Component` trait
(`components/mod.rs:1222`). Its `handle_event` (line 185) only matches
`Event::Key(key)` and returns `EventResult::ignored()` for everything else
(line 186–188):

```rust
fn handle_event(&mut self, event: &Event) -> EventResult {
    let Event::Key(key) = event else {
        return EventResult::ignored();   // <-- Event::Mouse falls through here
    };
    ...
}
```

So a `Event::Mouse` (wheel) event is **ignored** by the dialog and bubbles
down to the BoardView, which scrolls the *board column* behind the dialog
instead of the dialog's match list. The dialog has no wheel handler.

## The existing DRY primitives to reuse (do not re-invent)

The codebase already has a complete, tested mouse-scroll stack. The dialog
must reuse it, not reinvent it.

| Primitive | File | What it does |
|-----------|------|--------------|
| `WheelVelocity` | `components/scroll_viewport.rs:81` | 1×–5× mouse-wheel acceleration accumulator. `step(WheelDirection) -> i32` returns the signed step to apply; ramps up to 5 within 150 ms, resets after a gap. **This is the single source of truth for wheel speed.** |
| `WheelDirection` | `components/scroll_viewport.rs:70` | `Up` / `Down` enum the velocity accumulator takes. |
| `ensure_visible(offset, selected, visible_rows, total)` | `components/scroll_viewport.rs:46` | Adjusts `scroll_offset` so `selected` stays in the visible window. Already used by the dialog's `move_by` for keyboard nav. |
| `wrap_index(current, delta, total)` | `components/scroll_viewport.rs:29` | Wrap-around selection move. Already used by the dialog. |
| `ScrollbarDrag` + `ScrollbarGeometry` | `mouse/scrollbar_drag.rs` | Click-and-drag scrollbar state machine (TUI-101/103). Pure, no view imports. |
| `render_list_scrollbar(area, buf, offset, visible, total)` | `components/list_scrollbar.rs:23` | Paints a proportional `■`-over-`│` scrollbar thumb (RPC-352). |
| `rect_contains(rect, x, y)` | `mouse/hit_test.rs:29` | Half-open hit-test so wheel events outside the dialog fall through. |

### The reference consumer: `FileSearchPopup`

`rust/fspec-tui/src/views/agent/file_search_popup.rs` is the closest analog
— a centered dialog with a scrollable match list, mouse-wheel scrolling,
and a click-and-drag scrollbar. Its `handle_mouse` (line 180) is the exact
pattern to mirror:

1. **Hit-test** the popup's last-rendered rect; outside → `Ignored` so the
   event bubbles (lines 181–186).
2. **Left-button press/drag/release on the scrollbar gutter** → route
   through `ScrollbarDrag` with `ScrollbarGeometry`, convert absolute row
   to body-local row, apply the returned offset (lines 189–231).
3. **`ScrollUp` / `ScrollDown`** → `let step = self.wheel.step(dir);
   self.move_by(step);` (lines 234–243). `move_by` reuses `wrap_index` +
   `ensure_visible` — the SAME helper the dialog already uses for keyboard
   nav.
4. **`render`** caches `last_scrollbar_rect` + `last_body_origin` for
   hit-testing and computes the scrollbar gutter rect (lines 305–346).

The dialog's `move_by` (line 152) is already byte-identical to the popup's
`move_by` (line 157). So wiring the wheel is a small, mechanical change.

### How the board and AgentView do it (for parity, not to copy)

- **BoardView** (`views/board/mouse.rs:86`): `ScrollUp`/`ScrollDown` inside
  `last_content_area` → emit `Action::SelectPrev`/`SelectNext` (delegates to
  the store's viewport math). No `WheelVelocity` — one row per notch.
- **AgentView scrollback** (`views/agent/mouse_dispatch.rs:164`):
  `ScrollUp`/`ScrollDown` inside `last_scrollback_area` →
  `self.scrollback_wheel.step(dir)` → emit
  `Action::ScrollbackMouseWheel{Up,Down}(velocity)`. This is the
  `WheelVelocity` path.

The search dialog is a self-contained `Component` (it owns its own
`scroll_offset`), so it should follow the **`FileSearchPopup` model**
(self-contained wheel + `move_by`), NOT the Action-emitting model (which
needs the App to apply the scroll to a store the dialog does not own).

## Recommended design

1. **Add `Event::Mouse` handling to `WorkUnitSearchDialog::handle_event`.**
   Replace the early `let Event::Key(key) = event else { ignored }` with a
   branch: `Event::Mouse(m) => self.handle_mouse(*m)`, `Event::Key(key) =>
   { existing }`, `_ => ignored`.
2. **Add a `handle_mouse(&mut self, ev: MouseEvent) -> EventResult`** that
   mirrors `FileSearchPopup::handle_mouse`:
   - hit-test `last_dialog_rect` (cached in `render`); outside → `Ignored`.
   - `ScrollUp`/`ScrollDown` → `self.move_by(self.wheel.step(dir))` →
     `Consumed`.
   - (Optional, parity with TUI-103) left-button on the scrollbar gutter →
     `ScrollbarDrag`; otherwise `Ignored`.
3. **Add the fields** the popup uses: `wheel: WheelVelocity`,
   `last_dialog_rect: Option<Rect>` (or `last_scrollbar_rect` +
   `last_body_origin` if the scrollbar is added).
4. **Cache the rect in `render`.** Store the `dialog_rect`/body origin so
   the next mouse event can hit-test. (Once BUG-159 switches to
   `fixed_dialog_rect`, this rect is stable and trivially cached.)
5. **(Optional) paint a scrollbar** via `render_list_scrollbar` when
   `matches.len() > visible_rows`, matching the popup's gutter.

## Files to change

- `components/work_unit_search_dialog.rs` — `handle_event` gains an
  `Event::Mouse` arm; new `handle_mouse`; new `wheel` + rect-cache fields;
  `render` caches the rect.
- No new shared code — everything reused from `scroll_viewport`,
  `mouse::scrollbar_drag`, `components/list_scrollbar`, `mouse::hit_test`.

## Tests to add (ACDD — write first)

- A `ScrollDown` mouse event inside the dialog rect moves the selection
  down by the wheel step and updates `scroll_offset` (assert via
  `selected_index` / a new `scroll_offset` accessor).
- A `ScrollUp` event moves up; wheel velocity ramps (5 rapid notches →
  step 5, mirroring `WheelVelocity` tests).
- A wheel event **outside** the dialog rect is `Ignored` (bubbles to the
  board) — assert `EventResult::ignored()`.
- With more matches than `visible_rows`, repeated `ScrollDown` reaches the
  last match and `ensure_visible` keeps it on screen.
- (If scrollbar added) a left-button click on the gutter jumps the offset
  via `ScrollbarDrag`.

## Out of scope

- Horizontal wheel scroll (the dialog is a single-column list).
- Trackpad smooth-scroll coalescing beyond `WheelVelocity`'s existing
  150 ms gap logic.
