@done
@navigation
@rpc
@agent-view
@dialog
@tui
@RPC-098
Feature: Port AgentView ESC exit confirmation dialog (Detach/Close Session/Cancel) from TS Ink to Rust ratatui

  """
  New file: codelet/fspec-tui/src/components/exit_confirmation_dialog.rs (≈250 LoC) — modelled on create_session_dialog.rs but with ExitChoice enum {Detach, CloseSession, Cancel}, Accent::Yellow, Priority::Critical, ID 'exit-confirmation-dialog', is_busy-driven description, no work-unit binding.
  New Action variants in codelet/fspec-tui/src/components/mod.rs: enum ExitChoice {Detach, CloseSession, Cancel}; Action::AgentExitChoice { choice: ExitChoice }. Register the dialog module via `pub mod exit_confirmation_dialog;` in components/mod.rs.
  Modify codelet/fspec-tui/src/app/dispatch_rpc051.rs L7 fall-through (lines 55-65). Instead of `Action::BackToBoard`, push an ExitConfirmationDialog::new(is_busy).with_action_tx(self.action_tx.clone()) onto self.compositor. Guard with `if !self.compositor.contains(EXIT_CONFIRMATION_DIALOG_ID)` to prevent double-push.
  New file: codelet/fspec-tui/src/app/dispatch_rpc098.rs — impl App { fn handle_agent_exit_choice(&mut self, choice: ExitChoice) }. Cancel = no-op. Detach = send Action::BackToBoard. CloseSession = tokio::spawn(backend.destroy_session(id)) pushed onto self.pending_tasks, then send Action::BackToBoard. Wire in dispatch_rpc022.rs alongside the other Agent* actions: Action::AgentExitChoice { choice } => self.handle_agent_exit_choice(choice).
  Test plan: (a) component unit tests in components/exit_confirmation_dialog.rs covering priority, id, default selection, cyclic left/right, Enter on each option emits correct Action::AgentExitChoice, ESC emits Cancel; (b) two insta snapshot tests on 80x24 TestBackend (is_busy=true and is_busy=false); (c) integration test tests/agentview_esc_exit_confirmation_rpc098.rs covering App-level cascade routing — verifies L7 pushes dialog, no-session-skips-dialog, Compacting routes to L4 interrupt instead, no-double-push idempotence, and the three choice outcomes (Detach→BackToBoard, CloseSession→destroy_session+BackToBoard, Cancel→stays).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When ESC reaches Esc-cascade level 7 (no popup, no mode view, no running stream, empty input) and a session exists, the App pushes an ExitConfirmationDialog onto the compositor instead of dispatching Action::BackToBoard directly
  #   2. When ESC reaches Esc-cascade level 7 and NO session exists (current_session() is None), the App still dispatches Action::BackToBoard directly with no dialog (matches TS 'if (currentSessionId)' guard)
  #   3. ExitConfirmationDialog delegates rendering to dialog_theme::render_dialog with Accent::Yellow, title 'Exit Session?' (bold), Priority::Critical, and the canonical id 'exit-confirmation-dialog'
  #   4. Description text is conditional on session status: 'The agent is currently running. Choose how to exit.' when the session status is Running or Compacting, else 'Choose how to exit the session.'
  #   5. The dialog renders three flat buttons in order [Detach, Close Session, Cancel], selected button styled bg=Blue/fg=White/Bold with label padded ' <label> ', unselected fg=Gray, footer '← → Navigate | Enter Select | Esc Cancel', default selection Detach (index 0)
  #   6. Left/Right arrow keys cyclically navigate selection: Left from Detach wraps to Cancel; Right from Cancel wraps to Detach
  #   7. Enter emits Action::AgentExitChoice { choice } with the currently selected option (Detach | CloseSession | Cancel) and removes the dialog from the compositor via Callback
  #   8. ESC inside the dialog is equivalent to Cancel: emits Action::AgentExitChoice { choice: Cancel } and removes the dialog
  #   9. Dispatcher: AgentExitChoice::Cancel is a no-op (dialog already removed, AgentView remains active); AgentExitChoice::Detach dispatches Action::BackToBoard without calling backend.destroy_session; AgentExitChoice::CloseSession spawns backend.destroy_session(current_session_id) as a pending task THEN dispatches Action::BackToBoard
  #   10. Pressing ESC again while the dialog is already on the compositor does NOT push a second dialog (App L7 checks compositor.contains(EXIT_CONFIRMATION_DIALOG_ID) before pushing)
  #
  # EXAMPLES:
  #   1. User has an idle AgentView session, presses ESC with empty input → ExitConfirmationDialog appears centred with yellow rounded border, bold 'Exit Session?' title, dim 'Choose how to exit the session.' description, Detach highlighted blue, and dim footer '← → Navigate | Enter Select | Esc Cancel'
  #   2. User has a Running session (still streaming a response), presses ESC once → L4 interrupts the stream (no dialog). User presses ESC a second time (now idle, empty input) → ExitConfirmationDialog opens with description 'Choose how to exit the session.' (status now Idle)
  #   3. User is in AgentView during Compacting state, presses ESC → because session_status_for() == Compacting, L4 takes the interrupt path; dialog does NOT open in this case
  #   4. User opens AgentView but has not started any session (current_session() is None); presses ESC → App dispatches Action::BackToBoard immediately, no dialog is ever pushed (parity with TS 'else onExit()')
  #   5. Dialog is open with Detach focused; user presses Left → focus wraps to Cancel; user presses Right → focus moves to Detach; user presses Right → Close Session; Right again → Cancel; Right again → Detach (cyclic)
  #   6. Dialog is open, Detach focused. User presses Enter → Action::AgentExitChoice { choice: Detach } emitted; dialog removed from compositor; App dispatches BackToBoard; navigator.active_view becomes Board; backend.destroy_session is NEVER called (verified via mock backend); session remains alive in backend
  #   7. Dialog is open, user presses Right twice → Close Session focused. User presses Enter → Action::AgentExitChoice { choice: CloseSession } emitted; dialog removed; App spawns backend.destroy_session(current_session_id) as a tokio task (pending_tasks gains one entry); App dispatches BackToBoard; navigator.active_view becomes Board
  #   8. Dialog is open, user presses Right three times → Cancel focused. User presses Enter → Action::AgentExitChoice { choice: Cancel } emitted; dialog removed from compositor; navigator.active_view remains Agent; no BackToBoard dispatched; no backend call made
  #   9. Dialog is open with any selection. User presses ESC → equivalent to Cancel: Action::AgentExitChoice { choice: Cancel } emitted, dialog removed, AgentView remains active, no backend call
  #   10. User presses ESC at L7 to open the dialog. Before pressing anything else, user presses ESC again. Compositor still contains exactly ONE ExitConfirmationDialog (no duplicate push). Then the inner-ESC dismisses it (Cancel).
  #   11. insta snapshot on an 80x24 TestBackend for is_busy=true: yellow rounded border, bold 'Exit Session?' title row, dim 'The agent is currently running. Choose how to exit.' description row, ' Detach ' selected (blue bg), unselected ' Close Session ' and ' Cancel ' (gray fg), centered button row, dim footer '← → Navigate | Enter Select | Esc Cancel'
  #   12. insta snapshot on 80x24 for is_busy=false: same chrome, but description row reads 'Choose how to exit the session.' instead
  #   13. App-level integration test: drive ESC through full Esc-cascade with idle session — assert App.render buffer contains a yellow rounded border centred in 80x24 view AND the title 'Exit Session?' is visible, AND the previous AgentView chrome (input row, footer) is still painted underneath (modal overlay, not view switch)
  #
  # ========================================

  Background: User Story
    As a user in the Rust AgentView
    I want to see a three-button confirmation dialog (Detach / Close Session / Cancel) when I press ESC and the Esc-cascade reaches the final fall-through level
    So that I can choose to leave my session running in the background, terminate it, or stay — instead of being kicked back to the Board with no chance to decide

  Scenario: Idle session ESC opens dialog with idle description and Detach focused
    Given I am in the Rust AgentView with an active session whose status is Idle
    And the input buffer is empty
    And no popup or mode view is currently active
    When I press ESC once
    Then an ExitConfirmationDialog is pushed onto the compositor
    And the dialog renders a yellow rounded border centred on screen
    And the title row reads "Exit Session?" in bold
    And the description row reads "Choose how to exit the session." in dim text
    And the button "Detach" is selected with blue background and white foreground
    And the buttons "Close Session" and "Cancel" are rendered in gray
    And the footer reads "← → Navigate | Enter Select | Esc Cancel" in dim text

  Scenario: Running session ESC first interrupts then second ESC opens dialog
    Given I am in the Rust AgentView with an active session whose status is Running
    And the input buffer is empty
    When I press ESC once
    Then the App spawns a backend.interrupt task for the session
    And no ExitConfirmationDialog is pushed onto the compositor
    And the navigator remains on the Agent view
    When the session status transitions to Idle
    And I press ESC a second time
    Then an ExitConfirmationDialog is pushed onto the compositor
    And the description row reads "Choose how to exit the session."

  Scenario: Compacting session ESC routes to interrupt and not to dialog
    Given I am in the Rust AgentView with an active session whose status is Compacting
    And the input buffer is empty
    When I press ESC once
    Then the App spawns a backend.interrupt task for the session
    And no ExitConfirmationDialog is pushed onto the compositor

  Scenario: No active session ESC dispatches BackToBoard without dialog
    Given I am in the Rust AgentView with no active session
    And the input buffer is empty
    And no popup or mode view is currently active
    When I press ESC once
    Then Action::BackToBoard is dispatched
    And no ExitConfirmationDialog is pushed onto the compositor
    And the navigator switches to the Board view

  Scenario: Cyclic Left/Right navigation across the three buttons
    Given the ExitConfirmationDialog is open with Detach focused
    When I press Left
    Then Cancel is focused
    When I press Right
    Then Detach is focused
    When I press Right
    Then Close Session is focused
    When I press Right
    Then Cancel is focused
    When I press Right
    Then Detach is focused

  Scenario: Enter on Detach dispatches BackToBoard without destroying the session
    Given the ExitConfirmationDialog is open with Detach focused
    And the backend records every destroy_session call
    When I press Enter
    Then Action::AgentExitChoice { choice: Detach } is emitted
    And the ExitConfirmationDialog is removed from the compositor
    And Action::BackToBoard is dispatched
    And the navigator switches to the Board view
    And the backend records zero destroy_session calls
    And the backend session remains alive

  Scenario: Enter on Close Session destroys the session then dispatches BackToBoard
    Given the ExitConfirmationDialog is open with Detach focused
    And the current AgentView session is attached to work unit "AUTH-001" in BoardStore
    And the backend records every destroy_session call
    When I press Right once
    Then Close Session is focused
    When I press Enter
    Then Action::AgentExitChoice { choice: CloseSession } is emitted
    And the ExitConfirmationDialog is removed from the compositor
    And the App spawns a backend.destroy_session task for the current session
    And the BoardStore work-unit-to-session attachment for "AUTH-001" is cleared
    And Action::BackToBoard is dispatched
    And the navigator switches to the Board view
    And the destroyed session is removed from AgentViewStore open_sessions

  Scenario: Shift+Right on the same work unit after Close Session does not navigate back to the destroyed session
    Given I am in the Rust AgentView with an active session "s1" attached to work unit "AUTH-001" in BoardStore
    And the ExitConfirmationDialog is open with Close Session focused
    When I press Enter
    Then Action::AgentExitChoice { choice: CloseSession } is emitted
    And the App spawns a backend.destroy_session task for session "s1"
    And the BoardStore work-unit-to-session attachment for "AUTH-001" is cleared
    And Action::BackToBoard is dispatched
    And the navigator switches to the Board view
    When the user presses Shift+Right while "AUTH-001" is the focused work unit on the Board
    Then BoardView::selected_session returns None
    And Action::OpenAgentView(None) is emitted
    And the destroyed SessionId "s1" is NOT routed to AgentView

  Scenario: Enter on Cancel removes the dialog and stays on AgentView
    Given the ExitConfirmationDialog is open with Detach focused
    When I press Right twice
    Then Cancel is focused
    When I press Enter
    Then Action::AgentExitChoice { choice: Cancel } is emitted
    And the ExitConfirmationDialog is removed from the compositor
    And the navigator remains on the Agent view
    And no Action::BackToBoard is dispatched
    And no backend.destroy_session task is spawned

  Scenario: ESC inside the dialog is equivalent to Cancel
    Given the ExitConfirmationDialog is open with Close Session focused
    When I press ESC
    Then Action::AgentExitChoice { choice: Cancel } is emitted
    And the ExitConfirmationDialog is removed from the compositor
    And the navigator remains on the Agent view
    And no backend.destroy_session task is spawned

  Scenario: Pressing ESC twice from L7 only opens one dialog
    Given I am in the Rust AgentView with an active session whose status is Idle
    And the input buffer is empty
    And no popup or mode view is currently active
    When I press ESC
    Then exactly one ExitConfirmationDialog is on the compositor
    When I press ESC again before navigating the dialog
    Then exactly one ExitConfirmationDialog is on the compositor
    And the ExitConfirmationDialog is removed from the compositor

  Scenario: Snapshot of the dialog rendered on 80x24 with is_busy=true
    Given an ExitConfirmationDialog instance is constructed with is_busy=true
    When the dialog is rendered into an 80x24 TestBackend buffer
    Then the snapshot shows a yellow rounded border centred on the buffer
    And the title row reads "Exit Session?" in bold
    And the description row reads "The agent is currently running. Choose how to exit." in dim text
    And the button " Detach " is styled with blue background and white foreground
    And the buttons " Close Session " and " Cancel " are styled in gray
    And the footer reads "← → Navigate | Enter Select | Esc Cancel" in dim text

  Scenario: Snapshot of the dialog rendered on 80x24 with is_busy=false
    Given an ExitConfirmationDialog instance is constructed with is_busy=false
    When the dialog is rendered into an 80x24 TestBackend buffer
    Then the snapshot shows a yellow rounded border centred on the buffer
    And the title row reads "Exit Session?" in bold
    And the description row reads "Choose how to exit the session." in dim text
    And the button " Detach " is styled with blue background and white foreground
    And the footer reads "← → Navigate | Enter Select | Esc Cancel" in dim text

  Scenario: End-to-end App render overlays the dialog on top of the AgentView chrome
    Given I am in the Rust AgentView with an active idle session
    And the input buffer is empty
    When I press ESC once
    And the App renders one frame into an 80x24 TestBackend
    Then the rendered buffer contains a yellow rounded border centred on screen
    And the rendered buffer contains the title "Exit Session?"
    And the previous AgentView chrome (header, input row, footer) is still painted underneath the modal

  Scenario: Cycling sessions in AgentView after Close Session does not list the destroyed session
    Given I am in the Rust AgentView with two open sessions "s-1" and "s-2" where "s-1" is focused and attached to work unit "AUTH-001" in BoardStore
    And the ExitConfirmationDialog is open with Close Session focused
    When I press Enter
    Then Action::AgentExitChoice { choice: CloseSession } is emitted
    And the App spawns a backend.destroy_session task for session "s-1"
    And the AgentViewStore open_sessions list contains only "s-2"
    And first_open_session_id returns "s-2"
    And navigate_next from the focused session resolves to NavTarget::CreateDialog
    And navigate_prev from the focused session resolves to NavTarget::Board
    And the destroyed SessionId "s-1" never appears in open_sessions

