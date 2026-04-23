@PROV-070
@done
@agent-loop
@model-selection
@integration
@rust
@providers
@PROV-067
Feature: Custom provider ProviderManager integration and create_rig_agent
  """
  ProviderType::Custom(String) is added without Copy; Clone-only derives propagate; as_str() returns &str borrowing from Custom's inner String; FromStr and map_provider_id_to_type() consult the discovered custom-provider registry before erroring; ProviderCredentials carries custom_available HashMap<String,bool>; has_credentials(Custom(n)) -> credentials.has_custom(n); provider_limits_resolver() returns ConstantResolver with no hard ceiling for Custom; detect_default_provider() excludes Custom.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ProviderType enum gains a Custom(String) variant; the Copy derive is removed and replaced with Clone-only
  #   2. ProviderType::as_str() changes signature to take &self and return &str, borrowing from Custom's inner String
  #   3. ProviderType::FromStr falls through to the custom provider registry before returning an Unknown provider error
  #   4. map_provider_id_to_type() recognises a custom provider slug and returns ProviderType::Custom(slug)
  #   5. ProviderCredentials gains custom_available: HashMap<String, bool> populated by detect() scanning ~/.fspec/providers/ and .fspec/providers/
  #   6. ProviderType::has_credentials(Custom(name)) delegates to credentials.has_custom(name)
  #   7. provider_limits_resolver() for Custom returns a ConstantResolver with user/registry overrides and no hard ceiling
  #   8. detect_default_provider() NEVER auto-selects a custom provider; they are explicit-only
  #   9. Custom providers with a facade field route through the existing facade provider's match arm at dispatch time (via facade_override)
  #   10. Selecting 'codelet model <custom>/<model>' calls set_model_direct with provider_id=custom-slug, facade_override=facade-name, and sets OPENAI_BASE_URL/OPENAI_API_KEY/OPENAI_MODEL env vars when facade is openai
  #   11. discover_custom_providers() scans ~/.fspec/providers/*.json (user-global) then .fspec/providers/*.json (project-local overrides user-global) and returns HashMap<String, CustomProviderDefinition>
  #   12. list_available_providers() includes discovered custom providers alongside built-in providers
  #   13. When facade is null/absent, a custom provider uses a generic CustomProvider::create_rig_agent that wraps RhaiCustomProvider + RhaiSystemPromptFacade + RhaiToolFacadeAdapter
  #   14. NAPI bindings list_providers / show_provider / validate_provider / test_provider / init_provider are exposed to the TypeScript TUI
  #
  # EXAMPLES:
  #   1. User runs 'codelet providers init my-llm --template openai-compatible' -> .fspec/providers/my-llm.json created from template
  #   2. User runs 'codelet providers list' and sees 'my-llm (custom) [MY_LLM_API_KEY ✓]' alongside claude/openai/gemini
  #   3. User runs 'codelet providers show my-llm' and sees facade, baseUrl, models, and apiKeyEnvVar status
  #   4. User runs 'codelet providers validate my-llm' with a malformed JSON -> error lists schema violations
  #   5. User runs 'codelet providers test my-llm' -> HTTP GET /v1/models succeeds and at least one listed model appears in the response
  #   6. User runs 'codelet model my-llm/llama-3.1-70b' with facade:openai -> set_model_direct sets current_provider=Custom(my-llm), facade_override=Some(openai), OPENAI_BASE_URL=http://localhost:8888/v1
  #   7. Agent loop sees current_provider_name=my-llm and facade_override=openai -> dispatches to openai arm, get_openai constructs with the custom base_url
  #   8. ProviderCredentials::detect() finds MY_LLM_API_KEY unset -> custom_available['my-llm']=false -> provider listed as unavailable
  #   9. Project-local .fspec/providers/my-llm.json overrides ~/.fspec/providers/my-llm.json during discovery
  #   10. A custom provider with facade=null uses CustomProvider::create_rig_agent with RhaiToolFacadeAdapter and RhaiSystemPromptFacade
  #   11. ProviderType::from_str('my-llm') returns Custom('my-llm') when the provider is registered, and errors when unknown
  #   12. detect_default_provider() with only MY_LLM_API_KEY set returns an auth error (custom providers never auto-select)
  #
  # ========================================
  Background: User Story
    As a developer
    I want to select a custom provider via 'codelet model <custom>/<model>' and have it route through ProviderManager via facade_override
    So that I can use any OpenAI-compatible LLM API without recompiling fspec

  Scenario: Initialize custom provider definition from openai-compatible template
    Given I have a project root with no .fspec/providers/ directory
    When I run 'codelet providers init my-llm --template openai-compatible'
    Then the file .fspec/providers/my-llm.json is created with name=my-llm and facade=openai
    And the file contains placeholder baseUrl and apiKeyEnvVar fields

  Scenario: List providers shows custom providers with credential status
    Given a project with .fspec/providers/my-llm.json defining name=my-llm apiKeyEnvVar=MY_LLM_API_KEY
    And the environment variable MY_LLM_API_KEY is set to a non-empty value
    When I call list_providers()
    Then the result includes an entry with name='my-llm', isCustom=true, available=true
    And the result also includes built-in providers like claude and openai

  Scenario: Show custom provider returns full definition
    Given a custom provider 'my-llm' is discovered with facade=openai, baseUrl=http://localhost:8888/v1, 2 models, apiKeyEnvVar=MY_LLM_API_KEY
    When I call show_provider('my-llm')
    Then the returned info includes name, displayName, facade, baseUrl, apiKeyEnvVar, and the 2 models

  Scenario: Validate custom provider reports schema violations
    Given a file .fspec/providers/broken.json missing required field 'facade'
    When I call validate_provider('broken')
    Then the result is an error describing the missing 'facade' field

  Scenario: Test custom provider performs connectivity check against baseUrl
    Given a custom provider 'my-llm' with baseUrl pointing to a mock HTTP server returning 200 and a /v1/models response listing 'llama-3.1-70b'
    When I call test_provider('my-llm')
    Then the result is Ok with reachable=true and at least one model matched

  Scenario: Select custom model routes through openai facade via facade_override
    Given a ProviderManager with custom provider 'my-llm' discovered (facade=openai, baseUrl=http://localhost:8888/v1)
    When I call set_model_direct('my-llm', 'llama-3.1-70b', Some(131072), Some(4096), Some('openai'))
    Then current_provider equals ProviderType::Custom("my-llm")
    And facade_override returns Some("openai")
    And OPENAI_BASE_URL environment variable equals 'http://localhost:8888/v1'

  Scenario: Agent loop dispatches custom provider via facade_override to existing match arm
    Given a ProviderManager with current_provider=Custom("my-llm") and facade_override=Some("openai")
    And OPENAI_BASE_URL has been applied from the custom provider via apply_custom_provider_env_vars
    When the agent loop resolves the dispatch string via facade_override().unwrap_or(current_provider_name())
    Then the resolved dispatch string equals 'openai'
    And the current provider type remains ProviderType::Custom("my-llm")
    And OPENAI_BASE_URL reflects the custom provider's base_url so the 'openai' match arm picks up the custom endpoint transparently

  Scenario: Custom provider is unavailable when required env var is unset
    Given a custom provider 'my-llm' with apiKeyEnvVar=MY_LLM_API_KEY is discovered
    And the environment variable MY_LLM_API_KEY is not set
    When I call ProviderCredentials::detect()
    Then credentials.has_custom("my-llm") returns false
    And ProviderType::Custom("my-llm").has_credentials(&credentials) returns false

  Scenario: Project-local custom provider definition overrides user-global
    Given a user-global definition at ~/.fspec/providers/my-llm.json with baseUrl=http://global/v1
    And a project-local definition at <project>/.fspec/providers/my-llm.json with baseUrl=http://local/v1
    When I call discover_custom_providers(Some(project_root))
    Then the returned map contains exactly one entry 'my-llm' with baseUrl=http://local/v1

  Scenario: Custom provider without facade uses generic CustomProvider create_rig_agent
    Given a custom provider 'rhai-llm' discovered with facade=null and a Rhai script defining define_tools and format_system_prompt
    When the agent loop requests an agent for 'rhai-llm' with no facade_override
    Then CustomProvider::create_rig_agent is invoked and wires RhaiToolFacadeAdapter instances from the script's define_tools output
    And the agent uses RhaiSystemPromptFacade to format the system prompt

  Scenario: FromStr resolves registered custom provider slug to ProviderType::Custom
    Given a custom provider 'my-llm' is discovered and registered
    When I call ProviderType::from_str("my-llm")
    Then the result is Ok(ProviderType::Custom("my-llm"))
    And ProviderType::from_str("nonexistent") returns a config error

  Scenario: Detect default provider never auto-selects a custom provider
    Given ProviderCredentials with all built-in providers unavailable
    And custom_available contains 'my-llm' set to true
    When I call ProviderManager::detect_default_provider(&credentials)
    Then the result is an auth error with message 'No provider credentials available'
