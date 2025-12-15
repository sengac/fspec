@llm-provider
@provider-management
@PROV-001
Feature: Anthropic Claude Provider

  """
  Uses reqwest for HTTP calls to Anthropic API (https://api.anthropic.com/v1/messages). Implements LlmProvider trait from src/providers/mod.rs. Requires anthropic-version header (2023-06-01). Non-streaming implementation only (streaming deferred to separate story). Uses serde for JSON serialization of API requests/responses.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Provider MUST implement the LlmProvider trait (name, model, context_window, max_output_tokens, supports_caching, supports_streaming, complete)
  #   2. Provider MUST read API key from ANTHROPIC_API_KEY environment variable
  #   3. Provider MUST format messages according to Anthropic API format (system separate, user/assistant alternating)
  #   4. Provider MUST handle API errors gracefully and return meaningful error messages
  #   5. Provider MUST use claude-sonnet-4-20250514 as the default model
  #   6. Provider MUST report accurate context window (200000) and max output tokens (8192) for the model
  #
  # EXAMPLES:
  #   1. Given valid API key, when completing a simple user message, then receive a text response from Claude
  #   2. Given missing API key, when attempting to create provider, then return clear error about missing ANTHROPIC_API_KEY
  #   3. Given invalid API key, when completing a message, then return authentication error from API
  #   4. Given system message and user message, when completing, then system message sent in system field and user message in messages array
  #   5. Given provider instance, when querying name(), then return 'claude'
  #   6. Given provider instance, when querying context_window(), then return 200000
  #
  # ========================================

  Background: User Story
    As a AI agent runner
    I want to send messages to Claude and receive completions
    So that the agent can have conversations with users and execute tasks

  Scenario: Complete simple user message with valid API key
    Given a ClaudeProvider is created with valid ANTHROPIC_API_KEY
    When I send a user message 'Hello, Claude'
    Then I receive a non-empty text response


  Scenario: Reject provider creation when API key is missing
    Given the ANTHROPIC_API_KEY environment variable is not set
    When I attempt to create a ClaudeProvider
    Then I receive an error containing 'ANTHROPIC_API_KEY'


  Scenario: Handle authentication error for invalid API key
    Given a ClaudeProvider is created with an invalid API key
    When I send a user message
    Then I receive an authentication error from the API


  Scenario: Format system and user messages correctly for API
    Given a ClaudeProvider is created with valid credentials
    When I send messages with system and user roles
    Then the API request separates system content from the messages array


  Scenario: Return correct provider name
    Given a ClaudeProvider instance exists
    When I query the provider name
    Then I receive 'claude'


  Scenario: Return correct model limits
    Given a ClaudeProvider instance exists
    When I query the context window and max output tokens
    Then I receive context_window=200000 and max_output_tokens=8192

