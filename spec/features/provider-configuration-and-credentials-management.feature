@done
@config
@tui
@napi
@security
@CONFIG-004
Feature: Provider Configuration and Credentials Management

  """
  See attached plan: spec/attachments/CONFIG-004/CONFIG-004-provider-configuration-plan.md for complete implementation details including data flow, NAPI types, and UI mockups
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT (11 Rules, 15 Examples)
  # ========================================
  #
  # BUSINESS RULES:
  #   0. Credentials must be stored in ~/.fspec/credentials/credentials.json with 600 file permissions
  #   1. Provider settings (enabled, defaultModel, baseUrl) must be stored in ~/.fspec/fspec-config.json under providers key
  #   2. Credential resolution must follow priority chain: explicit config → credentials file → environment variables → .env file
  #   3. Codelet-NAPI must expose new_with_credentials() factory method to accept programmatic credentials from TypeScript
  #   4. Model selection screen must have a Settings tab for API key management with masked display
  #   5. Environment variables must continue to work for backward compatibility
  #   6. API keys must never be logged, even partially
  #   7. Must support all 19 rig providers: OpenAI, Anthropic, Cohere, Gemini, Mistral, xAI, Together, HuggingFace, OpenRouter, Groq, Ollama, DeepSeek, Perplexity, Moonshot, Hyperbolic, Mira, Galadriel, Azure OpenAI, Voyage AI
  #   8. Each provider may have different auth methods: Bearer token, x-api-key header (Anthropic), query param (Gemini), or no auth (Ollama)
  #   9. Azure OpenAI requires additional config: endpoint URL and API version in addition to API key
  #   10. Ollama is local-only and requires base URL config but no API key
  #
  # EXAMPLES:
  #   0. User opens Settings tab in model selection, enters Anthropic API key, key is saved to ~/.fspec/credentials/credentials.json with 600 permissions
  #   1. User configures OpenRouter with custom base URL, provider settings saved to ~/.fspec/fspec-config.json under providers.openrouter
  #   2. TypeScript code calls CodeletSession.new_with_credentials(modelString, providerConfig) with explicit API key, session created without reading environment
  #   3. Model selection shows 'Anthropic (configured)' with green checkmark when credentials exist, 'OpenAI (not configured)' with warning when missing
  #   4. User has ANTHROPIC_API_KEY in environment but also has different key in ~/.fspec/credentials, credentials file takes precedence
  #   5. User clicks Test Connection button for a provider, system validates API key by making a lightweight API call, shows success or error
  #   6. User configures Ollama with custom base URL http://localhost:11434, no API key required, can use local models
  #   7. User configures Azure OpenAI with endpoint https://myresource.openai.azure.com, API version 2024-10-21, and API key
  #   8. User passes explicit credentials via new_with_credentials(), credentials file exists with different key, explicit config is used (highest priority)
  #   9. User has no credentials file, no env vars, but .env file exists with ANTHROPIC_API_KEY, .env file is used (lowest priority fallback)
  #   10. User presses Tab in model selection to switch to Settings view, sees list of all 19 providers with configuration status
  #   11. User enters API key in Settings, key displays as 'sk-ant-••••••••vX8D' (masked with visible prefix/suffix)
  #   12. User deletes credential for anthropic provider, credentials.json updated, provider shows as 'not configured'
  #   13. User configures Gemini provider, API key passed as query parameter (key=...) not Bearer header
  #   14. Provider registry contains all 19 providers: OpenAI, Anthropic, Cohere, Gemini, Mistral, xAI, Together, HuggingFace, OpenRouter, Groq, Ollama, DeepSeek, Perplexity, Moonshot, Hyperbolic, Mira, Galadriel, Azure, VoyageAI
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to configure provider API keys and settings via the TUI
    So that I can manage credentials without editing .env files or environment variables

  @credentials @storage
  Scenario: Save API key with secure file permissions
    Given the credentials directory does not exist
    When I save an API key for the "anthropic" provider
    Then the credentials file should be created at "~/.fspec/credentials/credentials.json"
    And the credentials file should have 600 permissions
    And the credentials directory should have 700 permissions
    And the API key should be stored under "providers.anthropic.apiKey"

  @config @provider-settings
  Scenario: Configure provider with custom base URL
    Given I have an existing fspec configuration
    When I configure the "openrouter" provider with base URL "https://openrouter.ai/api/v1"
    Then the provider settings should be saved to "~/.fspec/fspec-config.json"
    And the settings should include "providers.openrouter.baseUrl"
    And the settings should include "providers.openrouter.enabled" as true

  @napi @programmatic
  Scenario: Create session with programmatic credentials via NAPI
    Given I have TypeScript code that imports codelet-napi
    When I call CodeletSession.new_with_credentials with model "anthropic/claude-sonnet-4" and API key
    Then a session should be created successfully
    And the session should use the provided API key
    And the session should not read from environment variables

  @ui @status-display
  Scenario: Display provider configuration status in model selection
    Given the "anthropic" provider has credentials configured
    And the "openai" provider does not have credentials configured
    When I open the model selection screen
    Then I should see "Anthropic" marked as "configured"
    And I should see "OpenAI" marked as "not configured"
    And configured providers should show available models
    And unconfigured providers should show a warning

  @priority @credential-resolution
  Scenario: Credentials file takes precedence over environment variables
    Given I have ANTHROPIC_API_KEY set in the environment
    And I have a different API key in the credentials file for "anthropic"
    When the system resolves credentials for "anthropic"
    Then the credentials file API key should be used
    And the environment variable should be ignored

  @ui @connection-test
  Scenario: Test provider connection before saving
    Given I am in the provider settings view
    And I have entered an API key for "anthropic"
    When I click the "Test Connection" button
    Then the system should make a lightweight API call to validate the key
    And I should see a success message if the key is valid
    And I should see an error message if the key is invalid

  @ollama @local
  Scenario: Configure Ollama without API key
    Given I want to use the local Ollama provider
    When I configure "ollama" with base URL "http://localhost:11434"
    Then the provider should be enabled without requiring an API key
    And I should be able to list available Ollama models

  @azure @complex-config
  Scenario: Configure Azure OpenAI with endpoint and version
    Given I want to use Azure OpenAI
    When I configure "azure" provider with:
      | setting    | value                                    |
      | endpoint   | https://myresource.openai.azure.com      |
      | apiVersion | 2024-10-21                               |
      | apiKey     | my-azure-api-key                         |
    Then the Azure configuration should be saved
    And I should be able to create sessions using Azure models

  @backward-compat @env-vars
  Scenario: Environment variables continue to work
    Given I have OPENAI_API_KEY set in the environment
    And I do not have credentials configured in the credentials file
    When the system detects available providers
    Then "openai" should be marked as available
    And sessions should be creatable using the environment variable

  @security @no-logging
  Scenario: API keys are never logged
    Given I configure an API key for any provider
    When the system processes the credential
    Then the API key should not appear in any log output
    And the API key should not appear in error messages
    And only masked versions should be displayed in the UI

  @priority @credential-resolution
  Scenario: Explicit credentials take highest priority
    Given I have an API key in the credentials file for "anthropic"
    And I have ANTHROPIC_API_KEY set in the environment
    When I call CodeletSession.new_with_credentials with a different explicit API key
    Then the explicit API key should be used
    And neither the credentials file nor environment variable should be used

  @priority @credential-resolution
  Scenario: .env file is lowest priority fallback
    Given I do not have credentials in the credentials file
    And I do not have ANTHROPIC_API_KEY in the environment
    And I have a .env file with ANTHROPIC_API_KEY defined
    When the system resolves credentials for "anthropic"
    Then the .env file API key should be used

  @ui @settings-navigation
  Scenario: Navigate to Settings view with Tab key
    Given I am in the model selection screen
    When I press the Tab key
    Then I should see the Settings view
    And I should see a list of all 19 providers
    And each provider should show its configuration status

  @ui @masked-display
  Scenario: API keys display with masked format
    Given I have configured an API key "sk-ant-api03-abcdefghijklmnop" for "anthropic"
    When I view the Settings for "anthropic"
    Then the API key should display as "sk-ant-••••••••mnop"
    And the full key should never be visible

  @credentials @delete
  Scenario: Delete a provider credential
    Given I have credentials configured for "anthropic"
    When I delete the credential for "anthropic"
    Then the credentials file should be updated
    And "anthropic" should no longer have an apiKey entry
    And the provider should show as "not configured"

  @providers @registry
  Scenario: All 19 rig providers are registered
    When I query the provider registry
    Then I should see exactly 19 providers:
      | provider    |
      | openai      |
      | anthropic   |
      | cohere      |
      | gemini      |
      | mistral     |
      | xai         |
      | together    |
      | huggingface |
      | openrouter  |
      | groq        |
      | ollama      |
      | deepseek    |
      | perplexity  |
      | moonshot    |
      | hyperbolic  |
      | mira        |
      | galadriel   |
      | azure       |
      | voyageai    |

  @providers @auth-methods
  Scenario: Gemini uses query parameter authentication
    Given I have configured an API key for "gemini"
    When the system makes an API request to Gemini
    Then the API key should be passed as a query parameter "key"
    And the API key should not be in the Authorization header

  @providers @auth-methods
  Scenario: Anthropic uses x-api-key header authentication
    Given I have configured an API key for "anthropic"
    When the system makes an API request to Anthropic
    Then the API key should be in the "x-api-key" header
    And the API key should not be in the Authorization header
