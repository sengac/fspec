@providers
@PROV-006
Feature: OpenAI-Compatible Local Model Support (vLLM, Ollama)
  """
  Modify OpenAIProvider to use openai::CompletionsClient::builder().base_url() when OPENAI_BASE_URL is set. Follow ZAIProvider pattern. Model list fetching requires GET request to {base_url}/models endpoint.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. OpenAIProvider must respect OPENAI_BASE_URL environment variable for custom endpoints
  #   2. Model list must be fetched from local endpoint (GET /v1/models) NOT from models.dev
  #   3. Tool calling (function calling) must work with local models that support it
  #   4. OPENAI_API_KEY can be any non-empty value for local servers without auth (e.g., 'local')
  #   5. Context window and max output tokens must be configurable via OPENAI_CONTEXT_WINDOW and OPENAI_MAX_OUTPUT_TOKENS env vars
  #   6. Streaming must work the same as with remote OpenAI API
  #   7. New explicit NAPI function models_list_local_openai(base_url) for TUI integration
  #   8. Return simplified structure: just model IDs as strings (local servers don't expose capability info)
  #
  # EXAMPLES:
  #   1. Developer sets OPENAI_BASE_URL=http://localhost:8888 and OPENAI_MODEL=Qwen/Qwen3-80B, codelet connects to local vLLM
  #   2. TUI model selection dialog fetches model list from local server's /v1/models endpoint
  #   3. Agent uses Read, Write, Edit, Bash tools with local Qwen model - all tool calls work correctly
  #   4. NAPI function models_list_local_openai('http://localhost:8888') calls GET and returns model IDs
  #   5. Error when local server unreachable: 'Cannot connect to local server at {base_url}'
  #
  # ========================================
  Background: User Story
    As a developer
    I want to connect codelet to my local vLLM/Ollama server
    So that use open-source models without API costs and keep data private

  @local-server
  @vllm
  Scenario: Connect to local vLLM server with custom base URL
    Given I have a vLLM server running at "http://localhost:8888"
    And I set OPENAI_BASE_URL to "http://localhost:8888"
    And I set OPENAI_MODEL to "Qwen/Qwen3-80B"
    And I set OPENAI_API_KEY to "local"
    When I start a codelet session with the OpenAI provider
    Then the provider should connect to the local vLLM server
    And the provider should use the model "Qwen/Qwen3-80B"

  @local-server
  @ollama
  Scenario: Connect to local Ollama server with custom base URL
    Given I have an Ollama server running at "http://localhost:11434"
    And I set OPENAI_BASE_URL to "http://localhost:11434/v1"
    And I set OPENAI_MODEL to "llama3:70b"
    And I set OPENAI_API_KEY to "ollama"
    When I start a codelet session with the OpenAI provider
    Then the provider should connect to the local Ollama server
    And the provider should use the model "llama3:70b"

  @model-list
  @local-server
  @unit
  Scenario: Fetch model list from local server via OpenAIProvider
    Given I have a local server running at "http://localhost:8888"
    And the server's /v1/models endpoint returns models "Qwen/Qwen3-80B" and "mistral-7b"
    When I call OpenAIProvider.list_local_models with base_url "http://localhost:8888"
    Then an HTTP GET request should be made to "http://localhost:8888/v1/models"
    And the result should contain model IDs "Qwen/Qwen3-80B" and "mistral-7b"
    And no request should be made to models.dev

  @model-list
  @napi
  @integration
  @model-list
  @error-handling
  Scenario: Local model listing handles unreachable server
    Given I have no local server running at "http://localhost:9999"
    When I call OpenAIProvider.list_local_models with base_url "http://localhost:9999"
    Then the function should return an error
    And the error message should include "localhost:9999"
    And the request should timeout within 5 seconds

  @tool-calling
  @local-server
  Scenario: Tool calling works with local models that support it
    Given I am connected to a local server with a tool-capable model
    And I set OPENAI_MODEL to a model that supports function calling
    When the agent needs to use the Read tool
    Then the tool call should be formatted correctly for the local model
    And the tool result should be processed correctly
    And the agent should receive the file contents

  @tool-calling
  @local-server
  Scenario: Multiple tool calls work in sequence with local model
    Given I am connected to a local server with a tool-capable model
    When the agent performs a multi-step task requiring Read, Write, and Edit tools
    Then all tool calls should execute successfully
    And the final result should reflect all operations

  @authentication
  @local-server
  Scenario: Accept any non-empty API key for local servers
    Given I have a local server without authentication
    And I set OPENAI_BASE_URL to the local server URL
    And I set OPENAI_API_KEY to "dummy-key"
    When I start a codelet session
    Then the session should start successfully
    And no authentication error should occur

  @context-window
  @configuration
  Scenario: Configure custom context window size
    Given I set OPENAI_BASE_URL to a local server URL
    And I set OPENAI_CONTEXT_WINDOW to "32000"
    When I create an OpenAI provider
    Then the provider should report context window of 32000 tokens
    And compaction should respect the configured context window

  @max-output
  @configuration
  Scenario: Configure custom max output tokens
    Given I set OPENAI_BASE_URL to a local server URL
    And I set OPENAI_MAX_OUTPUT_TOKENS to "8192"
    When I create an OpenAI provider
    Then the provider should report max output tokens of 8192
    And generation requests should respect the configured limit

  @streaming
  @local-server
  Scenario: Streaming works with local server
    Given I am connected to a local server
    And streaming is enabled
    When I send a chat completion request
    Then the response should stream incrementally
    And each chunk should follow the OpenAI SSE format

  @defaults
  @fallback
  Scenario: Use default OpenAI endpoint when no custom base URL is set
    Given OPENAI_BASE_URL is not set
    And I have a valid OPENAI_API_KEY
    And I set OPENAI_MODEL to "gpt-4-turbo"
    When I create an OpenAI provider
    Then the provider should connect to the standard OpenAI API
    And the behavior should be unchanged from current implementation
