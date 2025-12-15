@PROV-002
Feature: Claude Code OAuth Authentication

  """
  Architecture notes:
  - Port OAuth auth from codelet TypeScript implementation (claude-auth.ts, claude-provider.ts)
  - Custom HTTP client wrapper intercepts requests to modify headers and body for OAuth
  - System prompt MUST start with exact Claude Code identifier: "You are Claude Code, Anthropic's official CLI for Claude."
  - Beta features: oauth-2025-04-20, prompt-caching-2024-07-31, interleaved-thinking-2025-05-14
  - Request body must include metadata.user_id field
  - URL must have ?beta=true query parameter for messages endpoint
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. MUST send Authorization: Bearer header instead of x-api-key when using OAuth token
  #   2. MUST include anthropic-beta header with oauth-2025-04-20 and prompt-caching-2024-07-31
  #   3. MUST prepend system prompt with 'You are Claude Code, Anthropic's official CLI for Claude.'
  #   4. MUST add ?beta=true query parameter to messages endpoint URL
  #   5. MUST include metadata.user_id in request body
  #   6. SHOULD prefer ANTHROPIC_API_KEY over CLAUDE_CODE_OAUTH_TOKEN when both are present
  #
  # EXAMPLES:
  #   1. User has CLAUDE_CODE_OAUTH_TOKEN set, agent authenticates with Bearer token and Claude Code system prompt
  #   2. User has both ANTHROPIC_API_KEY and CLAUDE_CODE_OAUTH_TOKEN, agent uses ANTHROPIC_API_KEY
  #   3. User has only CLAUDE_CODE_OAUTH_TOKEN, API request includes beta headers and system prompt prefix
  #
  # ========================================

  Background: User Story
    As a developer with Claude Code subscription
    I want to use CLAUDE_CODE_OAUTH_TOKEN for API authentication
    So that I can use the agent without needing a separate Anthropic API key

  Scenario: OAuth token authentication with Bearer header and system prompt
    Given CLAUDE_CODE_OAUTH_TOKEN is set in the environment
    And ANTHROPIC_API_KEY is not set
    When the provider sends a request to the Claude API
    Then the request should use Authorization: Bearer header instead of x-api-key
    And the request should include anthropic-beta header with "oauth-2025-04-20,prompt-caching-2024-07-31"
    And the system prompt should start with "You are Claude Code, Anthropic's official CLI for Claude."
    And the request URL should include "?beta=true" query parameter
    And the request body should include metadata.user_id

  Scenario: API key takes precedence over OAuth token
    Given both ANTHROPIC_API_KEY and CLAUDE_CODE_OAUTH_TOKEN are set in the environment
    When the provider is initialized
    Then the provider should use ANTHROPIC_API_KEY for authentication
    And the request should use x-api-key header

  Scenario: OAuth request includes all required beta headers and body modifications
    Given CLAUDE_CODE_OAUTH_TOKEN is set in the environment
    When a completion request is sent
    Then the anthropic-beta header should include "oauth-2025-04-20"
    And the anthropic-beta header should include "prompt-caching-2024-07-31"
    And the first system block should contain only the Claude Code identifier
    And additional system content should be in subsequent blocks with cache_control
