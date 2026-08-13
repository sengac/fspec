@done
@providers
@llm-provider
@provider-management
@high
@PROV-004
Feature: Codex Provider (ChatGPT Backend API with OAuth)

  """
  Uses rig-core 0.25.0 OpenAI provider with custom base URL (ClientBuilder.base_url()). OAuth token management via refresh_token flow (POST to https://auth.openai.com/oauth/token). Token exchange grant (urn:ietf:params:oauth:grant-type:token-exchange) converts id_token to OpenAI API key (sk-proj-...). Credentials read from ~/.codex/auth.json or macOS keychain (service='Codex Auth'). Model: gpt-5.1-codex (272K context, 4K output). Follows ClaudeProvider/OpenAIProvider pattern with LlmProvider trait. Dependencies: keyring crate (macOS keychain), sha2 (key computation), uuid (conversation_id), serde_json (auth.json parsing).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Codex credentials are read from ~/.codex/auth.json (or $CODEX_HOME/auth.json if CODEX_HOME is set)
  #   2. On macOS, credentials are read from keychain first (service: 'Codex Auth', account: 'cli|{first 16 chars of sha256(CODEX_HOME)}'), then fall back to file
  #   3. auth.json contains two authentication modes: OPENAI_API_KEY (preferred) or tokens.{id_token, access_token, refresh_token, account_id}
  #   4. OAuth token refresh flow: POST to https://auth.openai.com/oauth/token with grant_type=refresh_token, client_id=app_EMoamEEZ73f0CkXaXp7hrann
  #   5. Token exchange flow: POST to https://auth.openai.com/oauth/token with grant_type=urn:ietf:params:oauth:grant-type:token-exchange to convert id_token into OpenAI API key (sk-proj-...)
  #   6. Provider must implement LlmProvider trait (complete, complete_with_tools, supports_streaming, supports_caching) following ClaudeProvider and OpenAIProvider patterns
  #   7. Default model is gpt-5.1-codex, configurable via CODEX_MODEL environment variable
  #   8. Codex does not support prompt caching - supports_caching() must return false
  #   9. Provider must fail gracefully with clear error messages for: missing auth.json, expired tokens, network failures, invalid refresh_token
  #   10. PREFER token exchange approach: (1) Refresh tokens → get fresh id_token, (2) Exchange id_token for sk-proj-... API key via urn:ietf:params:oauth:grant-type:token-exchange, (3) Use standard OpenAI API with exchanged key. This is simpler, more stable, and reuses existing OpenAIProvider infrastructure. Fall back to direct backend API only if exchange fails.
  #   11. gpt-5.1-codex model has 272,000 token context window (found in /tmp/codex/codex-rs/core/src/openai_model_info.rs:72). Max output tokens not specified - assume 4,096 like other GPT models.
  #
  # EXAMPLES:
  #   1. User has ~/.codex/auth.json with OPENAI_API_KEY field set to 'sk-proj-abc123', CodexProvider::new() succeeds and uses this API key directly
  #   2. User has ~/.codex/auth.json with tokens.refresh_token but no OPENAI_API_KEY, provider refreshes tokens and exchanges id_token for API key, caches result in auth.json
  #   3. User sends prompt 'Hello\!' via complete(), Codex API returns response, provider returns text
  #   4. User has no ~/.codex/auth.json, CodexProvider::new() returns error: 'CODEX auth.json not found at ~/.codex/auth.json. Run codex auth login to authenticate.'
  #   5. User on macOS has credentials in keychain (service='Codex Auth'), provider reads from keychain successfully and uses those credentials
  #   6. User sets CODEX_HOME=/custom/path, provider reads auth.json from /custom/path/auth.json instead of ~/.codex/auth.json
  #   7. User sets CODEX_MODEL=gpt-4o, provider uses gpt-4o instead of default gpt-5.1-codex
  #   8. OAuth token refresh fails with 401, provider returns error: 'Failed to refresh Codex tokens. Token may be expired. Run codex auth login to re-authenticate.'
  #
  # QUESTIONS (ANSWERED):
  #   Q: @ai: Does rig-core 0.25.0 support custom base URLs for OpenAI provider? Need to check if we can override baseURL to use https://chatgpt.com/backend-api/codex
  #   A: YES - rig-core 0.25.0 ClientBuilder has .base_url() method (found in /tmp/rig/rig-core/src/client/mod.rs:488-493). Can override default https://api.openai.com/v1 with custom URL. For Codex: openai::CompletionsClient::builder().api_key(token).base_url('https://chatgpt.com/backend-api/codex').build()
  #
  #   Q: @ai: What are the context window and max output token limits for gpt-5.1-codex model? Should we assume same as gpt-4-turbo (128K/4K) or query the API?
  #   A: YES - rig-core 0.25.0 supports custom base URLs via ClientBuilder.base_url() method (found in /tmp/rig/rig-core/src/client/mod.rs:488). We can use: openai::CompletionsClient::builder().api_key(key).base_url('https://chatgpt.com/backend-api/codex').build()
  #
  #   Q: @ai: Should we prefer token exchange (get sk-proj-... API key) or direct ChatGPT backend API (with access_token)? Token exchange is simpler and uses standard OpenAI API.
  #   A: PREFER token exchange approach: (1) Refresh tokens → get fresh id_token, (2) Exchange id_token for sk-proj-... API key via urn:ietf:params:oauth:grant-type:token-exchange, (3) Use standard OpenAI API with exchanged key. This is simpler, more stable, and reuses existing OpenAIProvider infrastructure. Fall back to direct backend API only if exchange fails.
  #
  # ASSUMPTIONS:
  #   1. YES - rig-core 0.25.0 supports custom base URLs via ClientBuilder.base_url() method (found in /tmp/rig/rig-core/src/client/mod.rs:488). We can use: openai::CompletionsClient::builder().api_key(key).base_url('https://chatgpt.com/backend-api/codex').build()
  #   2. gpt-5.1-codex is undocumented. ASSUME same limits as gpt-4-turbo: 128,000 context window, 4,096 max output tokens. Document in code that these are assumptions pending API documentation.
  #   3. YES - rig-core 0.25.0 ClientBuilder has .base_url() method (found in /tmp/rig/rig-core/src/client/mod.rs:488-493). Can override default https://api.openai.com/v1 with custom URL. For Codex: openai::CompletionsClient::builder().api_key(token).base_url('https://chatgpt.com/backend-api/codex').build()
  #
  # ========================================

  Background: User Story
    As a developer using codelet with GitHub Copilot CLI credentials
    I want to authenticate to OpenAI's ChatGPT backend API using OAuth tokens from ~/.codex/auth.json
    So that I can use codelet with my existing Codex CLI authentication instead of managing separate API keys

  Scenario: Initialize with cached API key from auth.json
    Given the file "~/.codex/auth.json" exists
    And the auth.json contains OPENAI_API_KEY field set to "sk-proj-abc123"
    When I call CodexProvider::new()
    Then the provider should initialize successfully
    And the provider should use the cached API key directly
    And the provider should use model "gpt-5.1-codex"
    And the provider name should be "codex"
    And the context window should be 272000
    And the max output tokens should be 4096

  Scenario: Initialize with refresh token and perform token exchange
    Given the file "~/.codex/auth.json" exists
    And the auth.json contains tokens.refresh_token but no OPENAI_API_KEY
    When I call CodexProvider::new()
    Then the provider should refresh OAuth tokens via POST to https://auth.openai.com/oauth/token
    And the provider should exchange id_token for OpenAI API key via token exchange grant
    And the provider should cache the exchanged API key in auth.json as OPENAI_API_KEY field
    And the provider should initialize successfully

  Scenario: Complete simple prompt without tools
    Given I have an initialized Codex provider
    And I have a message with role "user" and content "Hello!"
    When I call complete() with the messages
    Then the provider should return text response
    And the response should be non-empty

  Scenario: Fail when auth.json does not exist
    Given the file "~/.codex/auth.json" does not exist
    And the CODEX_HOME environment variable is not set
    When I call CodexProvider::new()
    Then I should receive an error
    And the error message should contain "CODEX auth.json not found at ~/.codex/auth.json"
    And the error message should contain "Run codex auth login to authenticate"

  Scenario: Read credentials from macOS keychain
    Given I am on macOS platform
    And the keychain contains credentials for service "Codex Auth"
    And the keychain account is "cli|{first 16 chars of sha256(CODEX_HOME)}"
    When I call CodexProvider::new()
    Then the provider should read credentials from keychain first
    And the provider should initialize successfully using keychain credentials

  Scenario: Support custom CODEX_HOME path
    Given the CODEX_HOME environment variable is set to "/custom/path"
    And the file "/custom/path/auth.json" exists with valid credentials
    When I call CodexProvider::new()
    Then the provider should read auth.json from "/custom/path/auth.json"
    And the provider should initialize successfully

  Scenario: Override model via CODEX_MODEL environment variable
    Given I have valid Codex credentials
    And the CODEX_MODEL environment variable is set to "gpt-4o"
    When I call CodexProvider::new()
    Then the provider should initialize successfully
    And the provider should use model "gpt-4o" instead of default "gpt-5.1-codex"

  Scenario: Handle OAuth token refresh failure gracefully
    Given the file "~/.codex/auth.json" exists with tokens.refresh_token
    And the refresh token is expired or invalid
    When I call CodexProvider::new()
    And the OAuth token refresh fails with 401 status
    Then I should receive an error
    And the error message should contain "Failed to refresh Codex tokens"
    And the error message should contain "Token may be expired"
    And the error message should contain "Run codex auth login to re-authenticate"

  Scenario: Provider reports no prompt caching support
    Given I have an initialized Codex provider
    When I call supports_caching()
    Then it should return false

  Scenario: Provider reports streaming support
    Given I have an initialized Codex provider
    When I call supports_streaming()
    Then it should return true

  Scenario: Create rig agent with all tools configured
    Given I have an initialized Codex provider
    When I call create_rig_agent()
    Then a rig Agent should be created
    And the agent should have 7 tools configured
    And the agent should use the provider's model name
    And the agent should have max_tokens set to 4096
