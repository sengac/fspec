@done
@model-selection
@providers
@session
@RPC-343
Feature: Model selector model-change re-resolves nothing server-side (drops rich metadata)
  """
  Extract creation-time resolution (model-type detect plus select_model/set_model_direct plus limits compute) into a shared helper (model_resolution::apply_model_selection) so create_session_with_id and set_model call the same code, avoiding drift. The inner provider_manager is reached via codelet_cli Session::provider_manager_mut() (session/mod.rs:148).
  Offline-testable via a CROSS-FAMILY switch (same-provider switches are not observable because the Claude limits resolver clamps every anthropic model to ctx 200000 / out 8192): a session created on anthropic/claude-opus-4-5 (ctx 200000, out 8192) switched to google/gemini-2.5-pro (ctx 1048576, out 65536, 80% compaction) changes all three cached limit fields without network. Dummy ANTHROPIC_API_KEY + GOOGLE_GENERATIVE_AI_API_KEY env vars satisfy credential detection. Test via SessionManagerHandle create_session then set_model then get_session_model, mirroring rpc081_restore_session_messages.rs setup.
  Scope: re-resolve the registry-derivable LIMIT fields (context_window, max_output_tokens, compaction_threshold) and the inner manager's selected model, with NO wire change. The per-selection facade override (and reasoning surfacing) for custom models is NOT in scope here — it requires widening the set_model wire to carry the facade and is tracked as a follow-up (RPC-348). A busy-session guard declines the switch while a request is streaming.
  Fix lives in codelet/sessions handle_impl.rs set_model. After swapping the label strings on BackgroundSession, re-resolve on the inner session provider_manager and call session.set_model_limits with the recomputed context_window / max_output_tokens / compaction_threshold, mirroring the creation-time chain.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When the active model is changed mid-session via set_model, the session re-resolves the new model's context_window, max_output_tokens, and compaction_threshold from the provider registry instead of leaving the previous model's cached values
  #   2. The mid-session set_model path updates the inner request-issuing provider manager's selected model (so the next request issues against the new model), not just the cosmetic provider_id/model_id label strings
  #   3. Mid-session re-resolution mirrors creation-time resolution (the same model-type detection plus select_model/set_model_direct plus set_model_limits chain), via a shared helper rather than duplicated logic
  #   4. get_session_model called after a model switch returns the NEW model's context_window, max_output_tokens, and compaction_threshold
  #   5. If re-resolution fails (unknown model or missing credentials), set_model returns Err and the session's existing model and cached limits are left unchanged (no partial corruption)
  #   6. set_model on an unknown session id still returns Err containing the substring "Session not found" (existing behavior preserved)
  #   7. The action signature stays a 3-tuple (SessionId, provider_id, model_id) with no wire change; the locked source-shape contract test asserting the 3-arg set_session_model signature remains green
  #   8. In-place switch works: select_model re-detects credentials and sets current_provider internally (manager.rs:446,491), so the existing provider_manager can switch provider given credentials are present; rebuilding the manager is not required.
  #   9. set_model stays synchronous: it reuses the inner session's existing provider_manager via provider_manager_mut(); select_model/set_model_direct, context_window(), max_output_tokens() and resolve_compaction_threshold() are all sync, so no async with_model_support() rebuild is needed.
  #
  # EXAMPLES:
  #   1. Session created on anthropic/claude-opus-4-5 (output cap 32000); developer switches to anthropic/claude-haiku-3-5 (output cap 8192); get_session_model now reports max_output_tokens 8192 and a recomputed compaction_threshold, not the stale 32000
  #   2. After switching opus to haiku, the inner session's provider manager reports the haiku model id as its selected model, so the next request issues against haiku
  #   3. set_model called with a model id that does not exist in the registry returns Err and leaves the session's prior model and cached limits intact
  #   4. set_model called on a nonexistent session id returns Err containing "Session not found"
  #
  # QUESTIONS (ANSWERED):
  #   Q: @research: For a cross-provider switch (e.g. anthropic to openai) mid-session, can select_model on the inner session's existing provider_manager switch provider in place, or must the manager be rebuilt?
  #   A: In-place switch works: select_model re-detects credentials and sets current_provider internally (manager.rs:446,491), so the existing provider_manager can switch provider given credentials are present; rebuilding the manager is not required.
  #
  #   Q: @research: Can set_model stay synchronous by reusing the inner provider_manager, or must it become async to rebuild a ProviderManager via with_model_support()?
  #   A: set_model stays synchronous: it reuses the inner session's existing provider_manager via provider_manager_mut(); select_model/set_model_direct, context_window(), max_output_tokens() and resolve_compaction_threshold() are all sync, so no async with_model_support() rebuild is needed.
  #
  #   Q: Is full-fidelity profile/custom/codex reconstruction (profile_config base_url/api_style/key, per-selection facade and compaction overrides) in scope for RPC-343, or is that the deep-dive's fallback (B) follow-up?
  #   A: Out of scope for RPC-343. This card implements deep-dive Fix (A): re-resolve all registry-derivable fields (limits, selected model, facade, reasoning) server-side from provider_id + model_id with no wire change. Genuinely-lost profile_config (base_url/api_style/key) and per-selection custom overrides are the fallback (B) follow-up requiring an action-signature widening.
  #
  # ASSUMPTIONS:
  #   1. Out of scope for RPC-343. This card implements deep-dive Fix (A): re-resolve all registry-derivable fields (limits, selected model, facade, reasoning) server-side from provider_id + model_id with no wire change. Genuinely-lost profile_config (base_url/api_style/key) and per-selection custom overrides are the fallback (B) follow-up requiring an action-signature widening.
  #
  # ========================================
  Background: User Story
    As a developer who switches the model mid-session
    I want to change the active model on a running session
    So that the session immediately uses the new model's real context window, output cap, compaction threshold, and request configuration instead of stale values from the previous model

  Scenario: Switching model re-resolves the cached limits to the new model
    Given a session created on "anthropic/claude-opus-4-5" whose resolved context_window is 200000 and max_output_tokens is 8192
    When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    Then set_model returns Ok
    And get_session_model reports context_window 1048576 for the session
    And get_session_model reports max_output_tokens 65536 for the session
    And get_session_model reports a compaction_threshold recomputed for the new model rather than the stale claude-derived value

  Scenario: Switching model updates the inner provider manager's selected model
    Given a session created on "anthropic/claude-opus-4-5"
    When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    Then the inner session provider manager reports a gemini model id as its selected model

  Scenario: Switching to an unknown model fails and leaves the prior limits intact
    Given a session created on "anthropic/claude-opus-4-5" whose resolved context_window is 200000 and max_output_tokens is 8192
    When I switch the session model to provider "anthropic" model "does-not-exist-model" via set_model
    Then set_model returns Err
    And get_session_model still reports context_window 200000 and max_output_tokens 8192 for the session

  Scenario: Switching the model on an unknown session reports session not found
    Given a SessionManagerHandle with no session registered for the id "nonexistent-uuid"
    When I switch the model for that id to provider "google" model "gemini-2.5-pro" via set_model
    Then set_model returns Err containing "Session not found"

  Scenario: Switching the model while the session is busy is declined
    Given a session created on "anthropic/claude-opus-4-5" whose inner session is currently locked
    When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    Then set_model returns Err containing "busy"
