@rpc
@bug
@parity
@ts-parity
@session-resume
@RPC-427
Feature: Filter /resume session list by current project

  """
  The project path is resolved at the TUI layer via std::env::current_dir().
  All call sites of list_sessions must be updated to pass the project path.
  Cross-transport parity: both EmbeddedFspecBackend and WebSocketFspecBackend
  must implement the new list_sessions(project_path: String) method.
  """

  Background: As an agent user
    Given I want to type /resume to see previous sessions
    Then I should only see sessions from my current project

  Scenario: Session list is filtered to current project on /resume
    Given I have sessions persisted in two different projects
    When I open the Rust TUI in project A and type /resume
    Then the session list should only contain sessions from project A
    And sessions from project B should not appear in the list

  Scenario: Session list refreshes with project filter after deleting a session
    Given I have multiple sessions in the current project
    When I delete a session from the resume list
    And the session list refreshes
    Then the refreshed list should still only contain sessions from the current project

  Scenario: Background sessions are included alongside filtered persisted sessions
    Given I have a background session running in the current project
    And I have persisted sessions in the current project
    When I type /resume
    Then both background and persisted sessions from the current project should appear

  Scenario: Cross-transport parity for both embedded and WebSocket backends
    Given the FspecBackend trait has a list_sessions method accepting project_path
    When I call list_sessions with a project path via EmbeddedFspecBackend
    Then the EmbeddedFspecBackend should pass the project path to the tarpc client
    When I call list_sessions with a project path via WebSocketFspecBackend
    Then the WebSocketFspecBackend should pass the project path to the tarpc client