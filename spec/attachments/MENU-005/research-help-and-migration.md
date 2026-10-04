# MENU-005 — Research: 'u' → menu-bar help re-purpose + test/snapshot migration + tag

Scope: re-purpose BOARD-023's `u` Actions dialog as the menu-bar help
overlay, update the board help content, register the `@menu-bar` tag, and
migrate every snapshot / source-shape / assertion test the previous units
touch. This is the "make the suite green + keep the docs honest" unit.

## Re-purposing the `u` dialog

Today (BOARD-023, `components/board_keybinding_dialog.rs`):

- `Priority::Foreground` modal, id `board-actions-dialog`, `Accent::Yellow`,
  title `Actions`, 8 rows (the six `SHORTCUTS` + `? Help` + `Esc Exit`),
  wrap-around Up/Down + wheel, Enter emits the row's `Action` + closes,
  Esc closes without executing, every other key consumed (true-modal).
- Rows are built from `views/board/board_shortcuts::SHORTCUTS` — the same
  table the menu bar uses (MENU-001 registry).

The re-purpose:

- The dialog STAYS a true-modal Foreground component (it's an overlay, not
  the anchored dropdown — keep both: the dropdown is the live GUI menu;
  this dialog is the "show me everything" help surface, reached by `u`).
- Rows are now generated from the FULL `MenuCategories` registry: for each
  category, its entries (key + label + description), in order. This is the
  "can never drift" guarantee (BOARD-023 R11) carried over to the new
  registry — the dialog, the dropdown, the bar glyphs, and the bare-key
  arms all read one table.
- The `? Help` / `Esc Exit` rows move INTO the registry as a `Help`
  category (MENU-001), so the dialog lists them as regular rows.
- `new_agent_target` snapshot (the `.` New Agent session target captured at
  open, BOARD-023 R5) is preserved: the registry entry for New Agent
  carries `MenuAction::OpenAgentView(Option<SessionId>)` and the dialog
  builder substitutes the snapshot for the `None` placeholder (same logic
  as the current `enter_action()` L142-164).
- Rename: `BoardKeybindingDialog` → `MenuBarHelpDialog` (id string
  `board-actions-dialog` stays STABLE for a release to avoid orphaning
  saved state — RED CARD: keep the id or rename to `menu-bar-help`;
  recommendation: keep `board-actions-dialog` this release, rename in the
  next, since the dialog is only ever pushed/popped within a session and
  never persisted).
- Footer text (`"↑↓ Navigate │ Enter Execute │ Esc Close"`) unchanged.
- Trigger: `u`/`U` in the board (single + focused mux board pane) unchanged
  (`views/board/keys.rs` L104-109); the dialog is opened via
  `Action::OpenBoardKeybindingDialog` — RED CARD: keep the action name vs.
  rename to `OpenMenuBarHelp` (recommendation: keep the name for this
  release; the name is an internal bus token, not user-visible).

## Help content

- `components/help_content.rs` gains/updates rows (the board help table
  already has a `u Actions` row from BOARD-023 R6):
  - `←/→` row now reads: "Cycle columns → menu items → session chips
    (wrap)" (was "Columns").
  - a new row: "Enter on a menu item — open its dropdown menu".
  - a new row: "Enter on a session chip — open that session".
  - the `u Actions` row becomes `u Menu bar help`.
- The agent-view help (slash `/help` path) is unchanged except it now
  mentions `←` (empty input) enters the menu bar.

## Test + snapshot migration inventory

Every test that asserts the OLD bar/chord/5-row-agent-chrome must be
updated or regenerated:

1. **`spec/features/board-actions-popup-dialog-triggered-by-u-key.feature`**
   — BOARD-023's feature. R10 (chord row replaced by `u Actions` hint) and
   R11 (single-source table) are SUPERSEDED: the hint is now the live bar;
   the table now lives in `components/menu_bar/items.rs`. Add
   `@superseded-by-MENU-002` / `@superseded-by-MENU-005` notes (or a new
   "superseded" docstring) rather than deleting the file (coverage history).
2. **Two header snapshots** (BOARD-023 commit regenerated
   `app_with_mock_backend__help_dialog_dismissed.snap` +
   `app_with_mock_backend_repl__repl_bootstrap_rpc012.snap`) — regenerate
   with the new bar row.
3. **`rpc014-source-shape`** (board split) — board vertical split is
   UNCHANGED (the bar replaces row 3 of the existing 4-row header). Verify
   the test still passes as-is; only its header-content assertions change.
4. **`rpc013-source-shape`** (`agent_view_splits_into_scrollback_input_and_footer_rows`)
   — passes UNCHANGED with the `PaneSession.menu_row=false` path; add a new
   scenario pinning the 6-row flag-on list (MENU-003).
5. **BOARD-023 suite** `tests/board_actions_dialog_board023.rs` — the 19
   scenarios mostly survive (the dialog is still a modal opened by `u`);
   the "header chord replaced by short Actions hint" scenario now asserts
   the LIVE bar; the "no trigger row" scenario is reworded (the `u` trigger
   is advertised by the bar/help, still not a dialog row).
6. **Chord assertions** in `rpc015`, `mux009`, `board022`, `rpc395`,
   `board021`, `source_shape_board_search_board022` (from the BOARD-023
   commit's stat) — update the `u Actions` string assertions to the new bar
   content.
7. **Board footer hint** (`views/board/footer.rs` L28) — the
   "← → Columns" string gains "→ menu" (asserted by a test? RED CARD —
   grep for the footer string in tests before migrating).
8. **New snapshots** for the three surfaces: board (bar + open dropdown),
   agent (bar row under header + open dropdown), mux (top-row bar +
   suppressed per-pane rows). Use `insta` (workspace snapshot pattern) at
   120x24 and a narrow 80x24 (exercises the truncation ladder).
9. **Tag registry** — register `@menu-bar` (category: Technical Tags /
   TUI), and if the categories need it, a `@menu-dropdown` tag.
   `fspec register-tag`.
10. **Coverage** — each MENU work unit gets its
    `.feature.coverage` linked (MENU-001..005 feature files exist once
    generated in the specifying phase); `link-coverage` for the new
    component files + the view wiring.

## ACDD process notes

- This unit is the LAST in the sequence; it should run `fspec validate`,
  `fspec validate-tags`, `cargo check/clippy/test -p codelet-fspec-tui`
  (scoped per the workspace "NEVER unscoped cargo test" rule — e.g.
  `cargo test -p codelet-fspec-tui --profile ci-test`), and regenerate all
  snapshots in one pass.
- The feature files for MENU-001..005 are generated in their own
  specifying phases (Example Mapping → `generate-scenarios`); MENU-005's
  feature file is the small "migration" feature that pins the superseded
  BOARD-023 scenarios + the tag registration.

## Open red cards

1. Dialog id: keep `board-actions-dialog` this release (my recommendation)
   vs. rename to `menu-bar-help` now.
2. Action name: keep `OpenBoardKeybindingDialog` (my recommendation) vs.
   rename to `OpenMenuBarHelp`.
3. Dialog rename `BoardKeybindingDialog` → `MenuBarHelpDialog` (my
   recommendation) — safe (internal struct).
4. Footer hint string change — confirm the exact new wording before
   regenerating any footer snapshot.
