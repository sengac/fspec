@done
@persistence
@thinking-detection
@session
@TUI-002
@TUI-092
@BUG-422
Feature: Default thinking level store and session inheritance
  """
  The saved thinking level is stored in fspec-config.json, per model under
  tui.thinkingLevels since MODEL-012 (TUI-092, via the path-injectable
  *_with_dirs cores in codelet_sessions::default_thinking_level_persistence).
  Which level a new session adopts is MODEL-012's
  thinking-level-remembered-per-model feature (TUI-002 applied one global
  value to every session). Tests:
  rust/sessions/tests/tui002_default_thinking_level.rs.
  BUG-422: tests that move the process-global data directory reset the
  persistence stores (codelet_core::persistence::reset_stores_for_tests)
  before creating a session, so the session-inheritance scenarios pass in
  either order within one test binary.
  """

  Background: User Story
    As a developer who prefers a thinking level
    I want my default thinking level saved and applied to every new session
    So that I do not have to set it again for each session

  Scenario: Saving a model's thinking level round-trips from disk
    Given a data directory with no persisted default thinking level
    When the thinking level High is saved for anthropic/claude-opus-4-5 to that data directory
    And the saved thinking level for anthropic/claude-opus-4-5 is loaded from that data directory
    Then the loaded thinking level is High

  Scenario: A missing config file has no saved level
    Given a data directory with no persisted default thinking level
    When the saved thinking level for anthropic/claude-opus-4-5 is loaded from that data directory
    Then there is no saved thinking level, not Off

  Scenario: set_thinking_level_default for an unknown session saves nothing
    Given a session manager rooted at a data directory with no persisted default
    When set_thinking_level_default is called with High for an unknown session
    Then the call fails naming the unknown session
    And no thinking level is saved in that data directory
