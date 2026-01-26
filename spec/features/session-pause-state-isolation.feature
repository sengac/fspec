@pause-integration
@codelet
@PAUSE-001
Feature: Session Pause State Isolation

  """
  Session integration: Add SessionStatus::Paused variant to session_manager.rs.
  Add pause_state: RwLock<Option<PauseState>> field to BackgroundSession struct.
  Follows existing AtomicU8 status pattern for thread-safe state management.
  NAPI bindings: session_get_pause_state(), session_pause_resume(), session_pause_confirm()
  """

  Background: User Story
    As a user running multiple agent sessions
    I want each session's pause state to be independent
    So that pausing one session doesn't affect others

  Scenario: Pause state is stored per-session not globally
    Given two sessions exist
    When session A pauses
    Then session A status should be "paused"
    And session B status should be "running"
    And querying session A pause state returns the details
    And querying session B pause state returns null
