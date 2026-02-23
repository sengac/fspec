@PROV-007
Feature: Provider Configuration Persistence and TUI Display

  """
  Architecture:
  - Extend ProviderConfig in src/utils/provider-config.ts: add contextWindow, maxOutputTokens, profiles (Record<string, ProfileConfig>), apiKey per profile
  - /model selector: profiles appear as separate sections (e.g., 'openai: work-vllm') alongside cloud providers. Fetch models from profile's baseUrl via modelsListLocalOpenai()
  - /provider screen: profile CRUD (create/read/update/delete). No 'activation' - that happens via /model selection
  - Model selection flow: user selects profile model → sessionService reads profile config → sets env vars → creates session
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /provider TUI view must display current configuration values for the selected provider
  #   2. TUI must allow editing provider configuration values and persist changes
  #   3. Support multiple named profiles per provider (e.g., 'work-vllm', 'home-ollama')
  #   4. Use existing system: ~/.fspec/fspec-config.json under 'providers' key
  #   5. Each profile has its own API key - profiles are fully independent configurations
  #   6. Greenfield implementation - only profiles exist, no legacy direct config
  #   7. Profiles appear as separate provider sections in /model selector
  #   8. Selecting a model from a profile section auto-activates that profile
  #   9. /provider is for profile management (create/edit/delete), not activation
  #
  # ========================================

  Background: User Story
    As a developer using local LLM servers
    I want to configure and view provider settings in the TUI
    So that I don't have to manage environment variables manually and can see my current configuration

  # ============================================
  # /MODEL SELECTOR - PROFILE AS PROVIDER SECTION
  # ============================================

  @model-selector
  @profiles
  Scenario: Profiles appear as separate sections in model selector
    Given I have a profile "work-vllm" configured for "openai" provider:
      | setting         | value              |
      | baseUrl         | http://work:8888   |
      | apiKey          | local-key          |
      | contextWindow   | 32768              |
      | maxOutputTokens | 8192               |
    And I have a profile "home-ollama" configured for "openai" provider:
      | setting         | value                  |
      | baseUrl         | http://localhost:11434 |
      | apiKey          | local-key              |
    When I run the "/model" command
    Then I should see a section "openai: work-vllm"
    And I should see a section "openai: home-ollama"
    And these sections should appear alongside cloud providers like "anthropic"

  @model-selector
  @local-models
  Scenario: Profile section fetches models from local server
    Given I have a profile "work-vllm" configured for "openai" provider:
      | setting | value            |
      | baseUrl | http://work:8888 |
      | apiKey  | local-key        |
    And the local server at "http://work:8888" has models:
      | model_id      |
      | Qwen/Qwen3-80B |
      | mistral-7b    |
    When I run the "/model" command
    And I expand the "openai: work-vllm" section
    Then I should see "Qwen/Qwen3-80B" in the model list
    And I should see "mistral-7b" in the model list
    And these models should be fetched via modelsListLocalOpenai("http://work:8888")

  @model-selector
  @session-creation
  Scenario: Selecting model from profile creates session with profile config
    Given I have a profile "work-vllm" configured for "openai" provider:
      | setting         | value              |
      | baseUrl         | http://work:8888   |
      | apiKey          | local-key          |
      | contextWindow   | 32768              |
      | maxOutputTokens | 8192               |
    When I run the "/model" command
    And I select "Qwen/Qwen3-80B" from the "openai: work-vllm" section
    Then sessionService should set environment variable "OPENAI_BASE_URL" to "http://work:8888"
    And sessionService should set environment variable "OPENAI_API_KEY" to "local-key"
    And sessionService should set environment variable "OPENAI_CONTEXT_WINDOW" to "32768"
    And sessionService should set environment variable "OPENAI_MAX_OUTPUT_TOKENS" to "8192"
    And a session should be created with model "openai/Qwen/Qwen3-80B"

  @model-selector
  @cloud-fallback
  Scenario: Cloud provider section uses models.dev when no profile
    Given I have ANTHROPIC_API_KEY configured
    And I have no profiles for "anthropic" provider
    When I run the "/model" command
    And I expand the "anthropic" section
    Then models should be fetched from models.dev
    And I should see "claude-sonnet-4" in the model list

  # ============================================
  # /PROVIDER SCREEN - PROFILE MANAGEMENT
  # ============================================

  @provider-screen
  @crud
  Scenario: View list of profiles for a provider
    Given I have profiles "work-vllm" and "home-ollama" configured for "openai" provider
    When I run the "/provider" command
    And I select the "openai" provider
    Then I should see profile "work-vllm" with its settings
    And I should see profile "home-ollama" with its settings

  @provider-screen
  @crud
  Scenario: Create a new profile
    Given I am viewing the "openai" provider in /provider screen
    When I create a new profile named "dev-server"
    And I set the profile configuration:
      | setting         | value                 |
      | baseUrl         | http://dev:8888       |
      | apiKey          | dev-api-key           |
      | contextWindow   | 16384                 |
      | maxOutputTokens | 4096                  |
    And I save the profile
    Then the config file should contain the profile under "providers.openai.profiles.dev-server"
    And the profile should appear in /model selector as "openai: dev-server"

  @provider-screen
  @crud
  Scenario: Edit an existing profile
    Given I have a profile "work-vllm" configured for "openai" provider with baseUrl "http://work:8888"
    When I run the "/provider" command
    And I select the "openai" provider
    And I edit the "work-vllm" profile
    And I change the baseUrl to "http://work:9000"
    And I save the changes
    Then the config file should have "providers.openai.profiles.work-vllm.baseUrl" set to "http://work:9000"

  @provider-screen
  @crud
  Scenario: Delete a profile
    Given I have profiles "work-vllm" and "home-ollama" configured for "openai" provider
    When I run the "/provider" command
    And I select the "openai" provider
    And I delete the "home-ollama" profile
    Then the config file should not contain "providers.openai.profiles.home-ollama"
    And the profile should no longer appear in /model selector

  # ============================================
  # PROFILE CONFIG STRUCTURE
  # ============================================

  @config
  @structure
  Scenario: Profile config structure
    Given I create a profile for "openai" provider
    Then the config file structure should be:
      | path                                           | type   | description                    |
      | providers.openai.profiles                      | object | Map of profile name to config  |
      | providers.openai.profiles.*.baseUrl            | string | API endpoint URL               |
      | providers.openai.profiles.*.apiKey             | string | API key for this profile       |
      | providers.openai.profiles.*.contextWindow      | number | Context window size (optional) |
      | providers.openai.profiles.*.maxOutputTokens    | number | Max output tokens (optional)   |

  # ============================================
  # INTEGRATION
  # ============================================

  @integration
  @rust
  Scenario: Profile settings flow through to Rust provider
    Given I have a profile "work-vllm" configured for "openai" provider:
      | setting         | value              |
      | baseUrl         | http://work:8888   |
      | apiKey          | my-local-key       |
      | contextWindow   | 32768              |
      | maxOutputTokens | 8192               |
    When I select a model from the "openai: work-vllm" section
    Then the Rust OpenAI provider should receive:
      | env_var                  | value            |
      | OPENAI_BASE_URL          | http://work:8888 |
      | OPENAI_API_KEY           | my-local-key     |
      | OPENAI_CONTEXT_WINDOW    | 32768            |
      | OPENAI_MAX_OUTPUT_TOKENS | 8192             |

  @integration
  @error-handling
  Scenario: Handle unreachable local server gracefully
    Given I have a profile "offline-server" configured for "openai" provider:
      | setting | value                     |
      | baseUrl | http://unreachable:8888   |
      | apiKey  | local-key                 |
    When I run the "/model" command
    Then the "openai: offline-server" section should show "(unreachable)"
    And I should still be able to use other providers
