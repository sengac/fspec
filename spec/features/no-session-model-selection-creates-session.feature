@BUG-582
@done
@session-creation
@model-selection
@tui
@MODEL-006
@BUG-410
Feature: Selecting a model in /model with no active session does nothing
  """
  Re-creation funnels through run_session_creation: a created id seeds the active session via SessionCreated, and a create_session error (for example no default model, BUG-410) becomes SessionOperationFailed and seeds nothing
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Selecting a model with no active session must result in a session being created or an explicit decline dialog, never a silent no-op
  #   2. The re-creation of the session only fires after set_default_model resolves Ok
  #   3. A failed retried create_session is shown through the session failure dialog and never seeds an active session
  #
  # EXAMPLES:
  #   1. No session active, open /model, pick anthropic/claude-opus-4-8 -> default model set -> create_session retried -> Action::SessionCreated dispatched -> Agent view usable
  #   2. set_default_model succeeds but the retried create_session fails -> Action::SessionOperationFailed -> session failure dialog shown
  #   3. Selecting a model with an active session updates the live session via set_session_model and does NOT call set_default_model or create_session (no regression)
  #
  # ========================================
  Background: User Story
    As a TUI user with no active session
    I want to select a model in the /model view
    So that a usable session is created so the agent works

  Scenario: Selecting a model with no active session retries session creation
    Given no session is active and no default model is set
    And the model selector is open with session_id None
    When I select the model "anthropic/claude-opus-4-8" with Enter
    Then the backend set_default_model is called with "anthropic/claude-opus-4-8"
    And after the default is committed create_session is retried
    And an Action::SessionCreated is dispatched
    And a usable active session exists

  Scenario: Retried session creation fails
    Given no session is active and no default model is set
    And the next create_session fails because no default model is set
    And the model selector is open with session_id None
    When I select the model "anthropic/claude-opus-4-8" with Enter
    Then the backend set_default_model is called with "anthropic/claude-opus-4-8"
    And create_session is retried after the default is committed
    And the session failure dialog is shown
    And no active session is seeded

  Scenario: Selecting a model with an active session does not retry creation
    Given an active session exists
    And the model selector is open with that session id
    When I select a different model with Enter
    Then the live session model is updated via set_session_model
    And the default-model path is not taken
    And create_session is not retried
