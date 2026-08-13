@done
@session
@providers
@model-selection
@RPC-348
Feature: Mid-session facade re-resolution for custom models on set_model
  """
  Fix location is rust/sessions/src/model_resolution.rs apply_model_selection. For the custom-model branch, after set_model_direct, derive the facade via codelet_providers::custom::derive_facade_for_custom(provider), call pm.set_facade_override(Some(facade)) when derived, and call apply_custom_provider_env_vars; for the non-custom branch call pm.set_facade_override(None) to clear any stale facade. Mirrors the NAPI session_set_model_profile post-set_model_direct block in session_bindings.rs lines 1955-1993.
  Offline test setup combines two fixtures: the rpc343 SessionManager setup (set_data_directory + dummy ANTHROPIC/GOOGLE keys + set_default_model) plus a DiscoveryFixture-style redirect of HOME/FSPEC_HOME/CWD so a custom provider config written under .fspec/providers is discoverable by derive_facade_for_custom and custom_provider_registered. Facade is observed via session.inner.lock().await then provider_manager().facade_override().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When set_model switches to a custom (registered custom-provider) model, the facade override is re-resolved server-side from the registered custom provider config and stored on the inner request-issuing provider manager, instead of being left as None
  #   2. Facade re-resolution lives in the shared model_resolution::apply_model_selection helper, so both creation-time resolution and the mid-session set_model path resolve the facade the same way (closing the pre-existing shared port gap, not just the set_model path)
  #   3. The facade is derived via codelet_providers::custom::derive_facade_for_custom(provider): an explicit config 'facade' field wins; otherwise it is derived from api_style (anthropic_messages -> 'claude', openai_chat -> 'openai'); a Rhai-scripted provider with no explicit facade resolves to None
  #   4. When set_model switches to a non-custom registry model (e.g. anthropic/google), the facade override is cleared to None so a stale facade from a previously-selected custom model cannot leak (select_model does not touch facade_override on its own)
  #   5. After applying a custom-model selection the facade env vars are applied (codelet_providers::custom::apply_custom_provider_env_vars) so the resolved facade works end-to-end during dispatch, mirroring the NAPI session_set_model_profile creation path
  #   6. No wire change: the set_model action/RPC/NAPI signature stays a 3-tuple (SessionId, provider_id, model_id); the facade is resolved server-side from the registered config, not carried across the boundary, so the locked 3-arg set_session_model source-shape contract test stays green
  #   7. If facade re-resolution / selection fails (unknown model, missing credentials), set_model returns Err before mutating session state and the prior model, limits and facade override are left unchanged (no partial corruption)
  #
  # EXAMPLES:
  #   1. Session created on a custom provider/model (default model is a custom slug) has its facade override resolved at creation time via the same shared helper, proving the gap is closed on the creation path too
  #   2. A session created on anthropic/claude-opus-4-5 is switched mid-session to a custom provider 'my-llm' (explicit facade 'openai') model 'llama-3.1-70b'; afterwards the inner provider manager reports facade override 'openai'
  #   3. A session is switched to a custom provider 'claude-compat' with no explicit facade and api_style 'anthropic_messages'; afterwards the inner provider manager reports facade override 'claude' (derived from api_style)
  #   4. A session is first switched to a custom model (facade 'openai' set), then switched to a registry model google/gemini-2.5-pro; afterwards the inner provider manager reports facade override None (the stale custom facade was cleared)
  #   5. A session on anthropic/claude-opus-4-5 is switched to google/gemini-2.5-pro (both registry models); the inner provider manager reports facade override None throughout (no regression for plain registry switches)
  #
  # ASSUMPTIONS:
  #   1. Reasoning surfacing on SessionModel (RPC-343 parity-review warning 2) is DEFERRED and out of scope here. It requires widening the SessionModel wire (rpc-types plus NAPI struct plus conversions) and there is no current consumer of a reasoning field. RPC-348 deliberately stays wire-compatible by resolving the facade server-side. Reasoning surfacing should be a separate wire-change follow-up if a consumer emerges.
  #
  # ========================================
  Background: User Story
    As a developer who switches mid-session to a custom (OpenAI-compatible / profile) model
    I want to have the per-selection facade override re-resolved server-side on set_model
    So that the agent loop dispatches the new custom model's request through the correct tool-schema facade instead of a stale or default facade left over from the previous model

  Scenario: Creating a session on a custom model resolves the facade at creation time
    Given a registered custom provider "my-llm" with explicit facade "openai" and model "llama-3.1-70b"
    And the session manager default model is "my-llm/llama-3.1-70b"
    When I create a new session
    Then the inner session provider manager reports facade override "openai"

  Scenario: Switching to a custom model with an explicit facade stores that facade
    Given a session created on "anthropic/claude-opus-4-5"
    And a registered custom provider "my-llm" with explicit facade "openai" and model "llama-3.1-70b"
    When I switch the session model to provider "my-llm" model "llama-3.1-70b" via set_model
    Then set_model returns Ok
    And the inner session provider manager reports facade override "openai"

  Scenario: Switching to a custom model with no explicit facade derives the facade from api_style
    Given a session created on "anthropic/claude-opus-4-5"
    And a registered custom provider "claude-compat" with no explicit facade and api_style "anthropic_messages" and model "opus-compat"
    When I switch the session model to provider "claude-compat" model "opus-compat" via set_model
    Then set_model returns Ok
    And the inner session provider manager reports facade override "claude"

  Scenario: Switching from a custom model to a registry model clears the stale facade
    Given a session that has been switched to a custom provider "my-llm" model "llama-3.1-70b" whose inner facade override is "openai"
    When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    Then set_model returns Ok
    And the inner session provider manager reports no facade override

  Scenario: Switching between two registry models leaves no facade override
    Given a session created on "anthropic/claude-opus-4-5" whose inner provider manager reports no facade override
    When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    Then set_model returns Ok
    And the inner session provider manager reports no facade override
