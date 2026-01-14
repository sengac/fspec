@done
@provider-abstraction
@providers
@PROV-004
Feature: Z.AI GLM Provider Integration
  """
  Provider Implementation:
  - ZAIProvider in codelet/providers/src/zai.rs uses rig's OpenAI client with custom base_url
  - Follows same pattern as GeminiProvider for OpenAI-compatible APIs
  - Supports two endpoints based on API key environment variable:
    - ZAI_API_KEY: Normal API (https://api.z.ai/api/paas/v4)
    - ZAI_PLAN_API_KEY: Coding Plan API (https://api.z.ai/api/coding/paas/v4)
  - ZAI_PLAN_API_KEY takes precedence if both are set

  Facade Pattern:
  - Reuses OpenAI-compatible tool facades where possible
  - May need ZAI-specific thinking config facade for reasoning_content handling

  Integration Points:
  - ProviderManager in codelet/providers/src/manager.rs (add ProviderType::ZAI)
  - provider-config.ts (add to SUPPORTED_PROVIDERS array)
  - credentials.ts (add ZAI_API_KEY and ZAI_PLAN_API_KEY handling)
  - AgentView.tsx (add zai provider ID mapping)
  - ModelRegistry (add GLM model definitions)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Z.AI provider supports two endpoints:
  #      - Normal API: ZAI_API_KEY env var → https://api.z.ai/api/paas/v4
  #      - Coding Plan API: ZAI_PLAN_API_KEY env var → https://api.z.ai/api/coding/paas/v4
  #   2. ZAI_PLAN_API_KEY takes precedence if both environment variables are set
  #   3. Default model is glm-4.7 with support for glm-4.6, glm-4.5 and variants
  #   4. Provider must implement facade pattern consistent with Claude and Gemini providers
  #   5. Provider must support streaming responses with tool calling
  #   6. Provider must support thinking mode (reasoning) for compatible models
  #
  # EXAMPLES:
  #   1. User selects glm-4.7 with thinking mode enabled, receives reasoning_content in response
  #   2. User without ZAI_API_KEY or ZAI_PLAN_API_KEY tries to select Z.AI provider, sees helpful error message
  #   3. ZAIProvider created using rig's OpenAI client with custom base_url following GeminiProvider pattern
  #   4. User sets ZAI_PLAN_API_KEY to use coding plan endpoint with different pricing/limits
  #
  # ========================================
  Background: User Story
    As a developer using fspec with Z.AI/GLM models
    I want to configure and use Z.AI's GLM models (GLM-4.7, GLM-4.6, GLM-4.5) as my LLM provider
    So that I can leverage Z.AI's powerful GLM models for AI-assisted development within fspec's TUI and CLI

  Scenario: Select Z.AI GLM model with thinking mode
    Given the ZAI_API_KEY or ZAI_PLAN_API_KEY environment variable is set
    When the user selects zai/glm-4.7 with thinking mode enabled
    Then the agent should be configured with the Z.AI provider
    And the user opens the fspec TUI model selector
    And streaming responses should include reasoning_content when available

  Scenario: Error when both ZAI API keys are missing
    Given neither ZAI_API_KEY nor ZAI_PLAN_API_KEY environment variable is set
    When the user attempts to select the Z.AI provider
    Then the system should display an error message about missing credentials
    And the error should suggest setting ZAI_API_KEY or ZAI_PLAN_API_KEY

  Scenario: ZAI provider uses normal API endpoint
    Given the ZAI_API_KEY environment variable is set
    And ZAI_PLAN_API_KEY is not set
    When a ZAIProvider instance is created
    Then it should use the base URL https://api.z.ai/api/paas/v4
    And it should authenticate using Bearer token
    And it should default to model glm-4.7

  Scenario: ZAI provider uses coding plan API endpoint
    Given the ZAI_PLAN_API_KEY environment variable is set
    When a ZAIProvider instance is created
    Then it should use the base URL https://api.z.ai/api/coding/paas/v4
    And it should authenticate using Bearer token
    And it should default to model glm-4.7

  Scenario: ZAI_PLAN_API_KEY takes precedence over ZAI_API_KEY
    Given both ZAI_API_KEY and ZAI_PLAN_API_KEY environment variables are set
    When a ZAIProvider instance is created
    Then it should use the coding plan endpoint https://api.z.ai/api/coding/paas/v4
    And it should use the ZAI_PLAN_API_KEY for authentication

  Scenario: Streaming response with tool calls
    Given the ZAI_API_KEY or ZAI_PLAN_API_KEY environment variable is set
    When the user sends a message that requires tool use
    Then the response should stream incrementally
    And a ZAIProvider is configured with tools
    And tool calls should be properly parsed from the stream
