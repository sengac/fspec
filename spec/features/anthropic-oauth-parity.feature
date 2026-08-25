@PROV-027
Feature: Anthropic OAuth parity with opencode behavior
  """
  PARITY tests comparing our behavior against opencode's anthropic-auth plugin.
  Rust tests in providers/tests/ for core OAuth primitives, headers, tool prefixing,
  URL rewriting, system prompt, cost zeroing, token refresh, concurrent refresh,
  and credential fallback chain.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tool names must be prefixed with mcp_ in requests and stripped on responses
  #   2. URL rewriting must append ?beta=true to /v1/messages endpoint
  #   3. OAuth requests must include merged anthropic-beta header with deduplication
  #   4. System prompt must include 'You are Claude Code' prefix in OAuth mode
  #   5. Token refresh must check expiry BEFORE each request with 30s buffer
  #   6. Auth credential fallback: OAuth tokens take precedence over ANTHROPIC_API_KEY
  #   7. Concurrent token refresh must use double-check locking
  #
  # ASSUMPTIONS:
  #   1. This card is pure TESTING — parity and regression tests against existing code
  #
  # ========================================
  Background: User Story
    As a developer with a Claude Max/Pro subscription
    I want parity between codelet Anthropic OAuth and opencode behavior
    So that OAuth login, token refresh, and request modification work correctly

  # ==========================================================================
  # PARITY SCENARIOS — Matching opencode's anthropic-auth plugin behavior
  # ==========================================================================
  # Architecture: mcp_ prefixing is a parity reference — codelet uses native tools
  # (not MCP), so prefixing is not applied in the production request path. These
  # functions exist for parity verification against opencode and future MCP support.
  @parity
  @tool-prefixing
  Scenario: Tool names are prefixed with mcp_ in OAuth mode requests
    Given I am authenticated with Claude via OAuth
    When a request is sent with tool definitions and tool_use blocks
    Then tool names in tool definitions should be prefixed with "mcp_"
    And tool_use block names in messages should be prefixed with "mcp_"
    And tool names in streaming responses should have "mcp_" prefix stripped

  @parity
  @url-rewriting
  Scenario: API URL is rewritten to append beta query parameter in OAuth mode
    Given I am authenticated with Claude via OAuth
    When a request is sent to /v1/messages
    Then the URL should have "?beta=true" appended
    And a URL that already has "?beta=true" should not be duplicated
    And non-messages URLs should pass through unchanged

  @parity
  @headers
  Scenario: OAuth requests include merged beta headers and Bearer auth
    Given I am authenticated with Claude via OAuth
    And the request has existing beta headers "max-tokens-3-5-sonnet-2024-07-15"
    When the request is prepared for the Claude API
    Then the Authorization header should be "Bearer {access_token}"
    And the anthropic-beta header should contain "oauth-2025-04-20"
    And the anthropic-beta header should contain "interleaved-thinking-2025-05-14"
    And the anthropic-beta header should contain "max-tokens-3-5-sonnet-2024-07-15"
    And the anthropic-beta header should have no duplicate entries
    And the user-agent should be "claude-cli/2.1.3 (external, cli)"
    And x-api-key should not be present

  @parity
  @system-prompt
  Scenario: System prompt includes Claude Code identity prefix in OAuth mode
    Given I am authenticated with Claude via OAuth
    When the system prompt is built for a request
    Then the first system block should start with "You are Claude Code, Anthropic's official CLI for Claude."
    And app name references should be replaced with "Claude Code"

  @parity
  @costs
  Scenario: OAuth max plan users see zero costs
    Given I am authenticated with Claude via OAuth with a Max subscription
    When the provider is queried for its auth mode
    Then the provider should report OAuth mode for cost zeroing
    And API key mode providers should not be flagged for cost zeroing

  @regression
  @token-refresh
  Scenario: Tokens loaded from disk force immediate refresh on first API call
  # ==========================================================================
  # REGRESSION SCENARIOS — Token refresh and credential fallback
  # ==========================================================================
    Given claude_auth.json contains week-old tokens with expired access_token
    When the provider creates a RefreshingClaudeClient from disk tokens
    Then expires_in_secs should be Some(0) to force immediate refresh
    And the first API call should trigger a token refresh before sending
    And the API call should succeed with the refreshed token

  @regression
  @token-refresh
  Scenario: Concurrent requests during expired token only refresh once
    Given I am authenticated with Claude via OAuth
    And the access token is expired
    When two simultaneous API requests are made
    Then only one HTTP refresh call should be made to the token endpoint
    And both requests should proceed with the same refreshed token

  @regression
  @credential-fallback
  Scenario: OAuth tokens take precedence over API key in credential chain
    Given claude_auth.json exists with valid OAuth tokens
    And ANTHROPIC_API_KEY environment variable is set
    When the provider manager creates a Claude provider
    Then the provider should be in OAuth mode
    And the ANTHROPIC_API_KEY should not be used for authentication

  @regression
  @credential-fallback
  Scenario: API key fallback works when no OAuth tokens exist
    Given claude_auth.json does not exist
    And ANTHROPIC_API_KEY environment variable is set
    When the provider manager creates a Claude provider
    Then the provider should be in API key mode
    And the ANTHROPIC_API_KEY should be used for authentication
