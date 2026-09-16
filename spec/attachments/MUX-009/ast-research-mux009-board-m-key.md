# AST Research — MUX-009: Board 'M' key opens the Mux config dialog + top-of-screen Mux hint

Work unit: MUX-009 · Epic: mux · Researched: 2026-09-15

## Scope

All integration points that the MUX-009 implementation must touch, verified
via AST search + targeted reads of `rust/fspec-tui`.

## Findings

### 1. BoardView keyboard handler — where the binding lands

- File: `rust/fspec-tui/src/views/board.rs`
- AST match: `pub fn handle_event(&self, event: &Event, store: &BoardStore) -> EventResult`
  at `views/board.rs:147`.
- The existing single-letter bindings live in the `match key.code` block
  starting at line 189: `KeyCode::Char('f') | KeyCode::Char('F')` →
  `Action::OpenChangedFilesView` (line 233), `KeyCode::Char('c') |
  KeyCode::Char('C')` → `Action::OpenCheckpointsView` (line 238), both
  guarded modifier-free-only (`!key.modifiers.contains(KeyModifiers::CONTROL)`
  on `a`/`d`/`.`/`/` arms at lines 245–279). `m`/`M` is currently UNBOUND
  anywhere in the TUI (no `Char('m')` matches in `rust/fspec-tui/src`), so
  the new arm is conflict-free.
- The `M` arm follows the RPC-364 `c`/`C` shape: emit
  `Action::OpenMuxConfigDialog`, `return EventResult::consumed()`.

### 2. Mux pane routing — the binding serves the mux Board pane automatically

- File: `rust/fspec-tui/src/views/multiplex/keys.rs`
- `forward_to_pane(event, board, board_view, …, kind, action_tx)`
  (line 71) — `MuxPaneKind::Board` arm calls
  `board_view.handle_event(event, board)` (line 82).
- `classify_key` (line 41) only intercepts Shift+Left/Right; every other
  key (including plain `m`) is `KeyDecision::Forward` to the FOCUSED pane
  only (R2 keyboard isolation). So when the Board pane is focused in
  `ViewMode::Mux`, the same new arm fires and the dialog overlays the grid
  (MUX-001 R9 — dialogs are Foreground in the Compositor, above the mux).
- No change needed in `keys.rs`.

### 3. Action enum + App dispatch — the single mutation surface

- `Action` enum: `rust/fspec-tui/src/components/mod.rs` (mux variants at
  lines 1305–1319: `MuxEnterWorkUnit`, `MuxConfigApplied`,
  `MuxConfigAppliedAndSaved`). New `OpenMuxConfigDialog` variant added in
  that block.
- `App::dispatch` (`rust/fspec-tui/src/app/dispatch.rs:11`) is the single
  mutation surface; mux arms are folded in via
  `a if App::is_mux_action(a) => self.dispatch_mux(a)` (dispatch.rs:290) and
  `is_mux_action` (`app/dispatch_mux.rs:17`). The new arm is simplest as a
  plain match arm next to `EnterWorkUnit`/`OpenAgentView` (lines 88–91),
  calling `self.handle_open_mux_config_dialog()`.
- `handle_open_mux_config_dialog` (`rust/fspec-tui/src/app/dispatch_mux_config.rs:27`):
  idempotent (`compositor.contains(MUX_CONFIG_DIALOG_ID)` early-return),
  seeds draft from `navigator.mux.config()`, pushes `MuxConfigDialog` at
  `Priority::Foreground`. No session guard — app-level. This is the exact
  same entry point the `/mux` slash command uses
  (`app/dispatch_slash_commands.rs:152-157`).

### 4. MuxConfigDialog (unchanged, reused)

- `rust/fspec-tui/src/components/mux_config_dialog.rs` — `MuxConfigDialog::new(MuxConfig)`
  (line 62); rows: Enabled / Orientation / one per pane
  (`components/mux_config_dialog_rows.rs:43 build_rows`); Enter applies
  (`Action::MuxConfigApplied`), `s` applies+saves
  (`Action::MuxConfigAppliedAndSaved`), Esc cancels. Footer:
  `"↑↓ Field · ←→ Value · A Add · ⌫ Remove · S Save · Enter Apply · Esc Cancel"`.

### 5. Top-of-screen hint chord — where "M" must appear

- File: `rust/fspec-tui/src/views/board/keybinding_shortcuts.rs`
- `render(area, buf, theme)` (line 24) paints ONE plain `Span` with the
  literal chord
  `"C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search"`
  (line 33) in `theme.fg`. This row is row 3 of the 4-row board header
  strip (`views/board/header.rs:84-90` → `keybinding_shortcuts::render`),
  i.e. the top-of-screen options line for the Board view.
- Change: append `" ◆ M Mux"` to the literal (rule R4), keep the single
  plain span — no per-chord styling.
- Note: this header strip is painted INSIDE every board pane render, so
  the chord is visible in the single Board view AND in the Board pane of
  the mux grid (no separate work).

### 6. Test seams (for the testing phase)

- `rust/fspec-tui/tests/common/mod.rs` — shared helpers (App construction,
  key injection). Existing board-key tests: `tests/mux001.rs`,
  `tests/bug165_esc_exit_dialog_on_board_pane_in_mux.rs` (escapes the
  board-pane-in-mux fixture — `mux_nav`-style setup visible in
  `views/navigator_mux.rs` tests).
- `BoardView` is reachable as `app.navigator.board`; `compositor.contains(MUX_CONFIG_DIALOG_ID)`
  is the observable for "dialog open".
- Chord render assertions: paint the board header into a `Buffer` and
  assert the chord row string contains the `"M Mux"` tail (precedent:
  `tests/mux005_footer_styling.rs` for footer-string assertions).

## Conclusions

- Exactly four files touched:
  1. `components/mod.rs` — `Action::OpenMuxConfigDialog` variant
  2. `views/board.rs` — `m`/`M` arm (modifier-free guard, consumed)
  3. `app/dispatch.rs` (or `app/dispatch_mux_config.rs`) — dispatch arm →
     `handle_open_mux_config_dialog()`
  4. `views/board/keybinding_shortcuts.rs` — chord string + doc comment
- No state changes, no `MuxConfig` serde change, no mux-routing change,
  no new dialogs. Estimate: 1 story point holds.
