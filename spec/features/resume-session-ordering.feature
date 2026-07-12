@TUI-099
Feature: Sessions in /resume view are not ordered by most recent to oldest

  """
  Architecture:
  - Sort in SessionManager::list_sessions() (codelet/sessions/src/session_manager.rs) by updated_at_ms descending with session ID as tiebreaker
  - In-memory sessions get updated_at_ms from chrono::Utc::now() at call time (background_session.rs:1584)
  - Persisted sessions get updated_at_ms from SessionManifest.updated_at (session_manager.rs:399)
  - Sessions without updated_at_ms (None) appear at the end of the list
  """

  Scenario: Sessions ordered by most recently updated first
    Given I have multiple sessions with different update timestamps
    When I open the /resume view
    Then the sessions are displayed in descending order by updated_at_ms

  Scenario: Sessions with identical timestamps are ordered by session ID
    Given I have two sessions with the same updated_at_ms timestamp
    When I open the /resume view
    Then the sessions are ordered alphabetically by session ID as a tiebreaker

  Scenario: Sessions without a timestamp appear at the end
    Given I have sessions with and without updated_at_ms timestamps
    When I open the /resume view
    Then sessions with timestamps appear first and sessions without timestamps appear last
