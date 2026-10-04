# MENU-005 — AST research: 'u' dialog re-purpose + dead-shim deletion + test migration

Discovery-phase structural survey (read-only) of the code MENU-005 re-purposes.
All facts below were verified against the tree as of commit 3235168d.

## The dialog being re-purposed

- `src/components/board_keybinding_dialog.rs` (373 LoC, under ceiling):
  - `pub struct BoardKeybindingDialog` (L69) — fields: id, cursor,
    `new_agent_target: Option<SessionId>`, action_tx, pending_action.
  - `impl Component for BoardKeybindingDialog` (L178): Priority::Foreground,
    id const `BOARD_KEYBINDING_DIALOG_ID = "board-actions-dialog"` (L53),
    `TITLE = "Actions"` (L56), `FOOTER = "↑↓ Navigate │ Enter Execute │ Esc
    Close"` (L57), `ACCENT = Yellow`, `MIN_WIDTH = 48`, `ROW_COUNT = 8` (L65).
  - `enter_action()` (L142-164): cursor 0..=5 resolves through
    `views::board::board_shortcuts::SHORTCUTS` (the BOARD-023 six-row table)
    with the `OpenAgentView(_)` arm substituting `new_agent_target`;
    cursor 6 → `Action::OpenBoardHelp`; cursor 7 →
    `Action::OpenBoardExitConfirmation`. **This is the row builder MENU-005
    replaces with the full MenuCategories registry walk.**
  - `render()` (L242-276): rows built from `SHORTCUTS.iter()` + two
    `label_description_row` pushes (Help/Exit). True-modal key blocking:
    Paste consumed (L190), Shift/Ctrl chords consumed (L197-205),
    u/U consumed no-op (L220), everything else consumed (L222);
    Up/Down wrap-around (L127-139), wheel scroll (L226-237).
  - Module declared in `src/components/mod.rs` L18
    (`pub mod board_keybinding_dialog;`) — the only src reference outside
    the file + `app/dispatch_board_keybinding.rs` (L20-21 import).

## The registry it must read instead (single source of truth)

- `src/components/menu_bar/items.rs` (276 LoC):
  - `MenuAction` enum (L35): NewAgent, Search, Attachments, Checkpoints,
    ChangedFiles, Foundation, Mux, Help, Exit.
  - `MenuAction::to_action(self, new_agent_target: Option<SessionId>) ->
    Action` (L61-73) — the resolver the new dialog Enter must call.
  - `MenuEntry { key, label, description, action }` (L79).
  - `ACTIONS: &[MenuEntry]` (L106, 7 entries: . / C F D M A) and
    `HELP: &[MenuEntry]` (L152, 2 entries: ?, Esc) — **9 rows total**.
  - `CATEGORIES: &[MenuCategory]` (L168) — [`actions`, `help`] in order.
  - `MenuCategory { id, label, entries }` (L93).

## The dead shim being deleted

- `src/views/board/board_shortcuts.rs` (161 LoC): `BoardShortcut` struct,
  `BoardShortcutAction` enum + `to_action`, `SHORTCUTS: [BoardShortcut; 6]`
  (L79), `CHORD_HINT = "u Actions"` (L127), and the
  `shortcuts_mirror_the_menu_registry` drift test (L139-160).
  - Only consumer in src: `components/board_keybinding_dialog.rs` (L45, L145,
    L244, L150) — the dialog itself. No other src file imports
    `board_shortcuts`.
- `src/views/board/keybinding_shortcuts.rs` (50 LoC): renders `CHORD_HINT`
  into a row. **Zero call sites** — the board header exposes row 3 to the
  menu bar instead (views/board/header.rs L89-97 `bar_row`); verified by
  Grep: no `keybinding_shortcuts::render` / `keybinding_shortcuts::`
  references anywhere in src or tests.
- Both declared in `src/views/board.rs`: L36 `pub mod board_shortcuts;`,
  L45 `pub mod keybinding_shortcuts;` — both lines deleted.

## App dispatch (unchanged contract)

- `src/app/dispatch_board_keybinding.rs` (62 LoC):
  `handle_open_board_keybinding_dialog` (L32-42) seeds the dialog with
  `board_store.selected_work_unit().and_then(|u|
  board_store.session_for(&u.id).cloned())` — the R3 snapshot seam.
  `try_dispatch_board_keybinding` matches `Action::OpenBoardKeybindingDialog`
  (kept this release), `OpenBoardHelp`, `OpenBoardExitConfirmation`.

## Help content being edited

- `src/components/help_content.rs` (121 LoC):
  - `board_help_lines()` (L22-48): row 27 `"←/h, →/l      Switch column"` →
    ring wording; row 40 `"u             Actions"` → `"Menu bar help"`;
    two NEW Enter rows inserted (key column width is 14: keys like
    `"←/h, →/l"` pad to 14 before 6 spaces? — measured layout: key field is
    14 wide then 6 spaces; new rows use keys `"Enter (item)"` /
    `"Enter (chip)"` style 14-wide keys).
  - `agent_help_lines()` (L55-88): gains one row documenting the
    empty-input bare-Left menu-bar entry (MENU-003 R3).
- `src/components/help_dialog.rs` L75/L81 wire `for_board`/`for_agent` to
  these lines; L295 holds the insta snapshot
  `help_dialog__centered_popup_80x24` (board help top rows — MUST be
  regenerated after the content change).

## Board render geometry (verification pins)

- `src/views/board/render.rs` L63-78: the 11-constraint vertical split is
  UNCHANGED (top border 1 / header 4 / sep 1 / details 5 / sep 1 / column
  header 1 / sep 1 / content Min(0) / sep 1 / footer 1 / border 1) —
  rpc014-source-shape stays green.
- Footer string (views/board/footer.rs L30):
  `"← → Cycle Columns → menu items → chips ◆ ↑↓ Work Units ◆ [ Priority Up ◆
  ] Priority Down ◆ ↵ Work Agent ◆ ESC Back"` — already MENU-002 wording,
  verified unchanged by this unit.
- `views/board/menu_keys.rs` L47: the true-modal open-dropdown gate (R6)
  — the registry's rows are the same ones the dropdown lists.

## Agent pane layout (new rpc013 flag-on pin)

- `src/views/agent/pane_render.rs`:
  - `PaneSession { session, is_focused, menu_row }` (L38-43);
    `PaneSession::current_session()` sets `menu_row: true` (L53).
  - `pane_layout_constraints(role_height, input_height) -> [Constraint; 5]`
    (agent.rs L278) — the rpc013-pinned flag-OFF list, unchanged.
  - `pane_layout_constraints_menu(role_height, input_height) ->
    [Constraint; 6]` (pane_render.rs L69-81):
    `[Length(1) header, Length(1) menu, Length(role_height), Min(0),
    Length(1), Length(input_height)]` — the NEW flag-on pin.
  - `render_session_pane` (L90) branches on `pane.menu_row` (L121) —
    ChromeAreas.menu rect (L135-143) + the bar paint at L202-205.

## Mux surface (per-pane suppression for the new snapshots)

- `tests/menu004_mux_surface.rs` harness (reused for the mux snapshots):
  `fresh_app()` (L52), `drain_pending` (L61), `seed_sessions` (L191),
  `set_status` (L200, via `Action::SessionStatusChanged`), `seed_units`
  (L211, via `Action::WorkUnitsLoaded`), `enable_mux(app,
  Vec<MuxPaneKind>)` (L154, MuxConfig + `enable_with_config`),
  `render_app` 240x24 / `render_app_sized` (L100-110), `bar_row`/`row_text`/
  `find_x` helpers. Board pane header row 3 paints blank when suppressed
  (MENU-004 R-SUPPRESS, views/board/render.rs L97-118).

## Test inventory the migration touches (verified by Grep)

| File | What MENU-005 changes |
|---|---|
| `tests/board_actions_dialog_board023.rs` | DIALOG_ROWS 8→9 (new order); offsets: Mux=5, Help=7, Exit=8; footer−title 11→12; title "Actions"→"Menu bar"; scenario 5 reworded (no 'Actions' body-label check); scenario 19 help row `u             Actions` → `Menu bar help` |
| `tests/help_dialog_content_rpc397.rs` | unchanged assertions (New Agent/Reorder/slash list) — still green |
| `tests/view_board_unit_rpc015.rs` | docstring refresh (bar assertions kept); no CHORD reference in code |
| `tests/source_shape_rpc015.rs` | required-modules list drops `keybinding_shortcuts.rs` (L102 + L69-77 existence assert) |
| `tests/source_shape_board_search_board022.rs` | `board_header_chord_gains_the_search_segment` (L128-148) re-pins: keybinding_shortcuts.rs + CHORD_HINT GONE, bar row hosts the menu items instead |
| `tests/view_board_version_under_logo_board021.rs`, `mux009_board_m_key.rs`, `board_period_new_agent_rpc395.rs`, `board_search_dialog_board022.rs`, `view_board_unit_rpc015.rs` | docstring/comment refresh only (assertions already bar-based) |
| `tests/scrollback_scroll_rpc094.rs` | components/mod.rs ceiling 1424 → +2 (rename of `pub mod board_keybinding_dialog;` line → `pub mod menu_bar_help;` is delta 0; but `BoardKeybindingDialog` doc stanzas in the Action enum stay; measure at implementation and itemize the stanza) |
| `tests/app_with_mock_backend.rs` + snapshots `help_dialog_visible` / `help_dialog_dismissed` | the `help_dialog_visible` frame shows the board help dialog over the bar — the board help CONTENT changed (row 27 + row 40) → regenerate |
| `tests/app_with_mock_backend_repl.rs` + `repl_bootstrap_rpc012` | board-only frame, bar content unchanged → expected UNCHANGED (verify, don't regenerate) |
| `src/components/help_dialog.rs` test module | `help_dialog__centered_popup_80x24` snapshot contains the changed board help rows (L14 "Switch column" row) → regenerate; the `help_dialog_body_lists_the_board_keybindings` needles (Navigate/New Agent/Reorder) still hold |
| NEW `tests/menu005_bar_help_snapshots.rs` | insta surface snapshots: board 120/80x24 (App harness + sessions + Running status), agent 120/80x24 (menu003 `harness()` + `seed_sessions` + `set_session_status`, render via `render_with_store`), mux 120/80x24 (menu004 harness: seed_sessions + set_status + seed_units + enable_mux) |
| NEW inline test in `components/menu_bar_help.rs` | 80x24 insta of the dialog (title "Menu bar", 9 registry rows, Yellow accent) |
