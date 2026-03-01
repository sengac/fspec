@done
@codex-oauth
@PROV-014
Feature: Device Auth Flow for Headless Environments

  """
  New file: codelet/providers/src/codex/codex_device_auth.rs - Implements the device authorization flow. Three new API functions: request_device_code() POSTs to usercode endpoint, poll_device_token() polls the token endpoint, and device_auth_login() orchestrates the full flow (request code → display → poll → exchange → persist). Returns Result<CodexTokens>.
  Reuses extract_account_id() from codex_oauth.rs for JWT parsing, write_codex_auth() from codex_auth.rs for persistence, and CodexTokens/CodexAuthJson structs. CANNOT reuse exchange_authorization_code() because it requires redirect_uri — needs a new exchange_device_code() function or pub(crate) post_to_token_endpoint().
  DeviceAuthConfig struct for testability (same pattern as OAuthServerConfig in codex_oauth_server.rs): issuer_url (test vs production), timeout_ms (short for tests), display callback (for TUI integration later). Tests use wiremock for the HTTP endpoints.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Device auth starts with POST to {ISSUER}/api/accounts/deviceauth/usercode with client_id, returns device_auth_id, user_code, and polling interval
  #   2. User must be shown the user_code and the verification URL ({ISSUER}/codex/device) so they can complete auth on another device
  #   3. System polls {ISSUER}/api/accounts/deviceauth/token with device_auth_id at the interval returned by the usercode endpoint
  #   4. When user completes auth, polling endpoint returns authorization_code and code_verifier
  #   5. Tokens (refresh_token, access_token, account_id) persisted to ~/.codex/auth.json using existing write_codex_auth() - identical output to browser OAuth flow
  #   6. Device auth flow must have a timeout - if user never completes auth on external device, flow fails gracefully
  #   7. Polling must handle 'authorization_pending' status gracefully - continue polling without error
  #   8. Polling must stop on terminal errors (expired_token, access_denied) and report the failure
  #   9. Device auth produces the same CodexTokens output as browser OAuth (PROV-013), ensuring both flows are interchangeable
  #   10. authorization_code is exchanged for tokens via POST to {ISSUER}/oauth/token with grant_type=authorization_code, code, code_verifier, and client_id — no redirect_uri
  #   11. Polling must handle slow_down response by increasing the polling interval by 5 seconds before retrying (RFC 8628 Section 3.5)
  #
  # EXAMPLES:
  #   1. User initiates device auth: POST to usercode endpoint returns device_auth_id='dev_abc123', user_code='ABCD-1234', interval=5. User sees 'Enter code ABCD-1234 at https://auth.openai.com/codex/device'. Polling starts. After user authorizes on phone, poll returns authorization_code and code_verifier. Code exchanged for tokens. Tokens persisted to auth.json.
  #   2. Polling receives authorization_pending status: system waits for the specified interval (e.g. 5 seconds) and polls again without error
  #   3. Polling receives expired_token error: device code has expired (user took too long), flow terminates with clear error message
  #   4. Polling receives access_denied error: user explicitly denied authorization, flow terminates with clear error message
  #   5. Device auth flow times out: user never authorizes, overall timeout expires, flow terminates gracefully
  #   6. Usercode endpoint returns error (network failure): flow terminates immediately with descriptive error, no polling attempted
  #   7. Token exchange after device auth succeeds: authorization_code + code_verifier exchanged at /oauth/token with grant_type=authorization_code (no redirect_uri), returns tokens including id_token with account_id claim
  #   8. Device auth output matches browser OAuth: both flows produce identical CodexTokens struct with id_token, access_token, refresh_token, account_id
  #   9. Polling receives slow_down response: system increases polling interval by 5 seconds (e.g. 5s becomes 10s) and continues polling
  #
  # ASSUMPTIONS:
  #   1. No browser dependency, no HTTP server, no port binding - device auth is purely an HTTP client flow (POST + poll). Much simpler than browser OAuth. The polling loop uses tokio::time::sleep for interval delays and tokio::time::timeout for overall timeout.
  #   2. NAPI bindings for device auth will be done in PROV-015 (separate story). This story only implements the Rust core.
  #
  # ========================================

  Background: User Story
    As a developer using codelet in a headless environment
    I want to authenticate with my ChatGPT subscription via device auth flow
    So that I can use Codex models from SSH sessions, containers, and headless servers where a browser can't be opened

  Scenario: Successful device auth login completes end-to-end
    Given no Codex credentials exist in auth.json
    When the user initiates device auth login
    Then a device authorization request should be POST-ed to the usercode endpoint with client_id
    And the response should contain a device_auth_id, user_code, and polling interval
    And the user should see the user_code and the verification URL to visit
    When the system polls the token endpoint at the specified interval
    And the user completes authorization on the external device
    Then the polling endpoint should return an authorization_code and code_verifier
    And the authorization_code should be exchanged for tokens at the token endpoint without redirect_uri
    And the account_id should be extracted from the JWT id_token claims
    And the tokens should be persisted to auth.json with refresh_token, access_token, and account_id

  Scenario: Polling continues on authorization_pending status
    Given a device auth login is in progress with a 5-second polling interval
    When the token polling endpoint returns authorization_pending status
    Then the system should wait for 5 seconds
    And the system should poll the token endpoint again without error

  Scenario: Polling backs off on slow_down response
    Given a device auth login is in progress with a 5-second polling interval
    When the token polling endpoint returns a slow_down error
    Then the polling interval should be increased by 5 seconds to 10 seconds
    And the system should continue polling at the new interval

  Scenario: Polling stops on expired_token error
    Given a device auth login is in progress
    When the token polling endpoint returns an expired_token error
    Then the device auth flow should terminate with an error
    And the error should indicate the device code has expired

  Scenario: Polling stops on access_denied error
    Given a device auth login is in progress
    When the token polling endpoint returns an access_denied error
    Then the device auth flow should terminate with an error
    And the error should indicate the user denied authorization

  Scenario: Device auth flow times out
    Given a device auth login is in progress with a short timeout
    When the timeout expires without the user completing authorization
    Then the device auth flow should terminate with a timeout error
    And no tokens should be persisted

  Scenario: Usercode endpoint network failure
    When the user initiates device auth login
    And the usercode endpoint is unreachable
    Then the device auth flow should terminate immediately with a network error
    And no polling should be attempted

  Scenario: Token exchange uses correct parameters without redirect_uri
    Given a device auth login received a successful polling response
    And the response contains authorization_code and code_verifier
    When the authorization_code is exchanged at the token endpoint
    Then the exchange should POST grant_type authorization_code, code, code_verifier, and client_id
    And the exchange should NOT include a redirect_uri parameter
    And the response should contain id_token, access_token, and refresh_token

  Scenario: Device auth produces same CodexTokens output as browser OAuth
    Given a device auth login completes successfully
    When the tokens are returned
    Then the output should be a CodexTokens with id_token, access_token, refresh_token, and account_id
    And the output should be identical in structure to browser OAuth login output
