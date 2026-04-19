@done
@integration
@validator
@rust
@providers
@PROV-063
Feature: Custom provider HTTP request/response lifecycle

  """
  RhaiCustomProvider<LlmProvider> delegates to 7 Rhai functions; request_bridge and response_bridge convert between CompletionRequest/Response and Rhai Dynamic; all Rhai calls run in tokio::task::spawn_blocking; HTTP via reqwest; errors flow through map_error
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. RhaiCustomProvider implements the LlmProvider trait by delegating to the 7 required Rhai functions from PROV-062
  #   2. build_url(config, model) returns the full URL to POST against
  #   3. build_headers(config, auth_headers) returns a map of HTTP headers
  #   4. build_request(config, messages, tools, model) returns a JSON-serializable Rhai Dynamic (body to send)
  #   5. parse_response(config, response_json) returns a map with content, tool_calls (optional), and stop_reason fields
  #   6. map_error(config, status, body) returns a structured ProviderError variant (Api, RateLimit, Auth, etc.)
  #   7. The request_bridge converts CompletionRequest (messages, tools, model) into Rhai Dynamic before calling build_request
  #   8. The response_bridge converts the Rhai Dynamic returned by parse_response into CompletionResponse with MessageContent and StopReason
  #   9. All Rhai script calls run inside tokio::task::spawn_blocking because the Rhai Engine is synchronous
  #   10. HTTP requests are performed with reqwest (async) using url, headers, and body supplied by the Rhai script
  #   11. HTTP responses with status >= 400 are passed to map_error and its result is returned as ProviderError
  #   12. Rhai runtime errors (panics avoided) are caught and converted to ProviderError::Api with script error details
  #   13. StopReason mapping: end_turn/stop -> EndTurn; tool_use/tool_calls -> ToolUse; max_tokens/length -> MaxTokens
  #
  # EXAMPLES:
  #   1. build_request is called with an OpenAI-compatible format and the returned body JSON contains 'messages':[{role:'user',content:'hi'}]
  #   2. build_url returns 'https://api.example.com/v1/chat/completions' when base_url='https://api.example.com' and path='/v1/chat/completions'
  #   3. build_headers adds 'Authorization: Bearer sk-xxx' and 'Content-Type: application/json'
  #   4. parse_response on {choices:[{message:{content:'hello'}, finish_reason:'stop'}]} returns content='hello' and stop_reason=EndTurn
  #   5. parse_response extracting a single tool_call with name='read_file' and input={path:'a.txt'} yields CompletionResponse with MessageContent::ToolUse and StopReason::ToolUse
  #   6. A 401 response causes map_error to return ProviderError::Auth with message 'unauthorized'
  #   7. A 429 response maps via map_error to ProviderError::RateLimit
  #   8. A Rhai runtime error thrown inside parse_response yields ProviderError::Api whose message contains the Rhai error text
  #   9. RhaiCustomProvider::complete_with_tools returns a CompletionResponse populated from a successful HTTP round-trip against a wiremock server
  #   10. RhaiCustomProvider.name() returns the provider name from ProviderConfig
  #   11. RhaiCustomProvider.context_window() returns the value from the selected ModelDef
  #   12. Request bridge converts a Vec<Message> with two user/assistant turns into a Rhai array preserving role and content
  #   13. Response bridge converts tool_calls with structured input into ToolUseContent with id, name, and serde_json::Value input
  #
  # ========================================

  Background: User Story
    As a custom provider author
    I want to have my Rhai script build HTTP requests, parse responses, and extract tool calls for the LLM API
    So that my custom provider can complete prompts through its native HTTP protocol without recompilation

  Scenario: Build request body from messages
    Given a Rhai script whose build_request produces {messages:[{role:"user",content:"hi"}]}
    When I call the request_bridge with a single user message "hi"
    Then the resulting JSON body contains messages array with role "user" and content "hi"


  Scenario: Build request URL from config
    Given a config with base_url "https://api.example.com" and a script build_url returning "/v1/chat/completions"
    When RhaiCustomProvider resolves the target URL
    Then the URL equals "https://api.example.com/v1/chat/completions"


  Scenario: Build HTTP headers including auth
    Given a Rhai script whose build_headers returns a map with Authorization and Content-Type
    When RhaiCustomProvider assembles outgoing HTTP headers
    Then the HeaderMap contains Authorization "Bearer sk-xxx" and Content-Type "application/json"


  Scenario: Parse plain text response
    Given a Rhai script whose parse_response extracts content from choices[0].message.content and finish_reason
    When I parse the JSON {choices:[{message:{content:"hello"},finish_reason:"stop"}]}
    Then the CompletionResponse has content text "hello" and stop_reason EndTurn


  Scenario: Parse tool call response
    Given a Rhai script parsing a tool_call with name "read_file" and input {path:"a.txt"}
    When I parse the response body
    Then the CompletionResponse contains MessageContent::ToolUse with name "read_file" and stop_reason ToolUse


  Scenario: Map HTTP 401 to auth error
    Given a Rhai script whose map_error returns an auth error for status 401
    When the HTTP response returns status 401 with body "{\"error\":\"unauthorized\"}"
    Then I receive ProviderError::Auth whose message contains "unauthorized"


  Scenario: Map HTTP 429 to rate limit error
    Given a Rhai script whose map_error returns rate_limit for status 429
    When the HTTP response returns status 429
    Then I receive ProviderError::RateLimit


  Scenario: Surface Rhai runtime errors as provider errors
    Given a Rhai script whose parse_response throws a runtime error
    When I complete with that provider
    Then I receive ProviderError::Api and the process does not crash


  Scenario: Complete end-to-end request against mock server
    Given a wiremock server responding to /v1/chat/completions with a valid OpenAI-style success payload
    When RhaiCustomProvider.complete_with_tools is called with a single user message
    Then the returned CompletionResponse contains the mock server's content text


  Scenario: Provider name reflects config
    Given a ProviderConfig with name "my-llm"
    When I construct a RhaiCustomProvider from that config
    Then provider.name() returns "my-llm"


  Scenario: Provider context window reflects selected model
    Given a config with a model "big" defining context_window 200000 and max_output_tokens 8192
    When I construct a RhaiCustomProvider selecting model "big"
    Then provider.context_window() equals 200000 and provider.max_output_tokens() equals 8192


  Scenario: Request bridge preserves multi-turn message structure
    Given a conversation with user then assistant then user turns
    When I convert the messages through the request_bridge
    Then the resulting Rhai array has three entries in the correct order with matching roles and contents


  Scenario: Response bridge preserves structured tool call input
    Given a Rhai response map with tool_call input {path:"a.txt", mode:"read"}
    When I convert it through the response_bridge
    Then the ToolUseContent input is a serde_json::Value object with fields path="a.txt" and mode="read"

