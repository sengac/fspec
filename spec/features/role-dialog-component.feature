@done
@slash-command
@agent-view
@dialog
@tui-component
@rust
@RPC-063
Feature: RoleDialog component — Priority::Foreground modal for editing the session role
  """
  RoleDialog is the Priority::Foreground modal mounted by the `/role`
  slash command. Seeds a text editor from `AgentViewStore::role_for`
  and emits `Action::SetSessionRole(session_id, draft)` on Enter or
  `Action::SetSessionRole(session_id, None)` on Ctrl+D.

  - Renders via `dialog_theme::render_dialog` with `Accent::Cyan`.
  - Title row reads `Role`.
  - Footer reads `Enter Save │ Ctrl+D Clear │ Esc Cancel`.
  - `id() == "role-dialog"`.
  - `take_pending_action()` test accessor mirrors ThinkingLevelDialog.
  """

  Background: User Story
    As a fspec TUI user
    I want a Priority::Foreground modal dialog that edits the current session role
    So that the /role slash command routes me into an interactive editor with the existing role pre-filled

  @dialog
  @component
  Scenario: RoleDialog renders at Priority::Foreground with the canonical id and title
    Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = None
    When its priority() method is invoked
    Then the result is Priority::Foreground
    And its id() method returns "role-dialog"
    When the dialog is rendered onto an 80x24 TestBackend
    Then the rendered buffer contains the substring "Role"
    And the rendered buffer contains the footer substring "Enter Save"
    And the rendered buffer contains the footer substring "Ctrl+D Clear"
    And the rendered buffer contains the footer substring "Esc Cancel"

  @dialog
  @component
  Scenario: RoleDialog seeded with no role opens with an empty editable buffer
    Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = None
    When the dialog's current draft is inspected
    Then the draft buffer is the empty string

  @dialog
  @component
  Scenario: RoleDialog seeded with existing role pre-fills the editable buffer
    Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = Some("You are a security reviewer")
    When the dialog's current draft is inspected
    Then the draft buffer reads "You are a security reviewer"

  @dialog
  @component
  Scenario: Enter saves the draft as a non-empty role and removes the dialog from the Compositor
    Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = Some("Reviewer")
    And the dialog is mounted on a Compositor
    When the user types " B" so the draft reads "Reviewer B"
    And the user presses Enter
    Then handle_event returns EventResult::Consumed with a callback
    And the pending action is Action::SetSessionRole(SessionId("s-1"), Some("Reviewer B"))
    And after the callback runs the Compositor no longer contains "role-dialog"

  @dialog
  @component
  Scenario: Enter on an empty draft clears the role (treated like Ctrl+D)
    Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = None
    And the dialog is mounted on a Compositor
    When the user presses Enter without typing
    Then handle_event returns EventResult::Consumed with a callback
    And the pending action is Action::SetSessionRole(SessionId("s-1"), None)
    And after the callback runs the Compositor no longer contains "role-dialog"

  @dialog
  @component
  Scenario: Ctrl+D clears the role and removes the dialog from the Compositor
    Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = Some("You are a reviewer")
    And the dialog is mounted on a Compositor
    When the user presses Ctrl+D
    Then handle_event returns EventResult::Consumed with a callback
    And the pending action is Action::SetSessionRole(SessionId("s-1"), None)
    And after the callback runs the Compositor no longer contains "role-dialog"

  @dialog
  @component
  Scenario: Esc cancels the dialog without emitting an Action
    Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = Some("You are a reviewer")
    And the dialog is mounted on a Compositor
    When the user types " typo" so the draft reads "You are a reviewer typo"
    And the user presses Esc
    Then handle_event returns EventResult::Consumed with a callback
    And no pending action is emitted
    And after the callback runs the Compositor no longer contains "role-dialog"

  @source-shape
  Scenario: RoleDialog file stays under 300 lines
    Given the file rust/fspec-tui/src/components/role_dialog.rs after RPC-063 lands
    When a test counts the line-count of the file
    Then the file has fewer than 300 lines

  @source-shape
  Scenario: The dispatch helper file for RPC-063 stays under 300 lines
    Given the file rust/fspec-tui/src/app/dispatch_role_dialog.rs after RPC-063 lands
    When a test counts the line-count of the file
    Then the file has fewer than 300 lines
