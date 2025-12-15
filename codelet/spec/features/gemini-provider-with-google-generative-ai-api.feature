@provider-management
@llm-provider
@high
@PROV-005
Feature: Gemini Provider with Google Generative AI API

  """
  Uses rig::providers::gemini for Google Generative AI API communication. Default model: gemini-2.0-flash-exp. Supports all 7 tools (Read, Write, Edit, Bash, Grep, Glob, AstGrep) via rig agent builder pattern. Authentication via GOOGLE_GENERATIVE_AI_API_KEY environment variable.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Gemini provider requires GOOGLE_GENERATIVE_AI_API_KEY environment variable
  #   2. Provider must use rig framework with rig::providers::gemini for Gemini API communication
  #   3. Default model must be gemini-2.0-flash-exp for best performance
  #   4. Provider must support all 7 tools (Read, Write, Edit, Bash, Grep, Glob, AstGrep) via rig agent
  #   5. Provider must fail gracefully with clear error message if GOOGLE_GENERATIVE_AI_API_KEY is not set
  #
  # EXAMPLES:
  #   1. User sets GOOGLE_GENERATIVE_AI_API_KEY and GeminiProvider initializes successfully
  #   2. User runs codelet --provider gemini and receives streaming response from Gemini model
  #   3. User with Gemini active executes tool and tool call works correctly
  #   4. User without GOOGLE_GENERATIVE_AI_API_KEY sees error when trying to use Gemini
  #
  # ========================================

  Background: User Story
    As a codelet user with Google Gemini API access
    I want to use Google Gemini models as my LLM provider
    So that I can choose the most suitable model for my needs and budget

  Scenario: Provider initialization with API key
    Given the GOOGLE_GENERATIVE_AI_API_KEY environment variable is set
    When I create a new GeminiProvider instance
    Then the provider should initialize successfully
    And the provider should use gemini-2.0-flash-exp model

  Scenario: Stream text generation with Gemini
    Given the GOOGLE_GENERATIVE_AI_API_KEY environment variable is set
    And Gemini is the active provider
    When I run codelet with prompt "Hello"
    Then I should receive a streaming text response
    And the response should come from the gemini-2.0-flash-exp model

  Scenario: Tool calling with Gemini provider
    Given the GOOGLE_GENERATIVE_AI_API_KEY environment variable is set
    And Gemini is the active provider
    When I request to execute a Bash tool
    Then the tool call should be processed correctly
    And the tool result should be returned to the model

  Scenario: Error without API key
    Given the GOOGLE_GENERATIVE_AI_API_KEY environment variable is not set
    When I attempt to create a GeminiProvider instance
    Then the provider should return an error
    And the error message should mention "GOOGLE_GENERATIVE_AI_API_KEY"

