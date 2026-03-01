@authentication
@providers
@done
@PROV-016
Feature: Codex Custom Fetch - Token Refresh and API Rewriting
  """
  New file: codelet/providers/src/codex/refreshing_client.rs — Implements RefreshingCodexClient struct that wraps reqwest::Client and implements rig's HttpClientExt trait. Contains TokenState struct with access_token, refresh_token, account_id, and expires_at (Instant). Arc<tokio::sync::RwLock<TokenState>> for shared mutable state.
  CodexProvider struct is UNIFIED: both OAuth and API key modes use RefreshingCodexClient. Fields become CompletionModel<RefreshingCodexClient> and CompletionsClient<RefreshingCodexClient>. RefreshingCodexClient has an internal TokenMode enum: OAuth{token_state, issuer_url} for refresh+rewrite+headers, ApiKey for pass-through to reqwest. No enum wrapper or generics needed on CodexProvider itself.
  refresh_access_token_at() from codex_oauth.rs is the refresh function used — it takes an issuer URL (testable with wiremock) and returns TokenRefreshResponse with expires_in. The RefreshingCodexClient stores the issuer URL for testability.
  The rig CompletionsClient builder accepts the generic H via ClientBuilder<OpenAICompletionsExtBuilder, OpenAIApiKey, H> — we pass RefreshingCodexClient as H. The builder's .api_key() sets a dummy key (RefreshingCodexClient replaces it). No .base_url() needed since RefreshingCodexClient rewrites URLs itself.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Implement HttpClientExt trait for a RefreshingCodexClient that wraps reqwest::Client, intercepting every HTTP request to check token expiry, refresh if needed, rewrite URL, and set headers
  #   2. Token expiry is tracked using expires_in from TokenRefreshResponse — if current time >= stored expiry, refresh before the request
  #   3. Token refresh uses existing refresh_access_token() from codex_oauth.rs with the stored refresh_token
  #   4. After refresh, updated tokens (access_token, refresh_token, id_token) are persisted to auth.json via write_codex_auth() and the in-memory state is updated
  #   5. URL rewriting uses existing rewrite_codex_url() — URLs containing /v1/responses or /chat/completions are rewritten to CODEX_API_ENDPOINT
  #   6. Every request sets three headers: Authorization (Bearer {access_token}), ChatGPT-Account-Id ({account_id}), originator (codelet)
  #   7. RefreshingCodexClient holds token state in Arc<tokio::sync::RwLock<TokenState>> (NOT std::sync::RwLock) — tokio RwLock required because refresh is async and guards must be held across .await points
  #   8. CodexProvider::from_oauth_tokens() changes from building CompletionsClient<reqwest::Client> to CompletionsClient<RefreshingCodexClient> with the refreshing middleware
  #   9. If token refresh fails (network error, invalid refresh_token), the original request is NOT sent — the error propagates up to the caller
  #   10. The RefreshingCodexClient strips any existing Authorization header from the original rig request before injecting the Bearer token (rig sets a dummy key)
  #   11. Concurrent refresh uses double-check locking: read lock → check expired → drop read → write lock → RE-CHECK expired → refresh only if still expired
  #   12. If TokenRefreshResponse.expires_in is None, default to 3600 seconds (1 hour) — standard OAuth 2.0 convention. The 30-second expiry buffer still applies
  #   13. RefreshingCodexClient is used for ALL CodexProvider modes (not just OAuth). In API key mode, it operates in pass-through mode (no refresh, no URL rewrite, forwards to reqwest unchanged). This gives one consistent type: CompletionModel<RefreshingCodexClient>
  #
  # EXAMPLES:
  #   1. Token still valid: CodexProvider makes API call, RefreshingCodexClient checks expiry (30 min remaining), skips refresh, rewrites URL to CODEX_API_ENDPOINT, sets Bearer + account headers, forwards to reqwest
  #   2. Token expired mid-session: CodexProvider makes API call, RefreshingCodexClient detects expiry, calls refresh_access_token_at() with stored refresh_token, gets new tokens, persists to auth.json, updates in-memory state, retries original request with fresh token
  #   3. Token refresh fails (invalid refresh_token): RefreshingCodexClient attempts refresh, gets 401, returns error to CodexProvider without sending original request — user sees auth error
  #   4. URL rewrite for /v1/responses: rig sends request to https://api.openai.com/v1/responses (default OpenAI base URL), RefreshingCodexClient detects /v1/responses in URL, rewrites to https://chatgpt.com/backend-api/codex/responses
  #   5. URL with no rewrite needed (e.g. /v1/models): RefreshingCodexClient passes URL through unchanged but still sets auth headers
  #   6. Streaming request with expired token: RefreshingCodexClient refreshes token before forwarding the streaming request, SSE stream works with fresh credentials
  #   7. Concurrent requests during refresh: Two requests arrive while token is expired, only one refresh occurs (double-check locking ensures serialization), both requests proceed with the same refreshed token
  #   8. CodexProvider::from_oauth_tokens() integration: constructs RefreshingCodexClient with initial tokens, passes as generic H to CompletionsClient builder, provider methods use it transparently
  #   9. Token refresh response has no expires_in field: RefreshingCodexClient defaults to 3600 seconds (1 hour), applies 30s buffer, next refresh triggers at 3570 seconds
  #   10. API key mode pass-through: CodexProvider::from_api_key() creates RefreshingCodexClient in ApiKey mode, requests flow through to reqwest unchanged — no URL rewrite, no token refresh, no header injection
  #
  # ASSUMPTIONS:
  #   1. The existing from_oauth_tokens() static headers approach is INCOMPATIBLE with token refresh — once rig builds the client, headers can't change. The RefreshingCodexClient solves this by intercepting at the HTTP layer where it can dynamically set headers on every request.
  #   2. Token expiry uses a 30-second buffer (refresh when expires_at - 30s < now) to prevent edge cases where token expires between check and actual API call
  #
  # ========================================
  Background: User Story
    As a developer using Codex OAuth
    I want to have API requests automatically refresh expired tokens and route to the correct Codex endpoint
    So that my long-running sessions don't break when access tokens expire

  @happy-path
  Scenario: Request with valid token passes through with correct headers
    Given a RefreshingCodexClient in OAuth mode with a valid access token expiring in 30 minutes
    And an account ID "acc_12345"
    When the client sends a request to "https://api.openai.com/v1/chat/completions"
    Then the request URL should be rewritten to "https://chatgpt.com/backend-api/codex/responses"
    And the Authorization header should be "Bearer {access_token}"
    And the ChatGPT-Account-Id header should be "acc_12345"
    And the originator header should be "codelet"
    And no token refresh should occur

  @happy-path
  Scenario: Expired token is automatically refreshed before request
    Given a RefreshingCodexClient in OAuth mode with an expired access token
    And a valid refresh token
    When the client sends a request to "https://api.openai.com/v1/chat/completions"
    Then the client should refresh the access token via the OAuth token endpoint
    And the refreshed tokens should be persisted to auth.json
    And the request should proceed with the new access token
    And the response should be returned successfully

  @error
  Scenario: Token refresh failure propagates error without sending request
    Given a RefreshingCodexClient in OAuth mode with an expired access token
    And an invalid refresh token that returns a 401 error
    When the client sends a request to "https://api.openai.com/v1/chat/completions"
    Then the client should attempt to refresh the access token
    And the refresh should fail with an authentication error
    And the original API request should NOT be sent
    And the error should propagate to the caller

  Scenario: URL rewrite for /v1/responses path
    Given a RefreshingCodexClient in OAuth mode with a valid access token
    When the client sends a request to "https://api.openai.com/v1/responses"
    Then the request URL should be rewritten to "https://chatgpt.com/backend-api/codex/responses"

  Scenario: URL rewrite for /chat/completions path
    Given a RefreshingCodexClient in OAuth mode with a valid access token
    When the client sends a request to "https://api.openai.com/v1/chat/completions"
    Then the request URL should be rewritten to "https://chatgpt.com/backend-api/codex/responses"

  Scenario: Non-API URLs pass through without rewrite
    Given a RefreshingCodexClient in OAuth mode with a valid access token
    When the client sends a request to "https://api.openai.com/v1/models"
    Then the request URL should NOT be rewritten
    And the auth headers should still be set correctly

  @happy-path
  Scenario: Streaming request with expired token refreshes before streaming
    Given a RefreshingCodexClient in OAuth mode with an expired access token
    And a valid refresh token
    When the client sends a streaming request to "https://api.openai.com/v1/chat/completions"
    Then the client should refresh the access token before streaming
    And the streaming response should use the refreshed credentials
    And the SSE stream should be returned successfully

  Scenario: Existing Authorization header is replaced with Bearer token
    Given a RefreshingCodexClient in OAuth mode with a valid access token
    And the original request has a dummy Authorization header "Bearer dummy-api-key" set by rig
    When the client sends the request
    Then the dummy Authorization header should be stripped
    And replaced with "Bearer {current_access_token}"

  # Note: This tests Rust-level type integration, NOT TUI agent loop dispatch.
  # TUI wiring (adding "codex" branch to run_with_provider!) is tracked by PROV-017.
  @integration
  Scenario: CodexProvider uses RefreshingCodexClient for OAuth mode
    Given OAuth tokens with access_token, refresh_token, and account_id
    When CodexProvider::from_oauth_tokens() is called
    Then a RefreshingCodexClient should be created with OAuth TokenMode
    And it should be passed as the HTTP client to rig CompletionsClient<RefreshingCodexClient>
    And the provider should be able to construct a rig Agent

  Scenario: Token refresh within expiry buffer triggers proactive refresh
    Given a RefreshingCodexClient in OAuth mode with a token expiring in 20 seconds
    And the expiry buffer is 30 seconds
    When the client sends a request
    Then the client should proactively refresh the token
    And the request should use the refreshed token

  Scenario: API key mode passes requests through unchanged
    Given a RefreshingCodexClient in ApiKey mode
    When the client sends a request to "https://api.openai.com/v1/chat/completions"
    Then the request URL should NOT be rewritten
    And no token refresh should occur
    And the original headers from rig should be preserved
    And the request should be forwarded to reqwest as-is

  Scenario: Default expiry when expires_in is not provided
    Given a RefreshingCodexClient in OAuth mode
    And a token refresh response with no expires_in field
    When the token is refreshed
    Then the expiry should default to 3600 seconds from now
    And the 30-second buffer should still apply
    And a request sent 3571 seconds later should trigger a refresh

  Scenario: Tokens loaded from disk trigger immediate refresh via Some(0)
    Given OAuth tokens exist in ~/.codex/auth.json from a previous session
    When CodexProvider passes Some(0) for expires_in_secs to RefreshingCodexClient
    Then the token is immediately considered expired
    Given the access token may be expired
    Then the first API request triggers a token refresh before sending

