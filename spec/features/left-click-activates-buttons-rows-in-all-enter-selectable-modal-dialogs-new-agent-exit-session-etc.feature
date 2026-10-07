@done
@dialog
@mouse-events
@modal
@agent-view
@board-view
@mux
@tui-component
@tui-112
Feature: Left-click activates buttons/rows in all Enter-selectable modal dialogs (New Agent, Exit Session, etc.)

  """
  Every modal dialog in the Rust TUI that commits a choice with Enter must
  also commit it with a single mouse left-click (user decision 2026-10-07):
  one click on a button/row performs exactly what Left/Right + Enter does
  for that choice — no separate select-then-activate step.

  Scope (R1-R3):
    * Three-button dialogs (R1): Start New Agent? (CreateSessionDialog),
      Exit Session? (ExitConfirmationDialog), Exit fspec?
      (BoardExitConfirmationDialog) — left-click a button commits it.
    * Shared-button-row dialogs (R2): ConfirmDialog, MergeConfirmDialog,
      SessionWorktreesDialog — left-click a button returns that outcome.
    * Row-list dialogs (R3): Thinking Level, Attachment Picker, Mux Layout,
      Role (draft row), Work Unit Search — left-click a row commits it.

  Hit-testing (R4) uses each dialog's last-rendered rect (the
  `Option<Rect>` + `mouse::hit_test::rect_contains` pattern already used by
  WorkUnitSearchDialog); clicks outside the rect stay Ignored so they bubble
  to whatever is behind the modal, and wheel behavior is unchanged.
  Clicks on non-activatable cells (gaps, title, description, footer,
  border — R5) are Ignored.

  OUT OF SCOPE: dialogs with no activatable buttons (Help, Loading,
  Notification, Status, Error, Disconnect, Checkpoint-restore);
  ModelSelector rows (separate click semantics owned by the model-selector
  view); scrollbar drag (existing behavior, untouched).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: A left-click on a three-button dialog button (CreateSessionDialog, ExitConfirmationDialog, BoardExitConfirmationDialog) commits THAT button — the exact action Left/Right + Enter produces for it — and the dialog removes itself (one click, no separate select-then-activate step).
  #   2. R2: A left-click on a button of a shared-button-row dialog (ConfirmDialog, MergeConfirmDialog, SessionWorktreesDialog — views/agent) returns the SAME outcome enum variant as Left/Right + Enter on that button (Primary/Secondary/Cancel, Merge/Discard/Cancel, Prune/Cancel).
  #   3. R3: A left-click on a row of a row-list dialog (ThinkingLevelDialog, AttachmentPickerDialog, MuxConfigDialog, RoleDialog, WorkUnitSearchDialog) commits that row immediately (ONE-click): it selects AND emits the same Action the Enter key would emit for that row — no separate select-then-activate step. (User decision; matches the R1/R2 one-click semantics, diverging from the checkpoints RPC-369 select-only precedent.)
  #   4. R4: Mouse hit-testing uses the dialog's last-rendered rect (the `Cell<Option<Rect>>` + `mouse::hit_test::rect_contains` pattern, as in WorkUnitSearchDialog and the model selector). A left-click OUTSIDE the dialog's rect is Ignored (bubbles to whatever is behind the dialog); wheel behavior of every dialog is unchanged.
  #   5. R5: A left-click inside the dialog rect but NOT on any activatable control (gaps between buttons, the title row, description text, footer, border) is Ignored — nothing is selected, committed, or dismissed. A left-click in the gap BETWEEN two three-button dialog buttons does nothing (buttons carry the exact ` <label> ` cells they paint, no bleed).
  #
  # EXAMPLES:
  #   1. CreateSessionDialog (Start New Agent?): user left-clicks the 'Yes - Isolated' button directly without touching the keyboard; the dialog emits Action::CreateSessionSubmitted{isolated:true} and removes itself — same as Left,Left,Enter.
  #   2. Exit Session? dialog (Esc cascade level 7, idle agent): user left-clicks 'Close Session' once; the session closes and the view returns to the board — identical to Right,Right,Enter.
  #   3. Exit fspec? dialog: user left-clicks the 'Exit' button once; the app quits (Action::Quit emitted, dialog removed). Left-clicking the 'Cancel' button dismisses the dialog and stays on the board (no action emitted).
  #   4. MergeConfirmDialog (agent view, worktree has unmerged changes): user left-clicks the 'Discard' button once; the view receives the Discard outcome (the same outcome Left/Right+Enter on Discard produces) and proceeds with the discard. Left-clicking 'Cancel' dismisses without acting.
  #   5. SessionWorktreesDialog (agent view): user left-clicks the 'Prune' button once; the view receives the Prune outcome (same as Left/Right+Enter on Prune) and prunes the listed detached worktrees. Left-clicking 'Cancel' dismisses without acting.
  #   6. Thinking Level dialog: user left-clicks the 'High' row once; the level is set to High (Action::ThinkingLevelSelected emitted) and the dialog closes — same as Down,Down,Enter from Off.
  #   7. Mux Layout dialog: user left-clicks the 'Pane 2' row once; the cursor jumps to Pane 2 AND the dialog applies (same as navigating to Pane 2 + Enter). Left-clicking the 'Orientation' row applies with the cursor on Orientation.
  #   8. Role dialog: with a draft of 'code reviewer', user left-clicks the draft row once; the role is saved (same as Enter) and the dialog closes. Left-clicking the title/border regions does nothing.
  #   9. Attachment picker (work unit has attachments design.md and notes.pdf): user left-clicks the 'notes.pdf' row once; that attachment opens in the browser (Action::OpenAttachment with the full path) and the dialog closes.
  #   10. Work unit search dialog (board view): user types a query, then left-clicks the 'MENU-009' result row once; that work unit is selected/acted on exactly as pressing Enter would — no double-click needed.
  #
  # ========================================

  Background: User Story
    As a TUI user
    I want to activate any Enter-selectable dialog choice (button or row) with a mouse left-click
    So that the dialogs I can confirm with Enter can also be confirmed by clicking, with no keyboard required

  # ========================================================
  # R1 — three-button dialogs: click commits that button
  # ========================================================

  Scenario: Start New Agent — clicking Yes - Isolated commits it
    Given the "Start New Agent?" dialog is open with "Yes" highlighted
    When the user left-clicks the "Yes - Isolated" button
    Then a non-isolated session creation is NOT emitted
    And an isolated session creation is emitted
    And the dialog is removed

  Scenario: Start New Agent — clicking Cancel dismisses without creating
    Given the "Start New Agent?" dialog is open
    When the user left-clicks the "Cancel" button
    Then no session is created
    And the dialog is removed

  Scenario: Start New Agent — keyboard behavior is unchanged
    Given the "Start New Agent?" dialog is open with "Yes" highlighted
    When the user presses Right then Enter
    Then an isolated session creation is emitted
    And the dialog is removed

  Scenario: Exit Session — clicking Close Session commits it
    Given the "Exit Session?" dialog is open with "Detach" highlighted
    When the user left-clicks the "Close Session" button
    Then the exit choice CloseSession is committed
    And the dialog is removed

  Scenario: Exit Session — clicking Detach commits it
    Given the "Exit Session?" dialog is open with "Detach" highlighted
    When the user left-clicks the "Detach" button
    Then the exit choice Detach is committed
    And the dialog is removed

  Scenario: Exit fspec — clicking Exit commits quit
    Given the "Exit fspec?" dialog is open with "Exit" highlighted
    When the user left-clicks the "Exit" button
    Then the app quit action is emitted
    And the dialog is removed

  Scenario: Exit fspec — clicking Cancel stays on the board
    Given the "Exit fspec?" dialog is open with "Exit" highlighted
    When the user left-clicks the "Cancel" button
    Then no quit action is emitted
    And the dialog is removed
    And the user remains on the board

  # ========================================================
  # R2 — shared-button-row dialogs: click returns that outcome
  # ========================================================

  Scenario: Merge confirm — clicking Discard returns Discard
    Given the merge-confirm dialog is open with "Merge" focused
    When the user left-clicks the "Discard" button
    Then the outcome is Discard
    And the dialog is dismissed

  Scenario: Merge confirm — clicking Cancel returns Cancel
    Given the merge-confirm dialog is open with "Merge" focused
    When the user left-clicks the "Cancel" button
    Then the outcome is Cancel

  Scenario: Session worktrees — clicking Prune returns Prune
    Given the session-worktrees dialog is open with "Prune" focused
    When the user left-clicks the "Prune" button
    Then the outcome is Prune

  Scenario: Session worktrees — clicking Cancel returns Cancel
    Given the session-worktrees dialog is open with "Prune" focused
    When the user left-clicks the "Cancel" button
    Then the outcome is Cancel

  Scenario: Confirm dialog — clicking the primary button returns Primary
    Given a two-button confirm dialog (Delete / Cancel) is open with "Delete" focused
    When the user left-clicks the "Cancel" button
    Then the outcome is Cancel

  # ========================================================
  # R3 — row-list dialogs: click commits that row (one-click)
  # ========================================================

  Scenario: Thinking Level — clicking High commits High
    Given the Thinking Level dialog is open with "Off" highlighted
    When the user left-clicks the "High" row
    Then the thinking level High is committed for the session
    And the dialog is closed

  Scenario: Mux Layout — clicking Pane 2 applies with the cursor on Pane 2
    Given the Mux Layout dialog is open with the cursor on "Enabled"
    When the user left-clicks the "Pane 2" row
    Then the mux config is applied
    And the cursor is on the "Pane 2" row

  Scenario: Mux Layout — clicking Orientation applies with the cursor on Orientation
    Given the Mux Layout dialog is open with the cursor on "Enabled"
    When the user left-clicks the "Orientation" row
    Then the mux config is applied
    And the cursor is on the "Orientation" row

  Scenario: Role dialog — clicking the draft row saves the role
    Given the Role dialog is open with a non-empty draft
    When the user left-clicks the draft row
    Then the role is saved
    And the dialog is closed

  Scenario: Attachment picker — clicking notes.pdf opens it
    Given the attachment picker is open with "design.md" and "notes.pdf" listed
    When the user left-clicks the "notes.pdf" row
    Then the attachment "notes.pdf" full path is opened
    And the dialog is closed

  Scenario: Work unit search — clicking a result row commits it
    Given the work-unit search dialog is open showing a "MENU-009" result row
    When the user left-clicks the "MENU-009" row
    Then the work unit MENU-009 is selected
    And the dialog is closed

  # ========================================================
  # R4/R5 — hit-testing & non-activatable cells
  # ========================================================

  Scenario: A click outside the dialog rect is ignored
    Given the "Start New Agent?" dialog is open
    When the user left-clicks a cell outside the dialog's rendered rect
    Then no action is emitted
    And the dialog stays open

  Scenario: A click in the gap between two buttons does nothing
    Given the "Start New Agent?" dialog is open
    When the user left-clicks a cell in the gap between two buttons
    Then no action is emitted
    And the dialog stays open
    And no button is committed

  Scenario: A click on the title, description or footer does nothing
    Given the "Start New Agent?" dialog is open
    When the user left-clicks a cell on the title row, a description cell, or a footer cell
    Then no action is emitted
    And the dialog stays open

  Scenario: Wheel navigation behavior is unchanged
    Given the "Start New Agent?" dialog is open with "Yes" highlighted
    When the user scrolls right then scrolls left
    Then "Yes" is highlighted again
    And no action is emitted
