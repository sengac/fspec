@done
@napi
@oauth
@PROV-088
Feature: Scriptable OAuth NAPI bridges - device flow + auto-refresh middleware
  """
  NAPI custom_oauth_device_start/custom_oauth_device_poll wrap a ScriptedDeviceFlow helper in codelet/providers/src/oauth/custom_oauth_device.rs that calls the script's auth_poll (fallback: poll_for_token) and normalises the result to {status, tokens?}. The ScriptedRefreshingClient in codelet/providers/src/oauth/scripted_refreshing_client.rs is an "ensure-fresh" helper that the custom-provider dispatch arm invokes before each outbound request: it reads the stored token map, calls auth_needs_refresh (fallback: needs_refresh), and on true runs auth_refresh (fallback: refresh_token), persisting the refreshed tokens through the shared CredentialStore under <fspec_home>/oauth/<provider>.json. Built-in providers keep their existing RefreshingHttpClient<TokenStrategy> untouched — resolve_refresh_middleware returns LoginImplementation::BuiltIn for them and LoginImplementation::Custom for shadowed names, reusing the single resolve_login_implementation dispatcher so /login and runtime refresh always agree.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. NAPI exposes custom_oauth_device_start(provider_name) that calls the script's device_start (fallback: build_authorization_request) and returns a map with device_code, user_code, verification_uri, interval
  #   2. NAPI exposes custom_oauth_device_poll(provider_name, device_data_json) that calls the script's auth_poll (fallback: poll_for_token) and returns {status, tokens?} where status is pending|success|denied|expired|slow_down
  #   3. When status is success, the returned tokens are persisted via the credential store keyed by provider_name so subsequent requests can use them
  #   4. An async ScriptedRefreshingClient middleware wraps RhaiCustomProvider's HTTP calls and, before each request, invokes auth_needs_refresh(tokens) followed by auth_refresh(config, tokens) whenever a refresh is required
  #   5. The middleware only activates when a Rhai shadow config exists; built-in providers keep using their existing RefreshingHttpClient<TokenStrategy> instances
  #
  # EXAMPLES:
  #   1. User runs /login codex-like; custom_oauth_device_start returns user_code ABCD-1234 and verification_uri; user enters the code in the browser
  #   2. Polling returns status=pending a few times, then status=success with tokens; tokens are stored under the provider name
  #   3. If the user denies the request in the browser, status=denied is returned and no tokens are stored
  #   4. During a long-running agent session a stored token expires; the next request triggers the middleware to call auth_refresh and seamlessly retries with the new access token
  #   5. When no shadow config exists for a provider, the middleware is inactive and built-in refresh (Codex/Claude/Copilot) runs unchanged
  #
  # ========================================
  Background: User Story
    As a user authoring a Rhai-scripted subscription provider requiring device code OAuth
    I want to drive the device-code flow and have tokens auto-refresh via middleware that calls my script's auth_poll / auth_needs_refresh / auth_refresh
    So that Codex-style device code providers can complete login and keep working across long sessions without the TS dispatcher re-implementing RFC 8628

  Scenario: User runs device-code login and receives a user_code
    Given a Rhai script shadowing provider "my-device" defines auth_start that returns user_code "ABCD-1234" and verification_uri "https://example.com/device"
    When custom_oauth_device_start("my-device") is invoked
    Then the returned payload contains user_code "ABCD-1234" and verification_uri "https://example.com/device"

  Scenario: Polling yields tokens after the user authorises the device
    Given a device-code session for provider "my-device" is active with device_code "DC-1"
    When custom_oauth_device_poll("my-device", device_data) is called and the script's auth_poll returns status="success" with access_token "AT1"
    Then the returned status is "success" and the tokens are persisted in CredentialStore under "my-device"

  Scenario: Denied polling does not persist tokens
    Given a device-code session for provider "my-device" is active and no tokens are stored yet
    When custom_oauth_device_poll returns status="denied"
    Then no credential file exists for "my-device"

  Scenario: Middleware auto-refreshes expired tokens on the next request
    Given a RhaiCustomProvider is active for "my-device" whose stored tokens are expired
    When the ScriptedRefreshingClient is asked to ensure fresh credentials before an outbound request
    Then the script's auth_needs_refresh returns true, auth_refresh is invoked, and the refreshed tokens replace the stored credentials before the request is sent

  Scenario: Built-in provider refresh is untouched when no shadow config exists
    Given no Rhai shadow config is present for "codex"
    When the dispatcher resolves the refresh middleware for "codex"
    Then the ScriptedRefreshingClient is not activated and the existing built-in refresh path is selected
