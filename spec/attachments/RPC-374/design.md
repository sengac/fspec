# RPC-374 — Wire A key on board to open attachment picker and browser

**Parent:** RPC-371 · **Depends on:** RPC-372 (viewer server) · **Blocks:** none

## Goal

Wire the `A`/`a` key in the Rust board view to open an attachment picker for the
**selected work unit**, and on selection open the chosen attachment in the
browser via the RPC-372 viewer server. Port of `BoardView.tsx:327-333, 592-616`
plus `AttachmentDialog.tsx`.

## Current state

- `board.rs::handle_event` has no `a/A` arm.
- The details strip already renders
  `Attachments (use the "A" key to view): <basenames>` — unwired hint.
- `WorkUnitInfo` (`codelet/rpc-types/src/lib.rs`) already carries
  `pub attachments: Vec<String>` (full relative paths; the strip renders
  basenames). `BoardStore::selected_work_unit() -> Option<&WorkUnitInfo>`.
- There are existing dialog/overlay patterns in the TUI (e.g. checkpoints,
  create-session dialog, model selector) to model the picker on.

## Behaviour (port of TS)

- Pressing `A`/`a`:
  - If the selected work unit **has no attachments** → **no-op**, but still
    **consume** the key (TS returns `true` either way). No dialog.
  - If it **has attachments** → open an **AttachmentPicker** dialog listing the
    attachments (render basenames for readability; keep the full path for the URL).
- In the picker: arrow keys move selection, `Enter` selects, `Esc` closes.
- On select: build `http://127.0.0.1:{port}/view/{percent-encoded path}` and
  launch via the `open` crate; close the dialog. Encode the path so spaces /
  unicode are URL-safe (TS uses `encodeURI(attachment)`).

## Architecture

- `board.rs::handle_event` adds:
  ```rust
  KeyCode::Char('a') | KeyCode::Char('A') => {
      if store.selected_work_unit().map_or(false, |u| !u.attachments.is_empty()) {
          self.emit(Action::OpenAttachmentPicker);
      }
      return EventResult::consumed(); // consume even when no attachments
  }
  ```
  (Decision: gate in the view using the borrowed store, OR always emit and let
  dispatch decide. Either is acceptable — document the choice. Consuming the key
  unconditionally matches TS.)
- New `Action` variants in `src/components/mod.rs`:
  - `OpenAttachmentPicker` (open dialog for the selected unit's attachments),
  - `OpenAttachment(String)` (selected attachment path → browser).
- A picker overlay component under `src/views/` (or `src/components/`) modeled on
  the existing dialog components. Keep it < 300 lines; split tests into a sibling
  `_tests.rs` if needed.
- `App::dispatch` (reuse the `dispatch_viewer.rs` helper from RPC-373):
  - `OpenAttachmentPicker` → store the selected unit's attachment list on the
    App/overlay state and show the dialog.
  - `OpenAttachment(path)` → build the URL from the viewer port and launch the
    browser (same launcher used by RPC-373). No port → warn + no-op.

## Browser launch & testability

Same approach as RPC-373: a pure `attachment_url(port, path) -> String`
(percent-encoding the path) that is unit-tested, with the `open::that` call in a
thin wrapper that tests don't invoke. No real browser in tests.

## Scenarios (acceptance criteria)

1. **A on a card with attachments opens the picker** — given the selected work
   unit has ≥1 attachment, when `A` is handled, then `Action::OpenAttachmentPicker`
   is emitted and the key is consumed.
2. **A on a card with no attachments is a silent no-op** — given the selected work
   unit has no attachments, when `A` is handled, then NO picker action is emitted
   but the key is still consumed (`EventResult::consumed`).
3. **Lowercase a behaves identically** — `a` behaves the same as `A`.
4. **Selecting an attachment builds the encoded viewer URL** — given a viewer port
   `P` and an attachment path `spec/attachments/RPC-001/a b.md`, the launched URL
   is `http://127.0.0.1:P/view/spec/attachments/RPC-001/a%20b.md` (spaces/unicode
   percent-encoded).
5. **Selecting an attachment is a safe no-op when the viewer is unavailable** —
   no viewer port → dispatching `OpenAttachment` does nothing (no panic) and warns.
6. **The picker lists the selected work unit's attachments** — the dialog renders
   one selectable entry per attachment (basename shown), preserving order.

## Testing

- Board-level: build `BoardView` + store seeded with a selected `WorkUnitInfo`
  that has attachments → feed `'A'`/`'a'` → assert emitted action + consumed.
  Seed a unit with empty `attachments` → assert no picker action but consumed.
- URL building: unit-test `attachment_url(port, path)` incl. encoding.
- Picker component: render snapshot / unit test that it lists the attachments and
  that `Enter` selects, `Esc` closes (follow existing dialog tests).
- No-op: dispatch `OpenAttachment` with `None` port → no launch.
- Every Gherkin step → `// @step …` comment.

## Definition of done

- 6 scenarios green; tests-first (red → green).
- `cargo build` + `cargo clippy` clean; files < 300 lines.
- The `A` key hint in the details strip is now backed by real behaviour.
- Coverage links recorded for every scenario.
