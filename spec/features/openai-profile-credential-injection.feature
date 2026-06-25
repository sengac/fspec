@done
@rust
@model-selection
@providers
@PROV-121
Feature: OpenAI profile model selection ignores profile baseUrl/apiKey and demands OPENAI_API_KEY env var
  """
  Profile credential bridge mirrors TS configureProfileEnvironment (src/tui/services/profileEnvironmentService.ts): on selecting a profile model 'provider:profile/model', load providers.<provider>.profiles.<name> from ~/.fspec/fspec-config.json via load_local_server_profiles() and set OPENAI_BASE_URL=profile.baseUrl, OPENAI_API_KEY=profile.apiKey (and OPENAI_CONTEXT_WINDOW from profile.contextWindow when present) before dispatch. The fix lands in the shared resolver codelet/sessions/src/model_resolution.rs (apply_model_selection), which is the single entry point for BOTH create_session_with_id (session_manager.rs:521) and the mid-session set_session_model path (handle_impl.rs:1040). A new is_profile_model branch calls set_model_direct_with_profile (preserving the profile name) and a small dedicated helper (e.g. apply_profile_env_vars) analogous to apply_custom_provider_env_vars. Cloud OpenAI (openai/<model>, no profile) and PROV-101 invariants (no anthropic fallback, no silent ambiguous pick) are preserved. NOTE: create_isolated_session_with_id (session_manager.rs:732) bypasses apply_model_selection via with_provider_and_model and is a separate site flagged for supervisor scope decision.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a profile model (provider:profile/model) is selected, the profile's stored baseUrl and apiKey from providers.<provider>.profiles.<name> are applied to the environment the OpenAI client reads (OPENAI_BASE_URL / OPENAI_API_KEY) before dispatch
  #   2. The profile bridge runs on BOTH the mid-session set_session_model path (apply_model_selection) and the create_session path (SessionManager::create_session_with_id)
  #   3. The profile name (segment between ':' and '/') is preserved on the manager via set_model_direct_with_profile so the composite string openai:qwen/qwen round-trips
  #   4. Cloud OpenAI selections (openai/<model> with no profile) are NOT affected and still resolve via the existing cloud path; no hardcoded anthropic fallback (PROV-101 preserved)
  #
  # EXAMPLES:
  #   1. Selecting openai:qwen/qwen with OPENAI_API_KEY unset: the agent dispatches using the profile's baseUrl (http://192.168.0.50:8000) and apiKey, no auth error
  #   2. After resolving a profile model, OPENAI_BASE_URL equals the profile baseUrl and OPENAI_API_KEY equals the profile apiKey
  #   3. Selecting a cloud openai model (openai/gpt-4) still behaves as before — profile bridge does not fire (no profileName), env is not overwritten with profile values
  #   4. A profile model selected at create_session time (fresh session) also applies the profile env bridge before the first dispatch, not just mid-session switches
  #
  # ========================================
  Background: User Story
    As a fspec TUI user
    I want to select an openai-compatible profile model (e.g. openai:qwen/qwen) and have the agent use that profile's stored baseUrl and apiKey
    So that I can talk to my local/self-hosted OpenAI-compatible endpoint without setting an OPENAI_API_KEY env var

  Scenario: Resolving a profile model sets OPENAI_BASE_URL and OPENAI_API_KEY to the profile values
    Given an openai profile "qwen" is stored in fspec-config.json with baseUrl "http://192.168.0.50:8000" and apiKey "test"
    When the profile model "openai:qwen/qwen" is resolved for selection
    Then OPENAI_BASE_URL equals "http://192.168.0.50:8000"
    Then OPENAI_API_KEY equals "test"
    Then the profile name "qwen" is preserved so the composite model string "openai:qwen/qwen" round-trips

  Scenario: Selecting an openai profile model bridges the profile credentials so dispatch succeeds without OPENAI_API_KEY
    Given an openai profile "qwen" is stored in fspec-config.json with baseUrl "http://192.168.0.50:8000" and apiKey "test" and the OPENAI_API_KEY environment variable is unset
    When the profile model "openai:qwen/qwen" is resolved for selection
    Then the OpenAI client resolves credentials from the profile and no "OPENAI_API_KEY not set" authentication error occurs


  Scenario: Cloud OpenAI model selection does not trigger the profile credential bridge
    Given no openai profile is referenced by the selection and OPENAI_BASE_URL and OPENAI_API_KEY hold their pre-existing values
    When the cloud model "openai/gpt-4" is resolved for selection
    Then the profile credential bridge does not fire because there is no profile name
    Then OPENAI_BASE_URL and OPENAI_API_KEY are not overwritten with profile values
    Then no hardcoded anthropic or claude fallback is substituted


  Scenario: A profile model selected at create-session time applies the credential bridge before first dispatch
    Given an openai profile "qwen" is stored in fspec-config.json with baseUrl "http://192.168.0.50:8000" and apiKey "test" and the OPENAI_API_KEY environment variable is unset
    When a fresh session is created with the profile model "openai:qwen/qwen" via the shared model resolver
    Then OPENAI_BASE_URL equals "http://192.168.0.50:8000" and OPENAI_API_KEY equals "test" before the first dispatch
    Then the create-session path and the mid-session switch path apply the same profile bridge via the shared resolver


  Scenario: A profile model selected on the isolated session create path applies the credential bridge via the shared helper
    Given an openai profile "qwen" is stored in fspec-config.json with baseUrl "http://192.168.0.50:8000" and apiKey "test" and the OPENAI_API_KEY environment variable is unset
    When the isolated session create path applies the profile credential bridge for the profile model "openai:qwen/qwen" before constructing the provider manager
    Then OPENAI_BASE_URL equals "http://192.168.0.50:8000" and OPENAI_API_KEY equals "test" before the isolated session dispatches
    Then the isolated path and the shared resolver apply the identical profile bridge through one shared helper so they cannot drift

