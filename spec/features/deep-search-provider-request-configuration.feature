@BUG-102
Feature: DeepSearch sub-agent applies provider-specific request configuration
  """
  DeepSearch must keep its read-only tool set, but it cannot use a one-size-fits-all
  request builder. Provider-specific request shaping is required so the ephemeral
  sub-agent behaves like the parent provider expects.

  In particular:
  - Codex Responses API requires store=false, reasoning config, and no max_output_tokens
  - Gemini needs its model-aware system prompt and generationConfig defaults
  - Z.AI/GLM needs temperature/top_p defaults
  - Claude needs facade-based system prompt formatting for OAuth/API-key modes
  """

  Scenario: Codex sub-agent sets required Responses API fields
    Given a DeepSearch sub-agent is constructed for provider "codex"
    When the agent is built with the read-only DeepSearch tools
    Then the request additional_params include store false
    And the request additional_params include reasoning.encrypted_content
    And the request additional_params include default reasoning config
    And the request does not set max_output_tokens

  Scenario: Gemini sub-agent uses model-aware prompt and generation config
    Given a DeepSearch sub-agent is constructed for provider "gemini"
    When the agent is built with the read-only DeepSearch tools
    Then the preamble uses build_gemini_system_prompt
    And the request additional_params include generationConfig temperature 1.0
    And the request additional_params include generationConfig topP 0.95
    And Gemini 3 models include thinkingConfig in generationConfig

  Scenario: Z.AI sub-agent uses GLM generation defaults
    Given a DeepSearch sub-agent is constructed for provider "zai"
    When the agent is built with the read-only DeepSearch tools
    Then the request additional_params include temperature 1.0
    And the request additional_params include top_p 0.95

  Scenario: Claude sub-agent uses facade-based prompt formatting
    Given a DeepSearch sub-agent is constructed for provider "claude"
    When the agent is built with the read-only DeepSearch tools
    Then the preamble is transformed through the Claude system prompt facade
    And the request additional_params include the facade-formatted system payload
