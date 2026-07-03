# COPY-010 — Text selection anchors at line-start instead of the pressed column

**Type:** Bug · **Epic:** tui-text-selection-copy · **Surfaces:** AgentView scrollback, input composer, turn-content modal, board details strip

---

## 1. Symptom (user-reported)

> "When I select something (in the message area, etc.) it doesn't select from where I clicked the mouse — it selects from the start of the line often."

Dragging to select highlights/copies from **column 0** of the press row through the drag point, instead of from the **exact cell the mouse was pressed at**.

## 2. Root cause

The shared primitives are **correct**:

- `src/mouse/gesture.rs` `SelectionRecognizer::on_mouse` builds the cell from the real event coords (`gesture.rs:69–72`) and, on a drag, emits `Begin(press_cell)` then `Extend(cursor_cell)` with the **true pressed column** (`gesture.rs:78–90`).
- `src/mouse/selection.rs` `Selection::spans` faithfully normalizes whatever anchor/cursor it is given.

The defect is duplicated in the **four view-level `Begin` handlers**, which throw the pressed column away and pin the anchor to column 0 and the cursor to the content width:

| Surface | File:line | Offending code |
|---|---|---|
| Scrollback | `src/views/agent/scrollback_copy.rs:84–94` (`selection_begin`) | `anchor: Cell { row, col: 0 }`, `cursor: Cell { row, col: self.content_width }` |
| Input composer | `src/views/agent/multiline_input_select.rs:120–137` (`apply_gestures`, `Begin`) | `anchor col 0`, `cursor col body_width` |
| Turn-content modal | `src/views/agent/turn_modal_select.rs:101–112` (`apply_turn_modal_gestures`, `Begin`) | `anchor col 0`, `cursor col cw` |
| Board details strip | `src/views/board/details_select.rs:91–104` (`apply_details_gestures`, `Begin`) | `anchor col 0`, `cursor col cw` |

The paired `Extend` handlers only move the **cursor**, never re-anchor:
- `scrollback_copy.rs:98–102`, `multiline_input_select.rs:138–142`, `turn_modal_select.rs:113–117`, `details_select.rs:105–109`.

Because a drag fires `Begin(press)` **immediately followed by** `Extend(drag)` (`gesture.rs:83–86`), the sequence is: anchor pinned to col 0 → cursor overwritten to drag point. The anchor is never revisited, so **every drag selection starts at column 0** = line start. This was an intentional "long-press = whole line" shortcut that was wrongly applied to *all* Begins.

## 3. Why the existing tests didn't catch it

Every drag test presses `Down` at `r.x` (local **column 0**) and asserts the whole line is copied (e.g. `agentview_text_selection_copy_copy006.rs:189`, `turn_content_modal_..._copy008.rs`, `board_details_strip_..._copy009.rs`). Pressing at column 0 makes the bug invisible because press-col == line-start. **No test presses at a mid-line column and asserts a partial-from-that-column selection.**

## 4. Constraint — DO NOT regress long-press

`agentview_text_selection_copy_copy006.rs:222–250` ("Long-pressing a line selects and copies it") is a **real, user-facing, tested** behavior: a stationary press held ~0.5 s (fired via `SelectionRecognizer::tick`, `gesture.rs:104–114`) then release must select and copy the **whole line** under the press. The fix must keep this working while making **drag** precise.

## 5. Required behavior (acceptance criteria)

For **each** of the four surfaces:

1. **Drag from a mid-line column** → the selection/highlight/copied text begins at that column, not at column 0.
2. **Drag** selects exactly the cells from the pressed cell to the released cell (single-row: `press.col .. release.col`; multi-row keeps existing linewise semantics for first/middle rows via `Selection::spans`, but the FIRST row starts at the pressed column).
3. **Long-press with no drag, then release** → still selects and copies the **whole line** under the press (unchanged).
4. A zero-width drag (press and release on the same cell, no movement) copies nothing (collapsed selection → empty), matching standard selection behavior.

## 6. Recommended architecture (distinct long-press gesture)

The wiring must be able to tell a **drag Begin** (precise anchor) from a **long-press Begin** (whole line). Recommended, explicit approach:

1. **`src/mouse/gesture.rs`** — introduce a distinct long-press gesture. Have `tick` (the long-press path) emit a new `SelectionGesture::BeginLine(Cell)` instead of `Begin(Cell)`. The drag path in `on_mouse` continues to emit `Begin(Cell)` then `Extend(Cell)`. Update the recognizer unit tests accordingly (`a_stationary_long_press...`, `long_press_then_drag...`). NOTE: `long_press_then_drag` — a `BeginLine` followed by a real `Extend` should behave as a precise drag from the press cell once dragging starts; document/keep whichever behavior the scenario pins, but do not silently change it.
2. **Each `Begin` handler** (4 sites): set `anchor = cursor = press cell` (real row+col) → precise, collapsed-until-Extend.
3. **New `BeginLine` handler** (4 sites): set `anchor = { row, col: 0 }`, `cursor = { row, col: content_width }` → whole line (the OLD Begin behavior, now reserved for long-press only).
4. **`Extend`** handlers: unchanged (move cursor only) — now correct because the anchor is the true press cell.
5. **Scrollback Action plumbing**: `apply_selection_gestures` (`mouse_dispatch.rs:202–217`) currently maps `Begin`→`Action::SelectionBegin(cell)`. Add a `BeginLine`→`Action::SelectionBeginLine(cell)` variant and route it in the reducer to a new `ScrollbackList::selection_begin_line(cell)` (whole line), while `selection_begin` becomes precise. The other three surfaces hold their `Selection` locally and need no Action change.

> Alternative (if the worker prefers fewer variants): keep a single `Begin`, set `anchor=cursor=press` precisely, and on **Commit** expand to whole-line **only when the selection is still collapsed AND the Begin came from a long-press**. This is rejected as the default because it cannot distinguish a deliberate zero-width drag from a long-press and it spreads special-casing into every Commit path. Prefer the explicit `BeginLine` gesture.

## 7. Files in scope

- `src/mouse/gesture.rs` (+ its unit tests)
- `src/views/agent/scrollback_copy.rs`
- `src/views/agent/multiline_input_select.rs` (+ `multiline_input_select_tests.rs`)
- `src/views/agent/turn_modal_select.rs`
- `src/views/board/details_select.rs`
- `src/views/agent/mouse_dispatch.rs` (`apply_selection_gestures`) + the `Action` enum + its scrollback reducer (for `SelectionBeginLine`)
- Keep every touched `src/` file **< 300 lines** (source_shape ceiling). Split via the existing `#[path] mod` convention if needed.

## 8. ACDD / testing notes

- New feature file: `spec/features/text-selection-anchors-at-pressed-column.feature`, tag `@COPY-010`.
- Write **failing** tests first (red): for each surface, a **mid-line** drag (press at col K>0, drag to col M) asserting the copied bytes start at column K (not 0); plus a regression test that long-press still copies the whole line; plus a zero-width drag copies nothing.
- Reuse existing harnesses: `agentview_text_selection_copy_copy006.rs` (App+MockBackend + injected OSC 52 writer + `poll_selection_tick_for_test`), `turn_content_modal_copy008_helpers.rs`, `board_details_strip_copy009_helpers.rs`.
- Every Gherkin step needs a verbatim `// @step` comment. No `unwrap/expect/panic` in production code.
- Full suite must stay green (`cargo test -p codelet-fspec-tui`) and clippy clean. Some pre-existing COPY-006/007/008/009 tests press at col 0 and will still pass; if any encodes the buggy line-start expectation for a *mid-line* press (none currently do), update it to the correct behavior and note it.
