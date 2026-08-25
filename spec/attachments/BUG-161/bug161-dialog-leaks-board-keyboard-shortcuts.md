# Research — BUG-162: Search dialog leaks board keyboard shortcuts

Research date: 2026-08-25. Scope: `rust/fspec-tui` event dispatch + the
BOARD-022 `WorkUnitSearchDialog`.

## Observed symptom

While the board `/` search dialog is open, keys that the dialog does not
explicitly handle leak through to the **BoardView behind the dialog**. The
user can, while typing in the search box, accidentally move the board
selection, open other views, or trigger board shortcuts — the dialog is not
a true modal.

## Root cause — the dialog's catch-all returns `Ignored`

`rust/fspec-tui/src/components/work_unit_search_dialog.rs::handle_event`
(line 185):

```rust
fn handle_event(&mut self, event: &Event) -> EventResult {
    let Event::Key(key) = event else {
        return EventResult::ignored();
    };
    if key.modifiers.contains(KeyModifiers::SHIFT)
        || key.modifiers.contains(KeyModifiers::CONTROL)
    {
        return EventResult::ignored();          // <-- modifier keys leak
    }
    match key.code {
        KeyCode::Esc => ... Consumed,
        KeyCode::Tab => ... Consumed,
        KeyCode::Backspace => ... Consumed,
        KeyCode::Char('/') => Consumed,
        KeyCode::Char(c) => ... Consumed,
        KeyCode::Up | Down | PageUp | PageDown | Home | End => ... Consumed,
        KeyCode::Enter => ... Consumed,
        _ => EventResult::ignored(),            // <-- catch-all leaks
    }
}
```

Two leak paths:

1. **The catch-all `_ => EventResult::ignored()`** (line 254). Any key not
   in the explicit list is `Ignored`.
2. **The modifier guard** (lines 189–193). Any SHIFT/CONTROL-chorded key
   is `Ignored` before the match.

## How `Ignored` reaches the board

`rust/fspec-tui/src/app/events.rs::handle_event` (the stage cascade):

- **Stage 2** — `self.compositor.handle_event(event)` (line 83). The
  `Compositor` walks layers high→low priority and **short-circuits only on
  `Consumed`** (`compositor.rs:156`). When the topmost layer (the search
  dialog, `Priority::Foreground`) returns `Ignored`, dispatch falls
  through.
- **Stage 3** — `self.navigator.handle_event(event, &self.board_store)`
  (line 98). With the BoardView active, this is `BoardView::handle_event`
  (`views/board.rs:147`).

So every key the dialog `Ignored`s is handed to the BoardView.

## The concrete keys that leak (BoardView arms, `views/board.rs`)

| Key | BoardView action (line) | Leaks while dialog open? |
|-----|--------------------------|--------------------------|
| `j` / `Down` (no match) | `SelectNext` (216) | yes |
| `k` / `Up` (no match) | `SelectPrev` (220) | yes |
| `h` / `Left` | `FocusPrevColumn` (208) | yes |
| `l` / `Right` | `FocusNextColumn` (212) | yes |
| `[` | `ReorderUp` (224) | yes |
| `]` | `ReorderDown` (228) | yes |
| `f` / `F` | `OpenChangedFilesView` (233) | yes |
| `c` / `C` | `OpenCheckpointsView` (238) | yes |
| `d` / `D` | `OpenFoundation` (245) | yes |
| `a` / `A` | `OpenAttachmentPicker` (255) | yes |
| `.` | `OpenAgentView` (268) | yes |
| `Enter` (zero matches) | `EnterWorkUnit` (175) | yes |
| `PageUp`/`PageDown`/`Home`/`End` (no matches) | column scroll / select first/last (190–206) | yes |
| `Shift+Right` | `OpenAgentView` (168) | yes (modifier guard) |
| `?` | App-level `HelpDialog` (`app/events.rs:146`, Stage 4) | yes |

Note the dialog *does* consume `Up`/`Down`/`PageUp`/`PageDown`/`Home`/`End`
**when there is at least one match** (its `move_by`/nav arms). The leaks are
the *other* board keys (`j`/`k`/`h`/`l`/`[`/`]`/`f`/`c`/`d`/`a`/`.`/`?`) and
the modifier-chorded keys, plus `Enter`/nav when the match list is empty.

## The existing DRY pattern to reuse (do not re-invent)

The canonical modal contract in this codebase is: **a `Priority::Foreground`
/ `Critical` modal consumes every key it does not explicitly act on, so the
background view is frozen.** The fix is to make the dialog's catch-all
`Consumed` (a no-op) instead of `Ignored`, and to consume the modifier-chorded
keys too.

Reference: the `Compositor` short-circuit contract
(`compositor.rs:150–161`) — `Ignored` means "propagate to the next layer";
`Consumed` means "stop". A modal that wants to block the background view
must return `Consumed` for unhandled input.

The minimal, correct change is in **one file**
(`work_unit_search_dialog.rs`):

1. Change the catch-all from `_ => EventResult::ignored()` to
   `_ => EventResult::consumed()`. This freezes the board for every key the
   dialog does not handle.
2. Change the modifier guard from `return EventResult::ignored()` to
   `return EventResult::consumed()` (or drop the guard and let the catch-all
   consume). This stops `Shift+Right` → `OpenAgentView` and other chords.

This is the same shape the dialog already uses for its explicit arms — it is
a one-word change per path, not a new mechanism.

## Files to change

- `components/work_unit_search_dialog.rs` — catch-all + modifier guard
  return `Consumed` instead of `Ignored`.

No shared-code change is required; this is a contract fix on the dialog
itself.

## Tests to add (ACDD — write first)

- With the dialog open and **no matches**, pressing `j`/`k`/`h`/`l` returns
  `Consumed` (does not move the board selection) — assert the dialog's
  `handle_event` returns `Consumed` and the BoardStore selection is
  unchanged.
- With the dialog open, pressing `f`, `c`, `d`, `a`, `.` returns `Consumed`
  and does **not** emit the corresponding `Action` (assert the action bus
  received nothing).
- With the dialog open, `Shift+Right` returns `Consumed` and does not emit
  `OpenAgentView`.
- With the dialog open, `?` returns `Consumed` (does not open HelpDialog).
- `Enter` with zero matches returns `Consumed` (no `EnterWorkUnit`).
- Regression: the dialog's own keys (Tab, Esc, Backspace, printable chars,
  Up/Down/Page/Home/End, Enter-with-match) still behave as before.

## Out of scope

- Letting the user pass *specific* keys through to the board (no such
  requirement; a search modal should fully own the keyboard).
- Mouse-event leakage (covered by BUG-161).
