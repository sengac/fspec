@provider-abstraction
@facade-pattern
@tools
@high
@TOOL-008
Feature: System Prompt Facade for Provider-Specific Prompt Handling

  """
  SystemPromptFacade trait with provider(), identity_prefix(), transform_preamble(), format_for_api() methods. Provider-specific implementations: ClaudeOAuthSystemPromptFacade (with identity prefix), ClaudeApiKeySystemPromptFacade (no prefix), GeminiSystemPromptFacade, OpenAISystemPromptFacade. Located in codelet/tools/src/facade/system_prompt.rs. Integrates with provider implementations via facade selection based on auth type.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each SystemPromptFacade MUST implement the trait with provider(), identity_prefix(), transform_preamble(), and format_for_api() methods
  #   2. Claude OAuth facade MUST prepend 'You are Claude Code...' identity prefix to the preamble
  #   3. Claude API key facade MUST NOT prepend any identity prefix (passes preamble through unchanged)
  #   4. Claude facades MUST format system prompts as array of content blocks with cache_control for caching
  #   5. Gemini and OpenAI facades MUST format system prompts as plain strings (no special structure)
  #   6. Provider implementations MUST use the facade to format system prompts before API calls
  #
  # EXAMPLES:
  #   1. Claude OAuth: preamble 'You are a helpful assistant' → format_for_api returns [{type:'text', text:'You are Claude Code...\nYou are a helpful assistant', cache_control:{type:'ephemeral'}}]
  #   2. Claude API key: preamble 'You are a helpful assistant' → format_for_api returns [{type:'text', text:'You are a helpful assistant', cache_control:{type:'ephemeral'}}] (no prefix)
  #   3. Gemini: preamble 'You are a helpful assistant' → format_for_api returns plain string 'You are a helpful assistant'
  #   4. OpenAI: preamble 'You are a helpful assistant' → format_for_api returns plain string 'You are a helpful assistant'
  #   5. ClaudeProvider uses ClaudeOAuthSystemPromptFacade when token starts with 'cc-' and ClaudeApiKeySystemPromptFacade otherwise
  #   6. identity_prefix() returns Some('You are Claude Code...') for Claude OAuth and None for other facades
  #
  # ========================================

  Background: User Story
    As a developer integrating LLM providers
    I want to have provider-specific system prompt formatting handled automatically
    So that each provider receives prompts in their expected format without duplicating logic across providers

  Scenario: Claude OAuth facade prepends identity prefix to preamble
    Given a ClaudeOAuthSystemPromptFacade
    And a preamble "You are a helpful assistant"
    When I call format_for_api with the preamble
    Then the result should be a JSON array with cache_control
    And the text should start with "You are Claude Code"
    And the text should contain the original preamble


  Scenario: Claude API key facade passes preamble through unchanged
    Given a ClaudeApiKeySystemPromptFacade
    And a preamble "You are a helpful assistant"
    When I call format_for_api with the preamble
    Then the result should be a JSON array with cache_control
    And the text should NOT start with "You are Claude Code"
    And the text should equal the original preamble exactly


  Scenario: Gemini facade formats preamble as plain string
    Given a GeminiSystemPromptFacade
    And a preamble "You are a helpful assistant"
    When I call format_for_api with the preamble
    Then the result should be a plain string
    And the result should equal "You are a helpful assistant"


  Scenario: OpenAI facade formats preamble as plain string
    Given an OpenAISystemPromptFacade
    And a preamble "You are a helpful assistant"
    When I call format_for_api with the preamble
    Then the result should be a plain string
    And the result should equal "You are a helpful assistant"


  Scenario: Claude facade formats system prompts with cache_control
    Given any Claude system prompt facade
    And a preamble "You are a helpful assistant"
    When I call format_for_api with the preamble
    Then the result should be a JSON array
    And each element should have a cache_control field with type "ephemeral"


  Scenario: ClaudeProvider selects correct facade based on token type
    Given a ClaudeProvider with OAuth token starting with "cc-"
    When the provider selects a system prompt facade
    Then it should use ClaudeOAuthSystemPromptFacade

  Scenario: ClaudeProvider uses API key facade for non-OAuth tokens
    Given a ClaudeProvider with API key not starting with "cc-"
    When the provider selects a system prompt facade
    Then it should use ClaudeApiKeySystemPromptFacade

