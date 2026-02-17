@session-management
@tui
@BRIDGE-012
Feature: TUI session chunk filtering

  """
  TUI (AgentView) registers a handler with GlobalSessionStreamManager for its current session.
  It only displays chunks that match the currently-viewed session_id.
  When viewing Session A, chunks from Session B are ignored.
  Bridge input works because bridge chunks have the correct session_id.
  """

  Background: User Story
    As a developer
    I want the TUI to only display chunks for the session I'm viewing
    So that multiple sessions don't mix their output

  Scenario: TUI displays only current session chunks
    Given the TUI is viewing session "session-a"
    And session "session-a" is running
    And session "session-b" is running in background
    When session "session-a" emits a TextDelta chunk
    And session "session-b" emits a TextDelta chunk
    Then the TUI should display the chunk from session "session-a"
    And the TUI should not display the chunk from session "session-b"

  Scenario: Bridge input displays in TUI with correct session
    Given the TUI is viewing session "session-main"
    And a bridge is connected to session "session-main"
    When the bridge sends input to session "session-main"
    Then the TUI should display the bridge input
    And the TUI should display the LLM response chunks
    And all displayed chunks should have session_id "session-main"

  Scenario: Session switch changes which chunks are displayed
    Given the TUI is viewing session "session-a"
    And session "session-b" is running in background
    When the user switches to view session "session-b"
    And session "session-a" emits a chunk
    And session "session-b" emits a chunk
    Then only the chunk from session "session-b" should be displayed
