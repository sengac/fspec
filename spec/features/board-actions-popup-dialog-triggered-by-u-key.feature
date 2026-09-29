@done
@board-023
@board-view
@dialog
@keyboard-navigation
@ui-enhancement
@tui
@BOARD-023
Feature: Board actions popup dialog triggered by the u key
  """
  New Action::OpenBoardKeybindingDialog variant (components/mod.rs Action enum, next to OpenMuxConfigDialog) emitted by BoardView via views/board/keys.rs handle_mode_view_key on modifier-free 'u'/'U' (case-insensitive — the arm matches BOTH KeyCode::Char('u') and KeyCode::Char('U') with a no-CONTROL guard, mirroring the a/c/f/d/m pattern). App::dispatch gains a new app/dispatch_board_keybinding.rs try_dispatch arm calling handle_open_board_keybinding_dialog() — idempotent on dialog id 'board-actions-dialog' (compositor.contains guard), seeds the dialog with the selected work unit's session target (Option<SessionId> snapshot at open time). The dialog (components/board_keybinding_dialog.rs) is titled 'Actions' and lists exactly 8 rows — the six board shortcuts from views/board/board_shortcuts.rs (C/F/D/./ /M) then '? Help' then 'Esc Exit'; there is NO trigger row inside the dialog (it is already open; Esc closes it). Two further Action variants OpenBoardHelp and OpenBoardExitConfirmation route to the SAME push helpers the stage-4 App shortcuts in app/events.rs use for '?' (HelpDialog::for_board) and Esc (BoardExitConfirmationDialog) — one shared push helper each (DRY).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: pressing modifier-free 'u' or 'U' (case-insensitive, mirroring the a/c/f/d/m key arms) in the single Board view opens the BoardKeybindingDialog (Priority::Foreground, shared dialog_theme renderer, Yellow accent, stable id 'board-actions-dialog'). The key is always consumed (no fall-through).
  #   2. R2: the trigger is modifier-free only — Ctrl+U (or any Ctrl-chorded u/U) must NOT open the dialog and falls through to the existing App-level handling, consistent with the a/c/f/d/m guard pattern in BoardView. While the dialog is open, pressing 'u'/'U' again is a consumed no-op (idempotent — never stacks a second layer).
  #   3. R3: the dialog body lists exactly 8 rows, in this order: C Checkpoints (open the checkpoints view), F Changed Files (open the changed-files view), D FOUNDATION.md (open in the browser), . New Agent (start/open an agent for the focused work unit), / Search (search work units), M Mux (open the mux layout config), ? Help (full keybinding help), Esc Exit (confirm exiting fspec). There is NO trigger row (no 'u Actions' row) — the dialog is already open and Esc closes it. Each row shows the key, the label, and a one-line description via the shared label_description_row renderer; the first row is pre-selected.
  #   4. R4: Up/Down arrows move the selection with wrap-around (Up from row 0 goes to row 7; Down from row 7 goes to row 0); the mouse wheel drives the same movement (MuxConfigDialog R4 parity).
  #   5. R5: Enter on a highlighted row emits the SAME Action the bare board key would emit and closes the dialog: C -> OpenCheckpointsView, F -> OpenChangedFilesView, D -> OpenFoundation, . -> OpenAgentView (with the session target snapshotted at dialog open — the modal blocks board selection while open), / -> OpenWorkUnitSearch, M -> OpenMuxConfigDialog. There is no close-only trigger row (R3).
  #   6. R6: Enter on the '? Help' row emits a new Action::OpenBoardHelp and closes the dialog; Enter on the 'Esc Exit' row emits a new Action::OpenBoardExitConfirmation and closes the dialog. These two actions are routed by App::dispatch to the SAME push helpers the stage-4 App shortcuts use for '?' (HelpDialog::for_board) and Esc (BoardExitConfirmationDialog) — one shared push helper per dialog, no duplicated push logic (DRY). The board help lines (help_content.rs) gain a 'u' row advertising this dialog ('u Actions').
  #   7. R7: the dialog is a TRUE MODAL (BUG-161 parity with WorkUnitSearchDialog): every key it does not explicitly handle (j/k/h/l, [, ], Enter on the board, Shift+arrows, Ctrl-chords, pastes) is CONSUMED as a no-op so the BoardView behind it stays frozen; no key ever leaks through the Compositor to the board while the dialog is open.
  #   8. R8: the Esc key ALWAYS closes the dialog (standard modal convention) and NEVER triggers the exit confirmation — even when the 'Esc Exit' row is highlighted. The only way to open the exit confirmation from this dialog is an explicit Enter on the 'Esc Exit' row. Likewise the '?' key is blocked by the modal (R7); only Enter on the '? Help' row opens the HelpDialog. This keeps Esc from creating a double-step accidental-quit path (exit confirmation opens with 'Exit' pre-selected).
  #   9. R9: when mux is active and the focused pane is the Board pane, the same modifier-free 'u'/'U' opens the dialog overlaying the whole grid (MUX-001 R9: dialogs overlay the mux) — the single binding in BoardView::handle_event serves both surfaces (MUX-009 precedent, same routing through views/multiplex/keys.rs).
  #   10. R10: the board header's chord row (row 3 of the header strip, views/board/keybinding_shortcuts.rs) is REPLACED by a short hint 'u Actions' (single plain primary-fg span, same styling as before). The full chord (C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search ◆ M Mux) no longer renders in the header.
  #   11. R11: the board shortcut list (key display, label, description, action variant) is a SINGLE source of truth (one table, e.g. views/board/board_shortcuts.rs) consumed by BOTH the dialog's row builder AND the board key arms' documentation — the dialog can never drift from the actual key handling again. The SHORTCUTS table stays the six bare board shortcuts (no trigger entry); the header hint constant CHORD_HINT is 'u Actions'.
  #   12. R12: the dialog title reads 'Actions' (not 'Board Keys' or any 'key bindings' phrasing) — painted via the shared dialog_theme renderer as the bold accent inner title.
  #   13. Board view only (single Board view + focused Board pane in mux mode). The top-row chord being collapsed only exists on the board; the Agent view keeps its own ? help and slash popup. The binding lives in BoardView::handle_event (views/board/keys.rs), which is exactly what the mux Board pane routes through.
  #
  # EXAMPLES:
  #   1. In the single Board view, the user presses 'u' and sees a centered 'Actions' dialog listing every board shortcut with a one-line description; the first row ('C Checkpoints') is highlighted. The board is still visible underneath.
  #   2. In the single Board view, the user presses Ctrl+U. The Actions dialog does NOT open — Ctrl-chorded u/U falls through to the existing App-level handling, exactly like Ctrl+M does today.
  #   3. With the Actions dialog open, the user presses Down three times: the highlight moves from 'C Checkpoints' to 'F Changed Files' to 'D FOUNDATION.md' to '. New Agent'. Pressing Up once returns the highlight to 'D FOUNDATION.md'.
  #   4. With the Actions dialog open, the user navigates to 'F Changed Files' and presses Enter. The three-pane Changed Files view opens (the board switches to it) and the Actions dialog is closed — exactly the same outcome as pressing 'f' directly on the board.
  #   5. With the Actions dialog open, the user navigates to the 'Esc Exit' row and presses Enter. The Actions dialog closes and the 'Exit fspec?' confirmation dialog opens (Exit pre-selected) — the same confirmation the plain Esc key on the board opens. Pressing Esc inside the confirmation cancels it and the user stays on the board.
  #   6. With the Actions dialog open and the 'C Checkpoints' row highlighted, the user presses Esc. The Actions dialog closes WITHOUT opening the exit confirmation and without opening the checkpoints view — the user stays on the board. (Esc is 'close this dialog', never 'execute the highlighted row'.)
  #   7. With the Actions dialog open, the user presses 'j' and 'f'. The board behind the dialog stays frozen: the selection does not move and the changed-files view does NOT open — every key the dialog does not explicitly handle is swallowed (true-modal blocking, BUG-161 parity).
  #   8. In the single Board view, the user looks at the board header's top-right area: the keybinding hint row reads 'u Actions' (a single short hint in plain foreground style) — the old six-action chord (C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search ◆ M Mux) is no longer rendered there.
  #   9. In the single Board view, the user opens the board help with '?'. The help lists a 'u Actions' row advertising that 'u' opens the actions dialog — no separate 'u' entry appears in the Actions dialog itself.
  #   10. In mux mode (Board | Agent grid) with the Board pane focused, the user presses 'u'. The Actions dialog opens over the whole grid (dialogs overlay the mux). Pressing Up until 'M Mux' and Enter opens the Mux config dialog over the grid, seeded from the live mux config — the same dialog bare /mux opens.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the trigger ONLY fire from the Board view (single Board view AND the focused Board pane in mux mode), or should it ALSO open the dialog from the Agent view?
  #   A: Board view only (single Board view + focused Board pane in mux mode). The top-row chord being collapsed only exists on the board; the Agent view keeps its own ? help + slash popup. The binding lives in BoardView::handle_event (views/board/keys.rs), which is exactly what the mux Board pane routes through.
  #   Q: Is 'u' case-sensitive?
  #   A: No — 'u' and 'U' both open the dialog (same as the other board shortcuts, case-insensitive; the arm matches both KeyCode::Char('u') and KeyCode::Char('U')).
  #   Q: Why is there no trigger row inside the dialog?
  #   A: The dialog is already open by the time you see it — a 'u Actions' row would only describe how you got there. Esc closes the dialog (R8), so no in-dialog affordance is needed. The trigger is advertised in the board header hint ('u Actions', R10) and in the board help lines (R6) instead.
  #
  # ========================================

  Background: User Story
    As a developer working from the Board view
    I want to press the 'u' key to open an actions popup dialog explaining every board shortcut and select one with the up/down arrows
    So that I can discover and trigger board actions without memorizing the top-row chord, and the header stays uncluttered

  @BOARD-023
  @dialog
  @board-view
  Scenario: u opens the actions dialog from the single Board view
    Given I am in the single Board view with mux disabled
    When I press the 'u' key
    Then the actions dialog is open overlaying the board with a Yellow accent border and the title 'Actions'
    And the dialog body lists 8 rows in order: C Checkpoints, F Changed Files, D FOUNDATION.md, . New Agent, / Search, M Mux, ? Help, Esc Exit — each with a one-line description
    And the first row ('C Checkpoints') is highlighted and the board view is still visible underneath

  @BOARD-023
  @dialog
  @board-view
  Scenario: U opens the actions dialog (case-insensitive trigger)
    Given I am in the single Board view with mux disabled
    When I press the 'U' key (uppercase)
    Then the actions dialog is open overlaying the board with the title 'Actions'
    And the first row ('C Checkpoints') is highlighted

  @BOARD-023
  @dialog
  @board-view
  Scenario: Ctrl+U does not open the actions dialog
    Given I am in the single Board view with mux disabled
    When I press Ctrl+U (Ctrl-chorded uppercase U)
    Then the actions dialog is not open
    And the key falls through to the App-level handling

  @BOARD-023
  @dialog
  @board-view
  Scenario: The actions dialog body lists all board shortcuts with descriptions
    Given I am in the single Board view
    When I open the actions dialog with 'u'
    Then the dialog shows 8 rows in order: "C Checkpoints", "F Changed Files", "D FOUNDATION.md", ". New Agent", "/ Search", "M Mux", "? Help", "Esc Exit"
    And each row shows its key, a label, and a one-line description
    And the footer reads "↑↓ Navigate │ Enter Execute │ Esc Close"

  @BOARD-023
  @dialog
  @board-view
  Scenario: The actions dialog has no trigger row
    Given I am in the single Board view
    When I open the actions dialog with 'u'
    Then the dialog shows exactly 8 rows
    And no row in the dialog body advertises the 'u' trigger key
    And the dialog body does not contain the text "Actions" as a row label other than the title

  @BOARD-023
  @dialog
  @board-view
  Scenario: Down moves the selection to the next row
    Given the actions dialog is open with "C Checkpoints" highlighted
    When I press the Down arrow
    Then "F Changed Files" is highlighted

  @BOARD-023
  @dialog
  @board-view
  Scenario: Up moves the selection to the previous row
    Given the actions dialog is open with "F Changed Files" highlighted
    When I press the Up arrow
    Then "C Checkpoints" is highlighted

  @BOARD-023
  @dialog
  @board-view
  Scenario: Up from the first row wraps to the last row
    Given the actions dialog is open with "C Checkpoints" (the first row) highlighted
    When I press the Up arrow
    Then "Esc Exit" (the last row) is highlighted

  @BOARD-023
  @dialog
  @board-view
  Scenario: Down from the last row wraps to the first row
    Given the actions dialog is open with "Esc Exit" (the last row) highlighted
    When I press the Down arrow
    Then "C Checkpoints" (the first row) is highlighted

  @BOARD-023
  @dialog
  @board-view
  Scenario: Enter executes the highlighted action and closes the dialog
    Given the actions dialog is open with "F Changed Files" highlighted
    When I press Enter
    Then the actions dialog is closed
    And the Changed Files view opens — the same outcome as pressing 'f' directly on the board

  @BOARD-023
  @dialog
  @board-view
  Scenario: Enter on the Mux row opens the Mux config dialog
    Given the actions dialog is open with "M Mux" highlighted
    When I press Enter
    Then the actions dialog is closed
    And the Mux config dialog is open seeded from the live mux config — the same dialog bare /mux opens

  @BOARD-023
  @dialog
  @board-view
  Scenario: Enter on the Help row opens the board help dialog
    Given the actions dialog is open with "? Help" highlighted
    When I press Enter
    Then the actions dialog is closed
    And the board Help dialog (the one '?' opens) is open on top

  @BOARD-023
  @dialog
  @board-view
  Scenario: Enter on the Exit row opens the exit confirmation
    Given the actions dialog is open with "Esc Exit" highlighted
    When I press Enter
    Then the actions dialog is closed
    And the "Exit fspec?" confirmation dialog is open with Exit pre-selected
    And pressing Esc inside the confirmation cancels it and I stay on the board

  @BOARD-023
  @dialog
  @board-view
  Scenario: Esc closes the dialog without executing the highlighted row
    Given the actions dialog is open with "C Checkpoints" highlighted
    When I press Esc
    Then the actions dialog is closed
    And the checkpoints view is NOT open
    And the exit confirmation is NOT open
    And I am still on the board

  @BOARD-023
  @dialog
  @board-view
  Scenario: Keys behind the dialog never leak to the board
    Given the actions dialog is open
    And a work unit below the current selection is available in the focused column
    When I press 'j' then 'f'
    Then the board selection has NOT moved
    And the Changed Files view has NOT opened
    And the actions dialog is still open

  @BOARD-023
  @dialog
  @board-view
  Scenario: Pressing u again while open is a no-op
    Given the actions dialog is open
    When I press 'u' again
    Then the actions dialog remains open exactly once (no stacked second layer)

  @BOARD-023
  @dialog
  @board-view
  Scenario: The board header chord row is replaced by a short Actions hint
    Given I am in the single Board view
    When I look at the board header's keybinding hint row
    Then the row reads "u Actions" as a single plain foreground span
    And the old chord "C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search ◆ M Mux" is no longer rendered in the header
    And the checkpoint status row above it is unchanged

  @BOARD-023
  @dialog
  @board-view
  Scenario: u opens the actions dialog from the focused Board pane in mux mode
    Given I am in mux mode with the Board | Agent grid and the Board pane focused
    When I press the 'u' key
    Then the actions dialog is open overlaying the whole grid
    And the view remains Mux while the dialog is open

  @BOARD-023
  @dialog
  @board-view
  Scenario: The board help dialog advertises the u keybinding
    Given I am in the single Board view
    When I open the board help dialog with '?'
    Then the help lists a row advertising 'u' opens the actions dialog (key column 'u', label 'Actions')
