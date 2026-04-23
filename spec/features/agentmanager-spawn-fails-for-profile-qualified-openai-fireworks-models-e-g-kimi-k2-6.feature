@done
@codelet
@agent-manager
@providers
@BUG-137
Feature: AgentManager spawn fails for profile-qualified OpenAI Fireworks models (e.g. kimi k2.6)
  """
  The ProviderManager stores the profile name in a new field (e.g. selected_profile_name: Option<String>) alongside selected_registry_provider_id and selected_model. This avoids re-parsing the composite and supports model ids containing slashes.
  create_session_with_id already has the profile-model branch (checks for ':' before '/'). Fixing selected_model_string() to emit the profile-qualified form is sufficient — the subordinate path will then route through set_model_direct correctly.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ProviderManager must preserve the profile name so selected_model_string() can emit the full 'provider:profile/model' composite
  #   2. session_set_model_profile NAPI accepts an optional profile_name that is plumbed through to set_model_direct
  #   3. modelSelectionService passes selection.profileName to sessionSetModelProfile when the selection is a profile-based model
  #   4. cloud-only model selections (no profile) continue to produce 'provider/model' with no colon
  #
  # EXAMPLES:
  #   1. User selects 'openai:fireworks/accounts/fireworks/models/kimi-k2p6' → ProviderManager.selected_model_string() returns 'openai:fireworks/accounts/fireworks/models/kimi-k2p6' → AgentManager spawn succeeds
  #   2. User selects cloud 'anthropic/claude-opus-4-6' → selected_model_string() returns 'anthropic/claude-opus-4-6' (no colon) → AgentManager spawn succeeds with registry validation
  #   3. set_model_direct(provider_id='openai', model_id='accounts/fireworks/models/kimi-k2p6', profile_name=Some('fireworks')) → selected_model_string() returns 'openai:fireworks/accounts/fireworks/models/kimi-k2p6'
  #   4. set_model_direct without profile_name (codex, custom provider) → selected_model_string() emits 'provider/model' with no colon segment
  #
  # ========================================
  Background: User Story
    As a fspec user with profile-qualified models (Fireworks via OpenAI-compatible API)
    I want to spawn AgentManager subordinate sessions on my profile model
    So that subordinate agents run on the same Fireworks endpoint as the spawner instead of failing with 'Model not found in provider openai'

  @bug
  @agent-manager
  @provider
  @profile
  Scenario: set_model_direct with profile_name emits profile-qualified composite
    Given a ProviderManager is created with model registry support
    When set_model_direct is called with provider_id "openai", model_id "accounts/fireworks/models/kimi-k2p6", and profile_name "fireworks"
    Then selected_model_string() returns "openai:fireworks/accounts/fireworks/models/kimi-k2p6"
    And selected_model_id() returns "accounts/fireworks/models/kimi-k2p6"

  @bug
  @agent-manager
  @provider
  @profile
  Scenario: set_model_direct without profile_name emits plain provider/model composite
    Given a ProviderManager is created with model registry support
    When set_model_direct is called with provider_id "codex", model_id "gpt-5-codex", and no profile_name
    Then selected_model_string() returns "codex/gpt-5-codex"
    And the composite contains no colon

  @bug
  @agent-manager
  @provider
  @cloud
  Scenario: select_model (cloud) does not inject a profile segment
    Given a ProviderManager is created with model registry support
    And the ANTHROPIC_API_KEY environment variable is set
    When select_model is called with "anthropic/claude-opus-4-5"
    Then selected_model_string() returns a composite without a colon before the first slash

  @bug
  @agent-manager
  @provider
  @profile
  @integration
  Scenario: AgentManager spawn round-trips profile-qualified Fireworks model
    Given a spawner session whose ProviderManager was configured via set_model_direct with profile_name "fireworks" on provider "openai" and model_id "accounts/fireworks/models/kimi-k2p6"
    When AgentManager captures selected_model_string() and passes it to create_session_with_id
    Then the subordinate path detects profile format by finding ':' before '/'
    And set_model_direct is used for the subordinate instead of select_model
    And no "Model '...' not found in provider 'openai'" error is raised
