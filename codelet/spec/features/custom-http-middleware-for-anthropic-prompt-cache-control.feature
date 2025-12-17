@wip
@providers
@provider-management
@PROV-006
Feature: Custom HTTP Middleware for Anthropic Prompt Cache Control

  """
  IMPLEMENTATION STATUS:

  IMPLEMENTED (via rig additional_params):
  - System prompt transformation to array format with cache_control metadata
  - OAuth mode: 2 blocks (prefix without cache_control, content with cache_control)
  - API key mode: 1 block with cache_control (REQUIRES preamble parameter)
  - User message cache_control transformation functions (exist but require HTTP middleware)
  - TokenTracker.effective_tokens() with 90% cache discount calculation
  - CacheTokenExtractor module for parsing SSE message_start events

  FIXED (API key mode cache_control):
  - ClaudeProvider.create_rig_agent(preamble: Option<&str>) now accepts optional preamble
  - When preamble is provided, cache_control is applied for BOTH auth modes
  - OAuth mode: Claude Code prefix + preamble, cache_control on preamble block
  - API key mode: preamble with cache_control
  - All providers updated with consistent API signature

  PARTIAL (user message cache_control):
  - transform_user_message_cache_control() function exists and is tested
  - NOT wired into production because it requires HTTP middleware to intercept requests
  - Current architecture stores CLAUDE.md as user messages, not system prompt
  - Would require either HTTP middleware or architectural change to wire in

  BLOCKED (rig 0.25+ streaming abstraction):
  - Cache token extraction from live streaming responses
  - Rig's streaming layer (rig-core/src/providers/anthropic/streaming.rs line 230)
    only extracts input_tokens from message_start events, discarding cache_read_input_tokens
    and cache_creation_input_tokens before returning PartialUsage
  - Without access to raw SSE before rig processes it, cache tokens cannot be extracted
  - Would require either patching rig or implementing custom streaming

  APPROACH:
  - Uses ClaudeProvider.create_rig_agent(preamble) with additional_params to inject cache_control
  - System prompt is set as array format with cache_control via additional_params override
  - Request-side caching works for system prompt when preamble is provided
  - Response-side cache token reporting blocked by rig's streaming abstraction
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System prompts must be sent as array of content blocks with cache_control metadata, not plain strings
  #   2. OAuth mode: first block is Claude Code prefix WITHOUT cache_control, second block has cache_control
  #   3. API key mode: single block with cache_control
  #   4. Middleware must only intercept requests to /v1/messages endpoint
  #   5. Cache tokens (cache_read_input_tokens, cache_creation_input_tokens) must be extracted from message_start SSE event
  #   6. effective_tokens() must apply 90% discount to cache_read_input_tokens
  #   7. Non-Anthropic providers must be unaffected by middleware
  #   8. CachingHttpClient MUST wrap reqwest::Client and be used by ClaudeProvider for all API requests
  #   9. ClaudeProvider.client() MUST return a client that automatically transforms /v1/messages requests
  #   10. CacheTokenExtractor MUST be wired into the streaming response handler in interactive.rs
  #   11. After streaming completes, cache tokens from SSE MUST be accumulated into session.token_tracker
  #
  # EXAMPLES:
  #   1. API key mode: system='You are helpful' → system=[{type:text, text:'You are helpful', cache_control:{type:ephemeral}}]
  #   2. OAuth mode: system='Claude Code prefix...Additional text' → system=[{type:text,text:'Claude Code prefix'},{type:text,text:'Additional text',cache_control:{type:ephemeral}}]
  #   3. Request to /v1/models is NOT transformed (passthrough)
  #   4. SSE event 'message_start' with usage.cache_read_input_tokens=5000 → turn_cache_read_tokens=5000
  #   5. SSE event 'message_start' with usage.cache_creation_input_tokens=2000 → turn_cache_creation_tokens=2000
  #   6. TokenTracker with input_tokens=10000, cache_read_input_tokens=5000 → effective_tokens()=5500 (10000 - 4500 discount)
  #   7. OpenAI provider request is passed through without transformation
  #   8. First user message content gets cache_control added for context caching
  #   9. INTEGRATION: ClaudeProvider sends request → CachingHttpClient intercepts → body is transformed with cache_control before reaching Anthropic API
  #   10. INTEGRATION: Streaming response arrives → CacheTokenExtractor parses message_start SSE → cache_read_input_tokens=5000 flows to session.token_tracker.cache_read_input_tokens
  #   11. INTEGRATION: After turn completes with cache_read_input_tokens=5000, session.token_tracker.effective_tokens() returns discounted value
  #   12. INTEGRATION: ClaudeProvider created with OAuth mode → CachingHttpClient.is_oauth=true → system prompt split into 2 blocks
  #
  # ========================================

  Background: User Story
    As a developer using codelet with Claude provider
    I want to have prompt caching work automatically when using the Anthropic API
    So that I get reduced latency and lower API costs for repeated context

  # ========================================
  # UNIT-LEVEL SCENARIOS (Transformation Functions)
  # ========================================

  Scenario: Transform system prompt to array format in API key mode
    Given a request body with system prompt 'You are a helpful assistant'
    When the middleware transforms the request for /v1/messages
    Then the system field should be an array with one content block
    And the content block should have type 'text'
    And the content block should have cache_control with type 'ephemeral'


  Scenario: Transform system prompt with OAuth prefix separation
    Given OAuth mode is enabled with Claude Code prefix
    And a request body with system prompt containing the prefix and additional text
    When the middleware transforms the request for /v1/messages
    Then the system field should be an array with two content blocks
    And the first block should contain the Claude Code prefix without cache_control
    And the second block should have cache_control with type 'ephemeral'


  Scenario: Passthrough non-messages endpoint requests
    Given a request to /v1/models endpoint
    When the middleware processes the request
    Then the request body should remain unchanged


  Scenario: Extract cache_read_input_tokens from SSE message_start
    Given an SSE message_start event with usage containing cache_read_input_tokens of 5000
    When the cache token extractor processes the SSE event
    Then turn_cache_read_tokens should be 5000


  Scenario: Extract cache_creation_input_tokens from SSE message_start
    Given an SSE message_start event with usage containing cache_creation_input_tokens of 2000
    When the cache token extractor processes the SSE event
    Then turn_cache_creation_tokens should be 2000


  Scenario: Calculate effective tokens with 90 percent cache discount
    Given a TokenTracker with input_tokens of 10000 and cache_read_input_tokens of 5000
    When effective_tokens() is called
    Then the result should be 5500 (10000 minus 4500 cache discount)


  Scenario: Passthrough OpenAI provider requests without transformation
    Given a request to OpenAI API endpoint
    When the middleware processes the request
    Then the request body should remain unchanged
    And no cache_control metadata should be added


  Scenario: Add cache_control to first user message content
    Given a request body with messages containing a first user message with string content
    When the middleware transforms the request for /v1/messages
    Then the first user message content should be an array with cache_control
    And the cache_control type should be 'ephemeral'

  # ========================================
  # INTEGRATION SCENARIOS (End-to-End Wiring)
  # ========================================

  @integration
  Scenario: ClaudeProvider uses CachingHttpClient for API requests
    Given a ClaudeProvider is created with API key authentication
    When the provider's HTTP client configuration is inspected
    Then the client should be a CachingHttpClient wrapper
    And the wrapper should have is_oauth set to false


  @integration
  Scenario: ClaudeProvider with OAuth uses CachingHttpClient with OAuth mode
    Given a ClaudeProvider is created with OAuth authentication
    When the provider's HTTP client configuration is inspected
    Then the client should be a CachingHttpClient wrapper
    And the wrapper should have is_oauth set to true
    And the wrapper should have oauth_prefix set to Claude Code prefix


  @integration
  Scenario: CachingHttpClient transforms outgoing request body
    Given a CachingHttpClient with API key mode
    And a mock request to https://api.anthropic.com/v1/messages
    And the request body has system as plain string 'You are helpful'
    When the request is executed through the CachingHttpClient
    Then the actual request body sent should have system as array with cache_control


  # NOTE: The following 3 scenarios test CacheTokenExtractor in isolation.
  # In production, rig's streaming abstraction prevents access to raw SSE data,
  # so cache tokens cannot be extracted from live streams. The extractor works
  # when given raw SSE lines, but rig processes SSE internally.

  @integration
  Scenario: Streaming response handler extracts cache tokens from SSE
    Given a streaming response handler processing Anthropic SSE events
    And a message_start SSE event arrives with cache_read_input_tokens of 5000
    When the streaming handler processes the event
    Then the cache token extractor should capture cache_read_input_tokens as 5000
    And the extracted tokens should be available after stream completion


  @integration
  Scenario: Cache tokens flow from SSE to session token tracker
    Given an interactive session with a token tracker
    And a streaming response completes with cache_read_input_tokens of 5000
    When the turn finishes processing
    Then session.token_tracker.cache_read_input_tokens should be 5000
    And session.token_tracker.effective_tokens() should return the discounted value


  @integration
  Scenario: Multiple turns accumulate cache tokens in token tracker
    Given an interactive session with a token tracker
    And a first turn completes with cache_read_input_tokens of 3000
    And a second turn completes with cache_read_input_tokens of 2000
    When both turns have finished processing
    Then session.token_tracker.cache_read_input_tokens should be 5000


  @integration
  Scenario: Non-Anthropic providers do not have cache tokens extracted
    Given an interactive session using OpenAI provider
    And a streaming response completes
    When the turn finishes processing
    Then session.token_tracker.cache_read_input_tokens should be None
    And session.token_tracker.cache_creation_input_tokens should be None

