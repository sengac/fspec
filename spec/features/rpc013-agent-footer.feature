@done
@RPC-013
@rust
@tui
@ui
@rpc
@ui-enhancement
@agent-interaction
Feature: RPC-013 AgentView footer — placeholder hint for the RPC-013 slice

  """
  RPC-013 (slice 2 of 3) — AgentView paints its own 1-row footer with a
  placeholder hint. The rich `~/projects/fspec [⌥ codelet-integration]`
  form lands in RPC-018.

  Pair: tests live in codelet/fspec-tui/tests/view_agent_unit_rpc013.rs.
  """

  Background: User Story
    As a Rust fspec frontend developer
    I want AgentView to render a placeholder 1-row footer beneath its input box
    So that the agent screen carries a discoverable hint and the legacy generic / BoardView footer text no longer leaks into the agent view

  Scenario: AgentView renders the placeholder footer string for this slice
    Given an App with Navigator.active_view = ViewMode::Agent
    And AgentViewStore.current_session = Some(SessionId::new("s-1"))
    When the App renders against a 120x24 TestBackend
    Then the rendered buffer contains the substring "Enter=send"
    And the rendered buffer contains the substring "Ctrl+C=interrupt"
    And the rendered buffer contains the substring "ESC=back"

  Scenario: AgentView footer omits the legacy generic hint and the BoardView hint
    Given an App with Navigator.active_view = ViewMode::Agent
    And AgentViewStore.current_session = Some(SessionId::new("s-1"))
    When the App renders against a 120x24 TestBackend
    Then the rendered buffer does NOT contain the substring "? help"
    And the rendered buffer does NOT contain the substring "switch pane"
    And the rendered buffer does NOT contain the substring "Columns ◆ ↑↓ Work Units"
