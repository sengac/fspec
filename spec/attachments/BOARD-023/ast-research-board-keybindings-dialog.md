# AST Research — BOARD-023 (Board keybindings popup dialog)

Date: 2026-09-28 · Scope: `rust/fspec-tui/src/views/board`, `rust/fspec-tui/src/components`,
`rust/fspec-tui/src/app`, `rust/fspec-tui/src/views/multiplex`

## 1. Board header chord widget (the thing being collapsed)

AstGrep `pub fn render($$$ARGS) { $$$BODY }` in `views/board`:

- `views/board/checkpoint_status.rs:21` — `render(area, buf, counts: CheckpointCounts)`
- `views/board/keybinding_shortcuts.rs:26` — `render(area, buf, theme: &Theme)` ← **the chord**
- `views/board/footer.rs:22` — `render(area, buf, theme: &Theme)`
- `views/board/details_strip.rs:32` — `render(area, buf, selected: Option<&WorkUnitInfo>)`
- `views/board/logo.rs:41` — `render(area, buf, theme: &Theme)`

`keybinding_shortcuts.rs` paints ONE hardcoded literal span
(`"C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search ◆ M Mux"`),
styled `Style::default().fg(theme.fg)`. **Change: replace literal with `", Keys"`** (R10).

`views/board/header.rs::paint` composes the 4-row strip: row 0 checkpoint_status,
row 2 `─` divider, row 3 keybinding_shortcuts. No layout change needed (still row 3).

## 2. Board key arms (emitters to reuse, NOT duplicate)

`views/board.rs::handle_event` (match arms):
- `/` → `Action::OpenWorkUnitSearch` (always consumed, no-CONTROL guard)
- `Enter` → `Action::EnterWorkUnit(unit.id)` when a unit is selected
- `Shift+Right` → `Action::OpenAgentView(selected_session(store))`
- `h/l/j/k`, arrows, `[`/`]`, PageUp/Down, Home/End → navigation actions

`views/board/keys.rs::handle_mode_view_key` (free function, extracted <300 LoC):
- `f`/`F` → `Action::OpenChangedFilesView`
- `c`/`C` → `Action::OpenCheckpointsView`
- `m`/`M` → `Action::OpenMuxConfigDialog`
- `d`/`D` → `Action::OpenFoundation`
- `a`/`A` → `Action::OpenAttachmentPicker` (only when selected unit has attachments; always consumed)
- `.` → `Action::OpenAgentView(view.selected_session(store))`

**All arms: `KeyCode::Char(X) if !key.modifiers.contains(KeyModifiers::CONTROL)` guard,
`Some(EventResult::consumed())` when claimed, `None` to fall through.**
**NEW ARM: `KeyCode::Char(',')` same guard → `Action::OpenBoardKeybindingDialog`.**

Mux mode: `views/multiplex/keys.rs::forward_to_pane` routes Board-pane keys through
`board_view.handle_event(event, board)` — so the single arm in `keys.rs` serves both
surfaces (MUX-009 precedent; no mux-specific code needed).

## 3. Existing dialog components (patterns to clone)

AstGrep `pub struct $NAME { $$$FIELDS }` in `components/` — the modal dialog family:

| Component | Priority | Accent | Cursor semantics | Enter | Esc |
|---|---|---|---|---|---|
| `ThinkingLevelDialog` (35) | Foreground | Yellow | wrap-around | emits + `Consumed(Some(remove_callback))` | remove_callback |
| `MuxConfigDialog` (50) | Foreground | Cyan | wrap-around | apply + close | cancel |
| `AttachmentPickerDialog` (38) | Foreground | Yellow | clamped | emits `OpenAttachment` + close | close |
| `WorkUnitSearchDialog` (58) | Foreground | Cyan | wrap + scroll | `SelectWorkUnit` + close | close |
| `BoardExitConfirmationDialog` (66) | **Critical** | Yellow | L/R cyclic | `Quit` or cancel | cancel |
| `HelpDialog` (44) | **Critical** | Cyan | scroll | (none) | dismiss |

Shared rendering: `dialog_theme::{render_dialog, render_dialog_at, Accent, FspecDialog, DialogRow}` +
`dialog_theme_rows::label_description_row(label, desc, selected)` (marker `▸ ` + label + dim desc) +
`build_dialog` helper.

**Chosen pattern: ThinkingLevelDialog structure** (9 static rows, wrap-around cursor,
Enter → emit + close) **+ WorkUnitSearchDialog BUG-161 true-modal blocking**
(every unhandled key `consumed()`; Shift/Ctrl chords swallowed; trigger key consumed no-op)
**+ MuxConfigDialog mouse-wheel parity** (ScrollUp/Down → move cursor).

Key-blocking detail (BUG-161): `if key.modifiers.contains(SHIFT) || contains(CONTROL) {
return EventResult::consumed(); }` then match explicit arms, `_ => EventResult::consumed()`.
Also swallow `Event::Paste` (RPC-403 pattern) and mouse events outside wheel.

## 4. App wiring (idempotent push precedent)

- `app/dispatch_mux_config.rs::handle_open_mux_config_dialog` — `compositor.contains(id)`
  guard, `with_action_tx(self.action_tx.clone())`, `compositor.push(Box::new(dialog))`.
- `app/dispatch_work_unit_search.rs::handle_open_work_unit_search` — same pattern;
  seeds dialog with `board_store.work_units().to_vec()`.
- `app/dispatch_viewer.rs::handle_open_attachment_picker` — same + selected-unit guard.
- Capability chain: `app/dispatch_capability.rs` — `try_dispatch_*` OR-chain;
  **insert `try_dispatch_board_keybinding` into the chain** (new file `app/dispatch_board_keybinding.rs`).
- Stage-4 App shortcuts in `app/events.rs::handle_app_shortcut`:
  - `?` → `compositor.push(Box::new(HelpDialog::for_board()))`
  - `Esc` (Board view / mux board pane) → push `BoardExitConfirmationDialog::new().with_action_tx(...)`
    (guarded by `compositor.contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID)`)
  - **Refactor: extract these two pushes into `App` helpers (e.g. `open_board_help()`,
    `open_board_exit_confirmation()`); stage-4 arms AND the new `OpenBoardHelp` /
    `OpenBoardExitConfirmation` dispatch arms call the same helpers (R6 DRY).**

## 5. Action enum

`components/mod.rs` `pub enum Action` — last variants: `MuxEnterWorkUnit`, `MuxConfigApplied`,
`MuxConfigAppliedAndSaved`, `OpenMuxConfigDialog`.
**Add:** `OpenBoardKeybindingDialog`, `OpenBoardHelp`, `OpenBoardExitConfirmation`
(documented with the card, near the MUX-009 variant).

## 6. Tests pinning the old chord (will need updating — recorded supersession)

- `tests/view_board_unit_rpc015.rs` — asserts `C Checkpoints` / `F Changed Files` / `D FOUNDATION.md` / `. New Agent` substrings (scenario "KeybindingShortcuts chord row is painted in the header")
- `tests/mux009_board_m_key.rs` scenario 4 — exact chord string incl. `M Mux` + `/ Search`
- `tests/board_search_dialog_board022.rs:459-462` — `/ Search` in header
- `tests/board_period_new_agent_rpc395.rs:122-133` — `. New Agent` in header
- `tests/view_board_version_under_logo_board021.rs:135-138,208` — `C Checkpoints` on row 4 + `find_cell`
- `tests/source_shape_board_search_board022.rs:128-143` — literal `/ Search` inside `keybinding_shortcuts.rs` source
- `tests/snapshots/app_with_mock_backend__help_dialog_dismissed.snap:10` +
  `app_with_mock_backend_repl__repl_bootstrap_rpc012.snap:10` — insta snapshots containing the chord line
- `components/help_content.rs::board_help_lines` — `?` dialog board list (add `,` row)

Harness precedent: `tests/mux009_board_m_key.rs` — `fresh_app()` (MockBackend),
`drain_pending(app)`, `render_app_rows/render_app_text` (120x24 TestBackend),
`app.compositor().contains(DIALOG_ID)`, `app.active_view()`, `#[serial]` +
`root_data_dir()` when persisting (not needed here — dialog is stateless).

## 7. Constraints

- 300-LoC ceiling per file (enforced by `tests/source_shape_rpc015.rs` for `views/board/**` and
  `views/board.rs`); new `components/` files follow the same convention.
- No `unwrap()`/`expect_used` in production code (workspace denies); tests allow via attribute.
- `tracing` not needed (no errors expected; idempotent push is silent no-op).
- Compositor: `push` sorts by priority (Foreground=900 sits above Background views, below Critical
  Help/Disconnect); `contains(id)` for idempotency; `EventResult::Consumed(Some(cb))` runs `cb`
  (remove) after dispatch unwinds.
- Event stage order (`app/events.rs`): 1 DisconnectDialog → 2 Compositor → 3 Navigator → 4 App
  shortcuts. A Foreground dialog gets Stage 2 first crack → board keys never reach Stage 3/4.
