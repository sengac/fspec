@PROV-026
Feature: Claude OAuth Credential Detection and Session Routing

  """
  claude_auth.rs: read_claude_auth_sync() using std::fs for sync contexts (credentials.rs, manager.rs).
  credentials.rs: has_claude_auth() mirrors has_codex_auth() — checks claude_auth.json for OAuth tokens.
  manager.rs: get_claude() checks OAuth tokens first, uses from_oauth_tokens() with Some(0), falls back to new_with_model().
  resolver.rs: Fallback credential source for anthropic provider from claude_auth.json.
  """

  Background: User Story
    As a user with Claude Max/Pro subscription
    I want credential detection and session routing to use my OAuth tokens
    So that I can use Claude models without manually setting API keys

  Scenario: Credential detection includes claude_auth.json check
    Given claude_auth.json exists with valid access and refresh tokens
    And no ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN env vars exist
    When provider credentials are detected
    Then claude_available should be true

  Scenario: Credential detection without any Claude credentials
    Given claude_auth.json does not exist
    And no ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN env vars exist
    When provider credentials are detected
    Then claude_available should be false

  Scenario: Session creation routes to Claude provider with OAuth tokens
    Given I have authenticated with Claude via OAuth
    And claude_auth.json exists with valid access and refresh tokens
    When I create a session with model anthropic/claude-sonnet-4-20250514
    Then the provider manager should use from_oauth_tokens constructor
    And the expires_in_secs should be Some(0) to force immediate refresh

  Scenario: Session creation falls back to env var when no OAuth tokens
    Given I have not authenticated with Claude via OAuth
    And I have an ANTHROPIC_API_KEY environment variable set
    When I create a session with model anthropic/claude-sonnet-4-20250514
    Then the provider manager should use new_with_model constructor
    And the provider should use the ANTHROPIC_API_KEY for authentication

  Scenario: OAuth takes precedence over API key for session creation
    Given I have authenticated with Claude via OAuth
    And claude_auth.json exists with valid access and refresh tokens
    And I have an ANTHROPIC_API_KEY environment variable set
    When I create a session with model anthropic/claude-sonnet-4-20250514
    Then the provider manager should use from_oauth_tokens constructor
    And the ANTHROPIC_API_KEY should not be used

  Scenario: read_claude_auth_sync reads valid tokens from file
    Given claude_auth.json exists with valid access and refresh tokens
    When read_claude_auth_sync is called
    Then it should return Some with the stored tokens

  Scenario: read_claude_auth_sync returns None when file missing
    Given claude_auth.json does not exist
    When read_claude_auth_sync is called
    Then it should return Ok None
