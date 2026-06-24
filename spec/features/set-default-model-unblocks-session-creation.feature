@done
@tui @session @configuration @PROV-118
Feature: Set default model unblocks session creation

  """
  Architecture notes:
  - Rust port: SessionManagerHandle::set_default_model delegates to
    SessionManager::set_default_model (sessions/src/session_manager.rs), which
    feeds get_default_model used by create_session
    (sessions/src/handle_impl.rs:82-108).
  - PROV-101 preserved: empty model strings are ignored and no hardcoded
    anthropic fallback is re-introduced.
  """

  Background: User Story
    As a fspec TUI user with no active session
    I want to have my model choice set as the default model
    So that the next create_session succeeds instead of being declined by PROV-101

  Scenario: Setting the default model unblocks session creation
    Given no session exists and no default model is set
    When a no-session model selection sets the default model to "anthropic/claude-opus-4-5"
    And create_session is called
    Then create_session returns a non-empty session id
    And the created session uses the model "anthropic/claude-opus-4-5"
