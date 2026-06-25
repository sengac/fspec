@done
@model-selection
@providers
@PROV-123
Feature: Selecting a model in an active session does not update the global default, so a new session created in the same process inherits the stale startup model

  """
  Fix in handle_impl.rs::set_model (~1050-1061): after session.set_model/set_model_limits succeed, call SessionManager::set_default_model(self, &model) and REMOVE the now-redundant standalone save_persisted_model_string(&model) (set_default_model already writes RwLock + default-model.json + tui.lastUsedModel).
  VERIFY the model string format: set_model currently builds model=format!("{provider_id}/{model_id}"). For profile selections, confirm the string stored in the global default is the SAME form a new session can resolve (e.g. profile-qualified openai:<profile>/<model>). If set_model does not carry the profile, the worker must reconcile so new-session resolution matches the active-session selection; add a test that proves the round-trip.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Selecting a model in an ACTIVE session updates the global SessionManager default_model (get_default_model returns the newly selected model after a successful switch)
  #   2. A new session created in the SAME running process after an active-session switch inherits the just-selected model (no process restart required)
  #   3. Updating the global default on an active-session switch goes through set_default_model, so default-model.json and tui.lastUsedModel are also persisted (single source of truth; the standalone PROV-122 persist call is superseded/removed)
  #   4. PROV-101 invariant preserved: an empty/whitespace model is never written to the global default or disk, and no hardcoded fallback model is introduced
  #   5. Creating an ISOLATED session does NOT change the global default_model (isolated sessions remain ephemeral; unchanged behavior)
  #   6. A profile-qualified model selected in an active session updates the global default such that a new session resolves the same profile-qualified model
  #
  # EXAMPLES:
  #   1. default_model starts as anthropic/claude-sonnet-4; user selects anthropic/claude-opus-4-8 in the active session; get_default_model() then returns anthropic/claude-opus-4-8
  #   2. Session A is on opus-4-8; user creates a new session B in the same process; B is created using opus-4-8 (the model selected in A), not the startup model sonnet-4
  #   3. After selecting opus-4-8 in the active session, default-model.json and fspec-config.json tui.lastUsedModel both equal anthropic/claude-opus-4-8
  #   4. Creating an isolated session while the active session is on opus-4-8 leaves get_default_model() unchanged (no mutation from the isolated path)
  #   5. User selects profile-qualified openai:qwen/Qwen3-80B in the active session; get_default_model() returns openai:qwen/Qwen3-80B and a new session resolves that same profile model
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to have a model I select in my active session immediately become the model that new agents/sessions inherit
    So that new agents use the model I just picked without me having to restart fspec

  Scenario: Active-session model switch updates the global default
    Given the global default model is "anthropic/claude-sonnet-4"
    And an active session is running
    When the user selects model "anthropic/claude-opus-4-8" in the active session and the switch succeeds
    Then the global default model is "anthropic/claude-opus-4-8"

  Scenario: A new session inherits the model selected in the active session
    Given an active session has switched to model "anthropic/claude-opus-4-8"
    When the user creates a new session in the same process
    Then the new session is created using "anthropic/claude-opus-4-8"
    And the new session does not use the startup model "anthropic/claude-sonnet-4"

  Scenario: The active-session switch persists the new default to disk
    Given an active session is running
    When the user selects model "anthropic/claude-opus-4-8" in the active session and the switch succeeds
    Then default-model.json records model "anthropic/claude-opus-4-8"
    And fspec-config.json tui.lastUsedModel equals "anthropic/claude-opus-4-8"

  Scenario: Creating an isolated session leaves the global default unchanged
    Given the global default model is "anthropic/claude-opus-4-8"
    When the user creates an isolated session
    Then the global default model is still "anthropic/claude-opus-4-8"

  Scenario: A profile-qualified selection updates the default and round-trips to a new session
    Given an active session is running
    When the user selects model "openai:qwen/Qwen3-80B" in the active session and the switch succeeds
    Then the global default model is "openai:qwen/Qwen3-80B"
    And a new session created in the same process resolves "openai:qwen/Qwen3-80B"

  Scenario: An empty model selection never overwrites the global default
    Given the global default model is "anthropic/claude-opus-4-8"
    When an empty model string is applied to the active-session switch path
    Then the global default model is still "anthropic/claude-opus-4-8"
    And no empty value is written to disk
