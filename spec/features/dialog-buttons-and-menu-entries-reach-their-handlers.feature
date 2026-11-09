@done
@tui
@rust
@interaction
@TUI-127
Feature: Dialog buttons and menu entries reach their handlers
  """
  codelet-fspec-tui only. DA-5: App::handle_open_merge_confirm_dialog builds MergeConfirmDialog with .with_action_tx(self.action_tx.clone()) (the SessionWorktreesDialog / RoleDialog pattern), so emit_outcome sends MergeConfirmed / DiscardConfirmed / CancelMergeDialog to App::dispatch; the close order after a merge stays with BUG-585. DA-6: handle_slash_debug with no session pushes NotificationDialog::info(..).with_action_tx(..) and Action::Custom (components/action_runtime.rs) is deleted; the refac019 action snapshot records the removal. DA-8: the empty-list guards in views/board/workspace_dropdown.rs handle_key and app/dispatch_workspaces.rs OpenWorkspaceDropdown are removed; the empty dropdown paints one 'No workspaces' row. Tests drive the real App dispatch over MockBackend on a paused tokio clock, from the key or click to the backend call or the rendered text.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. In the /merge-worktree dialog, choosing Merge (Enter or a click) merges the session's worktree and choosing Discard discards it: the choice reaches the App's handler, which calls the backend once and shows the result notice in the session; Cancel only closes the dialog (DA-5)
  #   2. /debug with no session open toggles the debug capture for the next session and says so in a notification dialog: 'Debug capture will be on for the next session' when it turns on and 'Debug capture is off' when it turns off; no notice is sent where nothing handles it (DA-6)
  #   3. Action::Custom is removed: nothing handles it, so nothing may send it (DA-6)
  #   4. w on the board and Settings > Workspaces always open the workspace dropdown; with no workspaces listed it shows 'No workspaces', its key hint shows only 'a add' (there is nothing to remove) and a opens the Add workspace dialog, so the first workspace can be added (DA-8)
  #
  # EXAMPLES:
  #   1. Session s-1 has worktree changes; the user runs /merge-worktree and presses Enter on [Merge]: the backend merges s-1's worktree once and nothing is discarded
  #   2. Session s-1 has worktree changes; the user runs /merge-worktree, presses Right to [Discard] and Enter: the backend discards s-1's worktree once, nothing is merged, and s-1 shows '[discard] worktree discarded'
  #   3. Session s-1 has worktree changes; the user clicks [Merge] in the /merge-worktree dialog: the backend merges s-1's worktree once
  #   4. Session s-1 has worktree changes; the user presses Esc in the /merge-worktree dialog: the dialog closes and nothing is merged or discarded
  #   5. No session is open; the user picks /debug: a notification says 'Debug capture will be on for the next session'; picking /debug again shows 'Debug capture is off'
  #   6. The daemon lists no workspaces; the user presses w on the board: the dropdown opens showing 'No workspaces', and pressing a opens the Add workspace dialog
  #   7. The daemon lists no workspaces; the user picks Settings > Workspaces: the dropdown opens showing 'No workspaces'
  #
  # QUESTIONS (ANSWERED):
  #   Q: the pre-session /debug message used to name a capture directory, but the next session is started with set_debug_enabled(true) and no directory, so the daemon picks the directory. Conservative option taken: the notification says only whether capture is on for the next session or off, and names no directory
  #   A: Conservative option taken (supervisor instruction, 2026-11-07, pending product owner review): the notification states only the new state, 'Debug capture will be on for the next session' or 'Debug capture is off', because the TUI does not choose the directory the next session writes to.
  #
  #   Q: with no workspaces listed, should w open an empty dropdown (so a: add can add the first workspace) or show a notice? Conservative option taken: open the dropdown with 'No workspaces', since only the dropdown offers a: add
  #   A: Conservative option taken (supervisor instruction, 2026-11-07, pending product owner review): w and Settings > Workspaces open the dropdown even when the list is empty; it shows 'No workspaces' with the key hint 'a add' only, so the first workspace can be added.
  #
  # ========================================
  Background: User Story
    As a user acting on a TUI dialog or menu entry
    I want to have every dialog button and menu entry I choose reach its handler and do what it says
    So that a Merge, Discard, /debug or Workspaces choice never silently closes or does nothing

  # ─────────────────────────────────────────────────────────────────────
  # /merge-worktree: every button reaches its handler (rule 1, DA-5)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Enter on Merge merges the session's worktree
    Given session s-1 is focused and its worktree has changes
    When the user runs /merge-worktree
    And presses Enter on "Merge" in the "Merge Worktree" dialog
    Then the dialog is closed
    And the backend merges the worktree of s-1 once
    And nothing is discarded

  Scenario: Enter on Discard discards the session's worktree
    Given session s-1 is focused and its worktree has changes
    When the user runs /merge-worktree
    And presses Right and Enter on "Discard" in the "Merge Worktree" dialog
    Then the backend discards the worktree of s-1 once
    And nothing is merged
    And s-1 shows "[discard] worktree discarded"

  Scenario: Clicking Merge merges the session's worktree
    Given session s-1 is focused and its worktree has changes
    When the user runs /merge-worktree
    And clicks "Merge" in the "Merge Worktree" dialog
    Then the backend merges the worktree of s-1 once

  Scenario: Esc closes the merge dialog and changes nothing
    Given session s-1 is focused and its worktree has changes
    When the user runs /merge-worktree
    And presses Esc in the "Merge Worktree" dialog
    Then the dialog is closed
    And nothing is merged or discarded

  # ─────────────────────────────────────────────────────────────────────
  # /debug with no session says what it did (rules 2-3, DA-6)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: /debug with no session open says whether capture is on for the next session
    Given no session is open
    When the user picks /debug
    Then a notification says "Debug capture will be on for the next session"
    When the user picks /debug again
    Then a notification says "Debug capture is off"

  # ─────────────────────────────────────────────────────────────────────
  # Workspaces with an empty list (rule 4, DA-8)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: w with no workspaces listed opens the dropdown so the first workspace can be added
    Given the daemon lists no workspaces
    When the user presses "w" on the board
    Then the workspace dropdown is open and shows "No workspaces"
    And its key hint shows "a add" and not "d remove"
    When the user presses "a"
    Then the Add workspace dialog is open

  Scenario: Settings > Workspaces with no workspaces listed opens the dropdown
    Given the daemon lists no workspaces
    When the user picks Settings > Workspaces
    Then the workspace dropdown is open and shows "No workspaces"
