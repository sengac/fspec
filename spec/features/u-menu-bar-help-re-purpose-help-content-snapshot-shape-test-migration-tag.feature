@done
@ui-enhancement
@menu-bar
@menu-registry
@tui-component
@MENU-005
Feature: 'u' → menu-bar help re-purpose + help content + snapshot/shape test migration + tag
  """
  ARCHITECTURE: components/board_keybinding_dialog.rs is renamed to components/menu_bar_help.rs and the struct to MenuBarHelpDialog; its body rows are now built by walking components/menu_bar/items::CATEGORIES (every category's entries in order = 9 rows) with MenuAction::to_action(new_agent_target) resolving each Enter — the BOARD-023 SHORTCUTS shim (views/board/board_shortcuts.rs) and the dead chord painter (views/board/keybinding_shortcuts.rs) are deleted, the registry becomes the ONLY row source. The compositor id 'board-actions-dialog' and the Action::OpenBoardKeybindingDialog token stay stable this release; the dialog title becomes 'Menu bar'. help_content.rs board rows updated (ring wording, two Enter rows, 'u Menu bar help'); agent rows gain the empty-input Left entry. Migrated: board_actions_dialog_board023 suite (9 rows), rpc015 + source_shape_board_search_board022 + view_board_unit_rpc015 source-shape pins (deleted-file references removed), rpc013-source-shape (new flag-on 6-list scenario), the two board header-row snapshots regenerated. NEW: tests/menu005_bar_help_snapshots.rs — insta surface snapshots for board / agent / mux at 120x24 + 80x24 (board 80x24 exercises the truncation ladder), reusing the menu004 render harnesses. Tags: @menu-bar (already registered) + @menu-registry + @source-shape; BOARD-023 feature gains @superseded notes on R10/R11.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1 (registry rows): the 'u' dialog's body rows are generated from the FULL MenuCategories registry — every category's entries in registry order (10 rows after MENU-008: Kanban's 4, Tools' 2, Settings' 2, Help's 2; was Actions' 7 + Help's 2 = 9), each rendered as 'key label' + one-line description via the shared label_description_row. No other row source may be read (the board_shortcuts::SHORTCUTS shim is deleted). The dialog, the dropdown, the bar and the bare-key docs all read the one CATEGORIES table (BOARD-023 R11 carried over).
  #   2. R2 (rename + stability): BoardKeybindingDialog is renamed to MenuBarHelpDialog (struct + module board_keybinding_dialog.rs -> menu_bar_help.rs) with the dialog's compositor id STAYING 'board-actions-dialog' and the Action variant STAYING OpenBoardKeybindingDialog this release (both are bus/stable tokens; rename is next release). The dialog title changes from 'Actions' to 'Menu bar' (user-confirmed); footer '↑↓ Navigate │ Enter Execute │ Esc Close' unchanged; Accent::Yellow, Priority::Foreground and the true-modal key blocking (BOARD-023 R7/R8) unchanged.
  #   3. R3 (snapshot + execute): the new_agent_target snapshot semantics are preserved — the dialog is still constructed with the selected work unit's Option<SessionId> at open time (App::handle_open_board_keybinding_dialog), and Enter on the '.' New Agent row resolves through MenuAction::to_action(target) so the snapshot substitutes the registry's None placeholder. Enter on ANY registry row emits that entry's Action (resolved via MenuAction::to_action) and closes the dialog — so the registry's Help/Exit rows execute OpenBoardHelp / OpenBoardExitConfirmation exactly as before.
  #   4. R4 (help content): the board help lines (components/help_content.rs) are updated: the '←/h, →/l' row reads 'Cycle columns → menu items → chips' (was 'Switch column'); two NEW rows are added — 'Enter on menu item  Opens its dropdown menu' and 'Enter on chip      Opens that session'; the 'u' row reads 'Menu bar help' (was 'Actions'). The agent help lines gain one row: '← (empty input) Enter the menu bar' (MENU-003 R3 entry key).
  #   5. R5 (dead-shim deletion): views/board/board_shortcuts.rs (the SHORTCUTS shim + CHORD_HINT) and views/board/keybinding_shortcuts.rs (the now-dead chord-row painter) are DELETED (user-confirmed). The board_shortcuts module declaration + re-export are removed from views/board.rs. The 'shortcuts_mirror_the_menu_registry' drift test disappears with the file — the registry (components/menu_bar/items.rs) IS now the single source of truth with no mirror to drift from. Source-shape pins that referenced the files migrate: rpc015 'Header widget modules exist' + 'modules stay under 300 lines' no longer require keybinding_shortcuts.rs; source_shape_board_search_board022's CHORD_HINT pin is superseded (re-pinned against the bar/registry); view_board_unit_rpc015 keeps its superseded bar assertions.
  #   6. R6 (test/snapshot migration): (1) the board_actions_dialog_board023 suite migrates to the 9-row registry body (new DIALOG_ROWS constant; row-offset tests renumbered: Enter-on-Mux at index 5, Help at 7, Exit at 8; the 'no trigger row' scenario reworded — still no 'u' row, now exactly 9 rows, footer-title gap 12) [MENU-008: re-migrated to the 10-row registry body — Mux now index 6, Help 8, Exit 9]; (2) the two header-row snapshots (app_with_mock_backend__help_dialog_dismissed.snap + repl_bootstrap_rpc012.snap) are regenerated if they capture board/agent frames the bar changed; (3) rpc014-source-shape board split verified unchanged; (4) rpc013-source-shape keeps passing flag-off + gains a new scenario pinning the 6-constraint flag-on list (pane_layout_constraints_menu); (5) 'u Actions' chord-string assertions in rpc015/mux009/board022/rpc395/board021/source_shape_board_search_board022 are already superseded by the live bar — comments/docstrings refreshed where they still describe the deleted CHORD_HINT; (6) the board footer hint string is verified unchanged (MENU-002 already migrated it).
  #   7. R7 (tags + supersession + quality): the @menu-bar tag is ALREADY registered (MENU-001..004 era, Technical Tags) — verify it covers this feature (the MENU-005 feature file gains @menu-bar + @menu-registry + @tui-component tags; no new tag needed, @menu-dropdown already exists). The BOARD-023 feature file gains @superseded notes on R10/R11 (a docstring note that the live bar + registry supersede the chord hint + SHORTCUTS table; file stays for coverage history, scenarios untouched). Final quality pass: fspec validate + validate-tags + scoped cargo check/clippy/test -p codelet-fspec-tui (lib + the migrated integration suites), snapshot regeneration in one pass.
  #
  # EXAMPLES:
  #   1. On the board the user presses 'u': a centered dialog titled 'Menu bar' (Yellow border) lists the registry rows in order (10 after MENU-008 — '. New Agent', '/ Search', 'D FOUNDATION.md', 'A Attachments', 'F Changed Files', 'C Checkpoints', 'M Mux', 'P Providers', '? Help', 'Esc Exit' — was 9) — each with its one-line description; the first row is highlighted.
  #   2. The board help dialog ('?') now reads, in its key column: '←/h, →/l   Cycle columns → menu items → chips', a 'Enter on menu item  Opens its dropdown menu' row, an 'Enter on chip      Opens that session' row, and a 'u   Menu bar help' row replacing the old 'u   Actions' row.
  #   3. With the 'Menu bar' dialog open, the user presses Enter on the 'A Attachments' row (registry index 6, newly present because the registry carries the A arm the old SHORTCUTS table lacked): the attachment picker action fires exactly as the bare 'a' board key would, and the dialog closes.
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to open the menu-bar help overlay from 'u' and read accurate board/agent keybinding help
    So that the 'u' dialog, the menu bar, the dropdown and the bare-key docs are generated from one registry so they can never drift, and the whole test/snapshot suite is migrated to the new bar

  @menu-bar
  @menu-registry
  Scenario: u opens the Menu bar dialog listing all ten registry rows in order
    Given I am in the single Board view
    When I press the 'u' key
    Then the 'Menu bar' dialog is open on the compositor with a Yellow accent border
    And the body lists 10 rows in registry order: . New Agent, / Search, D FOUNDATION.md, A Attachments, F Changed Files, C Checkpoints, M Mux, P Providers, ? Help, Esc Exit — each with its one-line description
    And the first row ('. New Agent') is highlighted

  @menu-bar
  @menu-registry
  # NOTE (BUG-203 supersedes this scenario): the 'u' dialog is now payload-free
  # — Enter on the '. New Agent' row ALWAYS mounts the CreateSessionDialog
  # (registry MenuAction::NewAgent → OpenCreateSessionDialog { preselect: None
  # }); it never substitutes a session snapshot or re-enters a session (that is
  # the Shift+Right CYCLE gesture's job). See the BUG-203 feature.

  @menu-bar
  @menu-registry
  Scenario: Enter on the New Agent row substitutes the session snapshot (superseded by BUG-203 — the row now mounts the CreateSessionDialog)
    Given a board with a focused work unit that has an open agent session
    When I open the 'Menu bar' dialog with 'u' and press Enter on the '. New Agent' row
    Then the dialog is closed
    And the CreateSessionDialog mounts over the board (BUG-203 supersedes: the row NEVER substitutes a session snapshot — it always starts a new agent)

  @menu-bar
  @menu-registry
  Scenario: Enter on the A Attachments row fires the attachment picker
    Given I am in the single Board view with a selected work unit that has attachments
    When I open the 'Menu bar' dialog and press Enter on the 'A Attachments' row
    Then the dialog is closed
    And the attachment picker opens for the selected work unit — the same outcome as pressing 'a' directly on the board

  @menu-bar
  Scenario: Up from the first row wraps to the last row and Down wraps back
    Given the 'Menu bar' dialog is open with '. New Agent' (the first row) highlighted
    When I press the Up arrow
    Then 'Esc Exit' (the last of the 10 rows) is highlighted
    When I press the Down arrow
    Then '. New Agent' is highlighted again

  @menu-bar
  Scenario: The board help dialog lists the updated keybinding rows
    Given I am in the single Board view
    When I open the board help dialog with '?'
    Then the help lists the '←/h, →/l' row as 'Cycle columns → menu items → chips'
    And a row 'Enter (item)' described as 'Opens its dropdown menu'
    And a row 'Enter (chip)' described as 'Opens that session'
    And the 'u' row reads 'Menu bar help' instead of 'Actions'

  @menu-bar
  Scenario: The agent help lists the empty-input left-arrow menu bar entry
    Given I am in the Agent view
    When I open the agent help with '/help'
    Then the help lists a row '← empty input' described as 'Enter the menu bar'

  @menu-bar
  Scenario: The board footer hint keeps the ring wording
    Given I am in the single Board view
    When I look at the board footer row
    Then it reads '← → Cycle Columns → menu items → chips' unchanged (MENU-002 wording)

  @menu-bar
  @source-shape
  Scenario: The dead chord shim files are deleted and the dialog module renamed
    Given the rust/fspec-tui crate after MENU-005 lands
    When a developer scans the views/board/ and components/ directories
    Then the file rust/fspec-tui/src/views/board/board_shortcuts.rs does NOT exist
    And the file rust/fspec-tui/src/views/board/keybinding_shortcuts.rs does NOT exist
    And views/board.rs no longer declares the board_shortcuts module
    And rust/fspec-tui/src/components/menu_bar_help.rs exists and no longer references the old SHORTCUTS table

  @menu-bar
  @source-shape
  Scenario: The agent pane flag-on layout pins the 6-constraint list
    Given the AgentView pane layout in rust/fspec-tui/src/views/agent/pane_render.rs
    When a developer reads pane_layout_constraints_menu
    Then it returns [Length(1), Length(1), Length(role_height), Min(0), Length(1), Length(input_height)] in that order
    And the flag-off pane_layout_constraints stays the pinned 5-list (rpc013)

  @menu-bar
  @menu-geometry
  Scenario: The board surface renders the bar (wide and narrow snapshots)
    Given I am in the single Board view with 2 open sessions
    When I render the App at 120x24
    Then the frame matches the insta snapshot 'menu005_board_120x24'
    When I render the App at 80x24
    Then the frame matches the insta snapshot 'menu005_board_80x24'

  @menu-bar
  @menu-geometry
  Scenario: The agent surface renders the bar row under the header (wide and narrow snapshots)
    Given I am in the single Agent view with 2 open sessions
    When I render the App at 120x24
    Then the frame matches the insta snapshot 'menu005_agent_120x24' with the bar row directly below the session header
    When I render the App at 80x24
    Then the frame matches the insta snapshot 'menu005_agent_80x24'

  @menu-bar
  @menu-geometry
  Scenario: The mux surface renders one top-row bar with per-pane suppression (wide and narrow)
    Given I am in mux mode with the Board | Agent grid and 2 open sessions
    When I render the App at 120x24
    Then the frame matches the insta snapshot 'menu005_mux_120x24' with the bar on the mux top row and no per-pane bars (the board pane's header row 3 is blank)
    When I render the App at 80x24
    Then the frame matches the insta snapshot 'menu005_mux_80x24'
