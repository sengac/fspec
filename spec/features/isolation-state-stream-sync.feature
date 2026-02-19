@isolation-state
@stream-chunk
@GIT-029
Feature: Isolation State Stream Sync

  """
  GlobalSessionStreamManager handles IsolationStateChange StreamChunk events
  to sync isolation state from Rust to TypeScript sessionStore.
  """

  Background: User Story
    As a developer
    I want the TUI to automatically sync isolation state when a session is created
    So that the SessionHeader can display the [ISOLATED] badge correctly

  @tui
  Scenario: IsolationStateChange StreamChunk updates sessionStore
    Given the GlobalSessionStreamManager is initialized
    And I have an active session
    When an IsolationStateChange chunk is received with isIsolated=true and worktreePath set
    Then the sessionStore should have isIsolated set to true
    And the sessionStore should have worktreePath set to the received path

  @tui
  Scenario: IsolationStateChange StreamChunk ignored for non-active sessions
    Given the GlobalSessionStreamManager is initialized
    And I have an active session "session-A"
    When an IsolationStateChange chunk is received for a different session "session-B"
    Then the sessionStore isolation state should remain unchanged
