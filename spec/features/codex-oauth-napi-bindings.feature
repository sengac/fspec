@PROV-015
Feature: NAPI Bindings for Codex OAuth Flows

  """
  New file: codelet/napi/src/codex_oauth.rs — All 4 NAPI functions in one module. Imports browser_oauth_login from codelet_providers::codex::codex_oauth_server, device_auth_login/DeviceAuthConfig from codex_device_auth, refresh_access_token from codex_oauth, and read_codex_auth from codex_auth.
  NapiCodexTokens #[napi(object)] struct maps 1:1 to CodexTokens. NapiDeviceAuthStartResult #[napi(object)] struct with user_code: String and verification_url: String — returned synchronously before polling begins. A separate async function handles the polling and returns NapiCodexTokens.
  lib.rs registration: add `mod codex_oauth;` under #[cfg(not(feature = "noop"))] and `pub use codex_oauth::*;` — same pattern as models, git, blocklist modules.
  Device auth two-phase approach: codex_oauth_device_login_start() returns NapiDeviceAuthStartResult (sync, so TUI can display user_code immediately), then codex_oauth_device_login_poll(device_auth_id, interval) is an async NAPI function that polls and returns NapiCodexTokens. This avoids the complexity of returning both a sync result and a promise from a single function.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. codex_oauth_browser_login() is an async NAPI function that spawns a tokio task to run browser_oauth_login(), returning a Promise<NapiCodexTokens> to TypeScript
  #   2. codex_oauth_device_login() is an async NAPI function that returns NapiDeviceAuthStartResult with user_code, verification_url, and a Promise<NapiCodexTokens> that resolves when polling completes
  #   3. codex_oauth_refresh_token(refresh_token: string) is an async NAPI function that calls refresh_access_token() and returns refreshed NapiCodexTokens
  #   4. codex_oauth_get_tokens() is a synchronous NAPI function that reads auth.json via read_codex_auth() and returns NapiCodexTokens or null
  #   5. All NAPI functions convert Rust errors to napi::Error via Error::from_reason() — TypeScript sees rejected promises with descriptive error messages
  #   6. NapiCodexTokens is an #[napi(object)] struct with fields: id_token, access_token, refresh_token, account_id — all strings, matching the Rust CodexTokens struct
  #   7. Device auth NAPI binding needs a two-phase design: first returns user_code and verification_url synchronously (so TUI can display them), then provides a promise that resolves when the polling completes
  #   8. The NAPI module file is codelet/napi/src/codex_oauth.rs, registered in lib.rs under #[cfg(not(feature = "noop"))]
  #
  # EXAMPLES:
  #   1. TUI calls codex_oauth_browser_login(): tokio spawns browser_oauth_login(), browser opens, user authorizes, Promise resolves with NapiCodexTokens containing id_token, access_token, refresh_token, account_id
  #   2. TUI calls codex_oauth_device_login(): Rust calls request_device_code(), returns NapiDeviceAuthStartResult with user_code='ABCD-1234' and verification_url='https://auth.openai.com/codex/device'. TUI displays these. Background promise polls and resolves with NapiCodexTokens when user completes auth
  #   3. TUI calls codex_oauth_refresh_token('rt_abc123'): Rust calls refresh_access_token(), returns NapiCodexTokens with refreshed access_token. Also persists updated tokens to auth.json and extracts account_id from the new id_token
  #   4. TUI calls codex_oauth_get_tokens() with valid auth.json: returns NapiCodexTokens with all 4 fields populated from the stored tokens
  #   5. TUI calls codex_oauth_get_tokens() with no auth.json: returns null (not an error — absence of tokens is a valid state)
  #   6. Browser login times out: codex_oauth_browser_login() Promise rejects with descriptive error 'OAuth login timed out after 300 seconds'
  #   7. Device auth polling fails with expired_token: Promise rejects with error 'Device auth failed: Device code has expired...'
  #   8. Token refresh fails (invalid refresh_token): codex_oauth_refresh_token() Promise rejects with error describing the failure status
  #
  # ========================================

  Background: User Story
    As a TUI developer
    I want to call Codex OAuth flows from TypeScript via NAPI bindings
    So that the TUI can initiate browser login, device login, token refresh, and token retrieval without leaving the TypeScript/Ink layer

  Scenario: Successful browser OAuth login via NAPI
    Given the browser OAuth flow is configured with a test server
    When TypeScript calls codex_oauth_browser_login()
    Then the Promise should resolve with NapiCodexTokens
    And the tokens should contain id_token, access_token, refresh_token, and account_id

  Scenario: Browser OAuth login times out
    Given the browser OAuth flow is configured with a short timeout
    When TypeScript calls codex_oauth_browser_login()
    And no callback is received before the timeout
    Then the Promise should reject with an error containing "timed out"

  Scenario: Device auth login start returns user code and verification URL
    Given the device auth usercode endpoint is available
    When TypeScript calls codex_oauth_device_login_start()
    Then the result should contain a user_code string
    And the result should contain a verification_url string

  Scenario: Device auth login poll resolves with tokens after user authorizes
    Given a device auth flow has been started with a valid device_auth_id
    And the device token endpoint will return authorization_code after polling
    When TypeScript calls codex_oauth_device_login_poll with the device_auth_id and interval
    Then the Promise should resolve with NapiCodexTokens
    And the tokens should be persisted to auth.json

  Scenario: Device auth polling fails with expired token
    Given a device auth flow has been started
    And the device token endpoint will return expired_token error
    When TypeScript calls codex_oauth_device_login_poll
    Then the Promise should reject with an error containing "expired"

  Scenario: Token refresh returns new tokens
    Given valid OAuth tokens exist in auth.json
    And the token endpoint accepts refresh_token requests
    When TypeScript calls codex_oauth_refresh_token with a valid refresh token
    Then the Promise should resolve with NapiCodexTokens containing a new access_token
    And the refreshed tokens should be persisted to auth.json

  Scenario: Token refresh fails with invalid refresh token
    Given the token endpoint rejects the refresh_token
    When TypeScript calls codex_oauth_refresh_token with an invalid refresh token
    Then the Promise should reject with an error describing the failure

  Scenario: Get tokens returns stored tokens from auth.json
    Given valid OAuth tokens exist in auth.json
    When TypeScript calls codex_oauth_get_tokens()
    Then the result should be NapiCodexTokens with all 4 fields populated

  Scenario: Get tokens returns null when no auth.json exists
    Given no auth.json file exists
    When TypeScript calls codex_oauth_get_tokens()
    Then the result should be null
