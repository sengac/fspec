# RPC-097 — Fix Plan

## Scope

Two defects, both must be fixed before this card moves to done:

1. **Wiring** — `NavTarget::CreateDialog` from `handle_session_cycle` does
   not result in a `CreateSessionDialog` appearing on screen.
2. **Visual parity** — `CreateSessionDialog::render` does not match
   `src/components/CreateSessionDialog.tsx` styling. Per the user
   directive, the Rust output must be an EXACT copy of the TS dialog.

## Reuse Contract (User Directive)

> "I expect you to re-use the base dialog code in rust to create the
> dialogs for confirmation."

* Base dialog primitive: `codelet/fspec-tui/src/components/dialog_theme.rs`
  exposes `render_dialog(area, buf, &FspecDialog{..})`, `Accent`,
  `DialogRow`, and the canonical footer-separator constant. **All**
  dialog rendering goes through this primitive — no hand-rolled
  `Block`/`Paragraph`.
* The existing `CreateSessionDialog` already delegates to
  `render_dialog`. The fix changes what `DialogRow`s are produced — not
  the primitive.

## Fix 1: Dispatch Wiring

**File:** `codelet/fspec-tui/src/app/dispatch_rpc024.rs` — `NavTarget::CreateDialog`
branch.

Replace:

```rust
NavTarget::CreateDialog => {
    self.agent_view_store.request_create_session_dialog_no_auto();
}
```

With direct mount via the proven RPC-060 path:

```rust
NavTarget::CreateDialog => {
    self.handle_open_create_session_dialog(None);
}
```

Rationale: `handle_open_create_session_dialog` already:

* Idempotency-guards on `CREATE_SESSION_DIALOG_ID`.
* Reads `WorkUnitContext` from the current session so the TS
  context-aware title path (`"Work on <id>?"`) works when Shift+Right
  is pressed from a work-unit-bound session.
* Wires `action_tx` so `CreateSessionSubmitted`/`CreateSessionCancelled`
  flow back into `App::dispatch`.

`show_create_session_dialog` flag remains for backwards-compatibility
with any test that asserts on it, but is no longer the primary signal.

## Fix 2: Visual Parity Refactor

**File:** `codelet/fspec-tui/src/components/create_session_dialog.rs`
— rewrite `render` to produce a row layout that matches the TS output
exactly.

### Required `DialogRow`s (top-to-bottom)

| # | Content                                                            | Styling                              |
|---|---------------------------------------------------------------------|--------------------------------------|
| 1 | Title (`"Start New Agent?"` / `"Work on <id>?"`)                    | Span: `bold`                          |
| 2 | Description (`"Begin a fresh AI conversation, not linked to any task."` or `"Start an AI session for this task"`) | Span: `Modifier::DIM` |
| 3 | (blank)                                                              | —                                    |
| 4 | Three buttons: ` Yes `, ` Yes - Isolated `, ` Cancel ` separated by 2 spaces, centered (rely on `render_dialog` centering) | Selected button: `bg=Blue, fg=White, bold`; unselected: `fg=Gray` |
| 5 | (blank)                                                              | —                                    |
| 6 | Footer: `← → Select | Enter Confirm | Esc Cancel`                   | Span: `Modifier::DIM` |

### Constants to update

* Replace `FOOTER` constant:
  `"← → Select │ Enter Confirm │ Esc Cancel"` → `"← → Select | Enter Confirm | Esc Cancel"`
  (ASCII pipe U+007C, three occurrences).
* Drop the use of `MARKER_SELECTED` / `MARKER_UNSELECTED` constants from
  this dialog (TS source has no marker glyphs).

### Field semantics on `DialogRow`

* `selectable=false` for ALL rows in this dialog (the button row's
  selection is conveyed by per-Span styling, not by row-level inverse
  highlight).
* `selected=false` for ALL rows for the same reason.

### Centering / sizing

* Keep `min_width: 50` (matches the visual breadth of TS dialog).
* `render_dialog` already centers single-line rows. The button row is a
  single `DialogRow` containing three styled buttons separated by two
  spaces — it will center as one composite row.

## Tests to Add (Before Implementation, ACDD Order)

1. **Wiring test (failing first):**
   * Build `App` with one open session at index 0.
   * Dispatch `Action::SessionNext`.
   * Assert `compositor.contains(CREATE_SESSION_DIALOG_ID) == true`
     after the dispatch returns.

2. **Wiring test — empty store:**
   * Build `App` with zero open sessions.
   * Dispatch `Action::SessionNext`.
   * Assert dialog present.

3. **Visual parity insta snapshot:**
   * Render `CreateSessionDialog` into an 80x24 `TestBackend`.
   * Snapshot must show:
     - cyan rounded border
     - centered bold `Start New Agent?` title row
     - dim description row
     - centered button row where ` Yes ` is rendered with blue
       background + white bold text and ` Yes - Isolated ` /
       ` Cancel ` are rendered in gray
     - dim ASCII-pipe footer.

4. **Visual parity test — selection cycling:**
   * Push dialog.
   * Send Right arrow → next render: ` Yes - Isolated ` is the blue-bg
     button.
   * Send Right arrow → ` Cancel ` is blue-bg.
   * Send Right arrow → ` Yes ` is blue-bg (wrap-around).
   * Send Left arrow → ` Cancel ` is blue-bg (wrap-around).

5. **Behaviour test — Enter on each option:**
   * Enter on Yes → `Action::CreateSessionSubmitted { isolated: false }`.
   * Enter on Yes - Isolated → `Action::CreateSessionSubmitted { isolated: true }`.
   * Enter on Cancel → `Action::CreateSessionCancelled`.
   * Esc → `Action::CreateSessionCancelled`.

6. **End-to-end Shift+Right:**
   * App in AgentView with one open session.
   * Crossterm `KeyEvent { code: Right, modifiers: SHIFT }` through
     `App::handle_event`.
   * Assert dialog mounted; assert title is `"Start New Agent?"`
     (no work_unit) or `"Work on <work-unit-id>?"` (when current
     session is attached to a work unit).

## Source-Shape Budget

* `create_session_dialog.rs` currently 258 LoC. The refactor adds
  ~30 LoC for the new row construction and removes the marker logic
  (~10 LoC). Net target < 280 LoC — comfortably under the 300 ceiling.
* `dispatch_rpc024.rs` currently 82 LoC. Net change is a single-line
  swap.

## Out of Scope

* `show_create_session_dialog` flag is preserved (deprecated in spirit
  but not removed) — removing it touches RPC-096 scenarios and adds
  blast radius beyond this bug.
* Slash-command `/isolation` path is already correct (RPC-060); no
  changes there.
* Board-view → CreateSessionDialog path is not in scope (handled by a
  separate dispatch in RPC-060).
