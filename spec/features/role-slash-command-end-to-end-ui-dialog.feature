@done
@slash-command
@agent-view
@dialog
@tui-component
@rust
@RPC-063
Feature: /role slash command end-to-end (UI dialog)
  """
  Wires the `/role` slash command to the RoleDialog. Both the palette
  pick AND the bare `/role` submit-line open the dialog (seeded from
  the AgentViewStore). Submit-line direct-set (`/role <text>`) and
  explicit clear (`/role clear`) keep their behaviour without opening
  the dialog. Per architecture note [H] this CHANGES the default of
  bare `/role` from clear→open-dialog; the previous tests in
  app_dispatch_rpc022.rs and slash_command_wiring_rpc022.rs that
  asserted the old clear semantics are updated in this card.

  [D] Slash parser changes (rust/fspec-tui/src/app/slash_parser.rs):
  bare `/role` → SlashCommandParse::OpenRoleDialog. `/role <text>` →
  SetRole(text). `/role clear` (case-insensitive) → ClearRole. `/role `
  (trailing-space empty arg) → OpenRoleDialog.

  [E] Dispatch wiring (rust/fspec-tui/src/app/dispatch_role_dialog.rs):
  SlashCommandAction::Role arm AND SlashCommandParse::OpenRoleDialog
  arm both route to handle_open_role_dialog(), which reads the current
  role from AgentViewStore::role_for(current_session) and pushes a
  fresh RoleDialog at Priority::Foreground.

  Component-level scenarios (priority, render, handle_event paths,
  footer text, take_pending_action, source-shape) live in
  spec/features/role-dialog-component.feature and the matching test
  file is rust/fspec-tui/tests/role_dialog_rpc063.rs.
  """

  Background: User Story
    As a fspec TUI user
    I want to open a dialog when I run /role with no argument
    So that I can view and edit the current session role interactively rather than having to remember the exact text

  @parse
  Scenario Outline: parse_slash_command routes /role variants to the new RoleDialog parse outcome
    Given the function parse_slash_command from app/slash_parser.rs
    When it is called with text=<input>
    Then it returns <expected_variant>

    Examples:
      | input                             | expected_variant                       |
      | /role                             | OpenRoleDialog                         |
      | /role clear                       | ClearRole                              |
      | /role CLEAR                       | ClearRole                              |
      | /role You are a security reviewer | SetRole("You are a security reviewer") |
      | /role  leading space ok           | SetRole("leading space ok")            |

  @dispatch
  @palette
  Scenario: Palette pick of /role on a session with no role opens the dialog with empty draft
    Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is None
    When the user picks /role from the slash palette
    Then a RoleDialog with id "role-dialog" is pushed onto the Compositor at Priority::Foreground
    And the dialog's draft buffer is the empty string
    And no tokio task is spawned that calls backend.set_session_role

  @dispatch
  @palette
  Scenario: Palette pick of /role on a session with an existing role pre-fills the dialog draft
    Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is Some("You are a security reviewer")
    When the user picks /role from the slash palette
    Then a RoleDialog with id "role-dialog" is pushed onto the Compositor at Priority::Foreground
    And the dialog's draft buffer reads "You are a security reviewer"
    And no tokio task is spawned that calls backend.set_session_role

  @dispatch
  @palette
  Scenario: Palette pick of /role with no active session is a silent no-op
    Given an App with NO open session
    When the user picks /role from the slash palette
    Then no RoleDialog is pushed onto the Compositor
    And no tokio task is spawned that calls backend.set_session_role
    And no scrollback line is appended

  @dispatch
  @submit-line
  Scenario: Submitting bare "/role" opens the RoleDialog (no longer clears the role)
    Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is Some("Reviewer A")
    When the input is submitted with text "/role"
    Then a RoleDialog with id "role-dialog" is pushed onto the Compositor at Priority::Foreground
    And the dialog's draft buffer reads "Reviewer A"
    And AgentViewStore.role_for(SessionId("s-1")) remains Some("Reviewer A")
    And no tokio task is spawned that calls backend.set_session_role

  @dispatch
  @submit-line
  Scenario: Submitting "/role You are a code reviewer" sets the role directly without opening the dialog
    Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is None
    When the input is submitted with text "/role You are a code reviewer"
    Then NO RoleDialog is pushed onto the Compositor
    And AgentViewStore.role_for(SessionId("s-1")) becomes Some("You are a code reviewer")
    And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), Some("You are a code reviewer"))

  @dispatch
  @submit-line
  @clear
  Scenario: Submitting "/role clear" clears the role directly without opening the dialog
    Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is Some("Reviewer A")
    When the input is submitted with text "/role clear"
    Then NO RoleDialog is pushed onto the Compositor
    And AgentViewStore.role_for(SessionId("s-1")) becomes None
    And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), None)

  @dispatch
  @idempotent
  Scenario: Opening the RoleDialog is idempotent when the dialog is already on the Compositor
    Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is Some("Reviewer A")
    And the user has already picked /role once so a RoleDialog is mounted on the Compositor
    When the user picks /role again from the slash palette
    Then exactly one RoleDialog with id "role-dialog" is on the Compositor (no duplicate push)
