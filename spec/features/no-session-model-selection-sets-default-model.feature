@done
@configuration
@model-selector
@tui
@PROV-118
Feature: Selecting a model with no active session does not set or persist a default model
  """
  Rust port fix. handle_model_selected (rust/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs) currently returns early when session_id is None. New plumbing: FspecBackend::set_default_model (default no-op), RPC service set_default_model, SessionManagerHandle::set_default_model delegating to SessionManager::set_default_model (sessions/src/session_manager.rs). On None session, spawn backend.set_default_model(provider/model). create_session decline (PROV-101) in sessions/src/handle_impl.rs no longer fires once default is set. PROV-101 no-fallback policy preserved: empty strings ignored, no hardcoded anthropic fallback. TS parity: modelSelectionService gates only the live-session write behind a session check.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When handle_model_selected receives session_id=None it MUST NOT return early; it must set the SessionManager default model from the selected provider/model
  #   2. The default-model write flows through a new set_default_model method on FspecBackend, the RPC service, and SessionManagerHandle, delegating to SessionManager::set_default_model
  #   3. After a no-session selection sets the default model, a subsequent create_session succeeds (the PROV-101 decline no longer fires) using the selected model
  #   4. The existing session-present path in handle_model_selected is unchanged (still calls set_session_model and refreshes chrome)
  #   5. set_default_model is a no-op when no SessionManager is present (mock/websocket backends) and ignores empty model strings; the fix does not re-introduce any hardcoded anthropic fallback (PROV-101 policy preserved)
  #
  # EXAMPLES:
  #   1. User opens /model with no session, expands anthropic, selects claude-opus-4-8 with Enter; handle_model_selected calls backend.set_default_model("anthropic/claude-opus-4-8") instead of returning early
  #   2. After the no-session selection sets the default model, calling create_session returns a non-empty SessionId for the selected model (previously it returned an empty SessionId / declined)
  #   3. A user with an ALREADY-ACTIVE session selects a different model; the live session model is updated as before and the default-model path is not taken (no regression)
  #
  # ========================================
  Background: User Story
    As a fspec TUI user with no active session
    I want to select a model in the /model view
    So that that model becomes the default so a session can be created with it

  Scenario: Selecting a model with no active session sets the default model
    Given no session exists and no default model is set
    And the model selector is open with session_id None
    When I select the model "anthropic/claude-opus-4-8" with Enter
    Then handle_model_selected does not return early
    And the backend set_default_model is called with "anthropic/claude-opus-4-8"

  Scenario: Selecting a model with an active session updates the live session unchanged
    Given an active session exists
    And the model selector is open with that session id
    When I select a different model with Enter
    Then the live session model is updated via set_session_model
    And the default-model path is not taken
