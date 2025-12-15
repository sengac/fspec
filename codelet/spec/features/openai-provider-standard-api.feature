@done
@llm-provider
@provider-management
@PROV-003
Feature: OpenAI Provider (Standard API)
  """
  Uses reqwest HTTP client to call api.openai.com/v1/chat/completions endpoint. Follows ClaudeProvider pattern with LlmProvider trait implementation. Supports tool calling with OpenAI-specific format conversion (function.name and function.arguments as JSON string). No streaming support in v1 (returns false for supports_streaming). No prompt caching (returns false for supports_caching). Default model: gpt-4-turbo (128K context, 4K output), configurable via OPENAI_MODEL environment variable.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. OpenAI provider requires OPENAI_API_KEY environment variable
  #   2. Default model is gpt-4-turbo (128K context, 4K output tokens)
  #   3. Provider must implement LlmProvider trait (complete, complete_with_tools, supports_streaming, supports_caching)
  #   4. OpenAI tool calls use different format than Claude: function.name and function.arguments (JSON string) instead of name and input (JSON object)
  #   5. Tool results must be injected as role: tool messages with tool_call_id field, not tool_result content blocks
  #   6. Token usage must be parsed from response.usage field (prompt_tokens, completion_tokens, total_tokens)
  #   7. Provider must fail gracefully with clear error messages for auth failures (401), rate limits (429), invalid requests (400), and model not found (404)
  #   8. OpenAI does not support prompt caching - supports_caching() must return false
  #
  # EXAMPLES:
  #   1. User sets OPENAI_API_KEY=sk-proj-abc123, calls OpenAIProvider::new(), provider initializes successfully with gpt-4-turbo model
  #   2. User does not set OPENAI_API_KEY, calls OpenAIProvider::new(), provider returns error: OPENAI_API_KEY environment variable not set
  #   3. User sends prompt Hello! via complete(), OpenAI API returns Hi! How can I help you?, provider returns response text
  #   4. User requests Read /tmp/test.txt with tools, OpenAI returns tool_call with function.name=Read and function.arguments={\"file_path\":\"/tmp/test.txt\"}, provider parses to ToolCall{name:Read, arguments:{file_path:/tmp/test.txt}}
  #   5. Tool execution returns File contents, provider injects as {role:tool, tool_call_id:call_abc123, content:File contents}, continues conversation
  #   6. OpenAI API returns usage: {prompt_tokens:100, completion_tokens:50, total_tokens:150}, provider tracks token usage correctly
  #   7. User provides invalid API key sk-invalid, OpenAI returns 401 Unauthorized, provider returns error: Invalid OPENAI_API_KEY
  #   8. User hits rate limit, OpenAI returns 429 Rate Limit Exceeded with retry-after header, provider returns error with retry guidance
  #   9. User calls supports_caching() on OpenAI provider, returns false (OpenAI does not support prompt caching)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the provider support model override via OPENAI_MODEL environment variable, or should model be hardcoded to gpt-4-turbo?
  #   A: Support OPENAI_MODEL environment variable override - this provides flexibility for users and testing (e.g., use gpt-3.5-turbo for cheaper tests). Default to gpt-4-turbo if not set. This matches the pattern in ClaudeProvider where model can be configured.
  #
  #   Q: Should the provider support gpt-3.5-turbo model in addition to gpt-4-turbo, or focus only on gpt-4-turbo for MVP?
  #   A: Support both gpt-4-turbo (default) and gpt-3.5-turbo - minimal additional complexity since model is just a string parameter. This enables cost-conscious users and cheaper integration tests. Document supported models in code comments.
  #
  #   Q: For tool calling, should we test with all existing tools (Read, Write, Edit, Bash, Grep, Glob, AstGrep) or just a subset for MVP?
  #   A: Test with Read, Write, and Bash tools as representative subset - covers file operations and command execution. These are sufficient to validate tool format conversion works correctly. Can expand to all tools in future integration tests if needed.
  #
  # ========================================
  Background: User Story
    As a developer using codelet
    I want to use OpenAI models (gpt-4-turbo, gpt-4, gpt-3.5-turbo) as my LLM provider
    So that I can choose the best model for my needs and leverage OpenAI's API when Claude is unavailable or unsuitable

  Scenario: Initialize OpenAI provider with API key from environment
    Given OPENAI_API_KEY environment variable is set to "sk-proj-abc123"
    When I call OpenAIProvider::new()
    Then the provider should initialize successfully
    And the provider should use gpt-4-turbo model by default
    And the provider name should be "openai"
    And the context window should be 128000
    And the max output tokens should be 4096

  Scenario: Initialize OpenAI provider fails without API key
    Given OPENAI_API_KEY environment variable is not set
    When I call OpenAIProvider::new()
    Then I should receive an error
    And the error message should contain "OPENAI_API_KEY environment variable not set"

  Scenario: Override model via OPENAI_MODEL environment variable
    Given OPENAI_API_KEY environment variable is set to "sk-proj-abc123"
    And OPENAI_MODEL environment variable is set to "gpt-3.5-turbo"
    When I call OpenAIProvider::new()
    Then the provider should initialize successfully
    And the provider should use "gpt-3.5-turbo" model

  Scenario: Provider reports no prompt caching support
    Given I have an initialized OpenAI provider
    When I call supports_caching()
    Then it should return false

  Scenario: Provider reports streaming support
    Given I have an initialized OpenAI provider
    When I call supports_streaming()
    Then it should return true

  Scenario: Complete simple prompt without tools
    Given I have an initialized OpenAI provider
    And I have a message with role "user" and content "Hello!"
    When I call complete() with the messages
    Then the provider should return text response
    And the response should be non-empty

  Scenario: Create rig agent with all tools configured
    Given I have an initialized OpenAI provider
    When I call create_rig_agent()
    Then a rig Agent should be created
    And the agent should have 7 tools configured
    And the agent should use the provider's model name
    And the agent should have max_tokens set to 4096
