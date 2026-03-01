@PROV-011
Feature: Codex OAuth Login Flow - Browser and Device Auth for ChatGPT Subscription

  """
  Rust implementation: codex_oauth.rs module alongside existing codex_auth.rs. PKCE using sha2+base64 crates (already in dependencies). HTTP server using hyper (lightweight, already used by rig). Browser open via open crate.
  CodexProvider::new() changes: Try read_codex_auth() first (existing path). If no credentials found, initiate OAuth flow. After successful OAuth, persist tokens and create provider with access_token as Bearer auth + endpoint rewrite.
  Two auth modes for CodexProvider: (1) Legacy token-exchange mode (existing - gets OpenAI API key from id_token), (2) Direct Codex API mode (new - Bearer access_token to chatgpt.com/backend-api/codex). OpenCode uses mode 2 exclusively.
  NAPI binding: codex_oauth_browser_login() → starts server, returns auth URL. codex_oauth_device_login() → returns user_code+URL, polls internally. Both return serialized tokens. TUI calls these via NAPI from ProviderSettingsScreen.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Browser OAuth uses PKCE (RFC 7636) with S256 code challenge method - client_id app_EMoamEEZ73f0CkXaXp7hrann, issuer https://auth.openai.com
  #   2. Local HTTP server listens on port 1455 for OAuth callback at /auth/callback
  #   3. Device auth flow: POST to /api/accounts/deviceauth/usercode, display user_code, poll /api/accounts/deviceauth/token at specified interval
  #   4. Token refresh uses grant_type=refresh_token with client_id and refresh_token against /oauth/token
  #   5. Account ID extracted from JWT claims: chatgpt_account_id, https://api.openai.com/auth.chatgpt_account_id, or organizations[0].id
  #   6. API requests rewritten to https://chatgpt.com/backend-api/codex/responses with Bearer access_token and ChatGPT-Account-Id header
  #   7. OAuth tokens (refresh_token, access_token, account_id, expiry) persisted to ~/.codex/auth.json for compatibility with Codex CLI
  #   8. Access tokens auto-refresh when expired before making API calls
  #   9. OAuth callback validates state parameter to prevent CSRF attacks
  #   10. Browser OAuth has a 5-minute timeout - if no callback received, the flow fails gracefully
  #
  # EXAMPLES:
  #   1. User initiates browser OAuth login: PKCE challenge generated, browser opens auth URL, local server receives callback with code, code exchanged for tokens, tokens persisted
  #   2. User initiates device auth (headless): POST gets device_auth_id and user_code, user shown code and URL, polling detects completion, authorization_code exchanged for tokens
  #   3. Access token expired, API call triggers auto-refresh: refresh_token sent to /oauth/token, new access_token received, API call succeeds with fresh token
  #   4. JWT id_token contains chatgpt_account_id claim: extracted and used as ChatGPT-Account-Id header in API requests
  #   5. OAuth callback receives mismatched state parameter: login rejected with CSRF error, user sees error page
  #   6. Browser OAuth times out after 5 minutes: user sees timeout error, server shut down cleanly
  #   7. Codex API request: URL rewritten from /v1/responses to chatgpt.com/backend-api/codex/responses, Bearer token added, ChatGPT-Account-Id header added
  #   8. Existing ~/.codex/auth.json with valid tokens: codelet reads and uses them without requiring fresh login
  #
  # ASSUMPTIONS:
  #   1. TUI integration (ProviderSettingsScreen login option) is a separate story - this story focuses on the Rust OAuth core + NAPI bindings
  #   2. We maintain backward compatibility with existing ~/.codex/auth.json format and Codex CLI keychain storage
  #
  # ========================================

  Background: User Story
    As a ChatGPT Plus/Pro subscriber
    I want to authenticate with my OpenAI account directly from codelet
    So that use Codex models without needing to install and run the Codex CLI first

  Scenario: Browser OAuth login with PKCE completes successfully
    Given no Codex credentials exist in auth.json or keychain
    And a local HTTP server can bind to port 1455
    When the user initiates browser OAuth login
    Then a PKCE code verifier and S256 challenge should be generated
    And the OAuth authorize URL should include client_id "app_EMoamEEZ73f0CkXaXp7hrann"
    And the OAuth authorize URL should include the PKCE challenge and state parameter
    And the system should open the browser to the authorize URL
    And the local server should listen on port 1455 for the callback
    When the OAuth callback arrives with a valid authorization code and matching state
    Then the code should be exchanged for tokens at the issuer token endpoint
    And the tokens should be persisted to auth.json with refresh_token, access_token, and account_id

  Scenario: Device auth login for headless environments
    Given no Codex credentials exist in auth.json or keychain
    And the environment does not support opening a browser
    When the user initiates device auth login
    Then a device authorization request should be sent to the usercode endpoint
    And the user should see a user code and a URL to visit
    And the system should poll the token endpoint at the specified interval
    When the user completes authorization on the external device
    Then the authorization code should be exchanged for tokens
    And the tokens should be persisted to auth.json

  Scenario: Access token auto-refresh when expired
    Given valid Codex OAuth tokens exist with an expired access_token
    And the refresh_token is still valid
    When an API call is made to the Codex endpoint
    Then the access token should be refreshed using the refresh_token grant
    And the new access_token should replace the expired one in storage
    And the API call should proceed with the fresh access token

  Scenario: Account ID extracted from JWT id_token claims
    Given an OAuth token response contains an id_token JWT
    And the id_token payload contains a "chatgpt_account_id" claim
    When the account ID is extracted from the token response
    Then the chatgpt_account_id should be returned
    And subsequent API requests should include the ChatGPT-Account-Id header

  Scenario: Account ID extracted from nested JWT claims
    Given an OAuth token response contains an id_token JWT
    And the id_token payload contains "https://api.openai.com/auth" with chatgpt_account_id
    When the account ID is extracted from the token response
    Then the nested chatgpt_account_id should be returned

  Scenario: Account ID extracted from organizations claim as fallback
    Given an OAuth token response contains an id_token JWT
    And the id_token payload has no chatgpt_account_id but has organizations array
    When the account ID is extracted from the token response
    Then the first organization ID should be returned as the account ID

  Scenario: OAuth callback rejects mismatched state parameter
    Given a browser OAuth login is in progress with a known state value
    When the OAuth callback arrives with a different state parameter
    Then the login should be rejected with a CSRF error
    And the browser should show an error page explaining the failure
    And the pending OAuth flow should be cleaned up

  Scenario: Browser OAuth times out after 5 minutes
    Given a browser OAuth login is in progress
    When no callback is received within 5 minutes
    Then the OAuth flow should fail with a timeout error
    And the local HTTP server should be shut down cleanly
    And the pending OAuth state should be cleared

  Scenario: API requests rewritten to Codex endpoint with OAuth headers
    Given valid Codex OAuth tokens exist with access_token and account_id
    When an API request is made to the standard OpenAI completions endpoint
    Then the URL should be rewritten to chatgpt.com/backend-api/codex/responses
    And the Authorization header should use Bearer with the access_token
    And the ChatGPT-Account-Id header should be set to the account_id
    And the originator header should be set

  Scenario: Existing credentials used without fresh login
    Given valid Codex OAuth tokens exist in auth.json with a non-expired access_token
    When the Codex provider is initialized
    Then the existing tokens should be used directly
    And no OAuth login flow should be initiated

  Scenario: PKCE code verifier meets RFC 7636 requirements
    When a PKCE challenge is generated
    Then the code verifier should be between 43 and 128 characters
    And the code verifier should only contain unreserved URI characters
    And the code challenge should be the Base64URL-encoded SHA-256 hash of the verifier
    And the code challenge method should be "S256"
