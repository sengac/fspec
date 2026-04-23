@done
@oauth
@napi
@PROV-087
Feature: Scriptable OAuth NAPI bridges - browser loopback + PKCE flow
  """
  New file codelet/napi/src/custom_oauth.rs wraps codelet/providers ScriptedOAuthProvider + callback_server.rs. Exposes custom_oauth_authorize / _exchange / _needs_refresh / _refresh / _clear through NAPI. TypeScript /login dispatcher (in codelet-tui/src/login) routes to custom_oauth_* when a Rhai shadow config is found, else falls back to existing claude_oauth / codex_oauth / copilot_oauth bindings. Tokens round-trip through CredentialStore keyed by provider_name. Script functions are auth_start / auth_exchange / auth_needs_refresh / auth_refresh with the legacy build_authorization_request / exchange_code / needs_refresh / refresh_token kept as deprecated aliases to preserve PROV-060 script compatibility.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The NAPI layer exposes custom_oauth_authorize, custom_oauth_exchange, custom_oauth_needs_refresh, custom_oauth_refresh, and custom_oauth_clear functions keyed by provider_name
  #   2. custom_oauth_authorize reuses the existing callback_server.rs loopback server (random port, opens browser, awaits callback with code + state) and returns an authorization result containing code and pkce_verifier
  #   3. custom_oauth_* NAPI bindings dispatch to the script functions auth_start, auth_exchange, auth_needs_refresh, auth_refresh (new preferred names) with deprecated aliases build_authorization_request, exchange_code, needs_refresh, refresh_token still accepted
  #   4. When a Rhai script shadows a provider name, the TypeScript /login dispatcher routes to custom_oauth_* NAPI; when no shadow config exists, built-in claude_oauth / codex_oauth / copilot_oauth are used unchanged
  #   5. Refreshed tokens produced by auth_refresh are persisted back through CredentialStore under the provider_name so subsequent invocations see the latest tokens
  #
  # EXAMPLES:
  #   1. User runs /login my-custom and custom_oauth_authorize opens browser, receives callback, returns code+verifier; custom_oauth_exchange swaps code for tokens and stores them in CredentialStore
  #   2. Script defines auth_start / auth_exchange using the new preferred names and login completes successfully
  #   3. Script using the deprecated build_authorization_request/exchange_code aliases still logs the user in successfully
  #   4. After tokens expire, auth_needs_refresh returns true and auth_refresh silently updates the stored credentials without re-prompting the user
  #   5. Running /login claude without any shadow config still uses the built-in Claude OAuth implementation unchanged
  #   6. custom_oauth_clear my-custom removes the stored tokens so the next /login re-prompts
  #
  # ========================================
  Background: User Story
    As a user authoring a Rhai-scripted OAuth provider
    I want to invoke browser-loopback PKCE OAuth through NAPI bindings that dispatch to my script's auth_start / auth_exchange / auth_needs_refresh / auth_refresh functions
    So that custom providers can complete OAuth login via the standard /login flow without being limited to hard-coded Claude/Codex/Copilot implementations

  Scenario: User logs in with custom shadow provider using auth_start and auth_exchange
    Given a Rhai script my-custom.rhai defining auth_start and auth_exchange is registered with provider name "my-custom"
    When I invoke custom_oauth_authorize("my-custom") and then custom_oauth_exchange("my-custom", code, verifier) with the values returned from the loopback callback
    Then the script's auth_start is called to produce the authorization URL and pkce_verifier
    Then the script's auth_exchange is called with the returned code and verifier and produces tokens
    Then the resulting tokens are persisted in CredentialStore under provider_name "my-custom"

  Scenario: Legacy script using deprecated function aliases still authenticates successfully
    Given a Rhai script registered as "legacy-custom" defines only build_authorization_request and exchange_code (the PROV-060 names)
    When custom_oauth_authorize("legacy-custom") and custom_oauth_exchange("legacy-custom", code, verifier) are invoked
    Then the NAPI layer falls back to build_authorization_request when auth_start is not defined
    Then the NAPI layer falls back to exchange_code when auth_exchange is not defined
    Then the login completes and tokens are stored under provider_name "legacy-custom"

  Scenario: Expired tokens are refreshed silently via auth_needs_refresh and auth_refresh
    Given tokens for provider "my-custom" exist in CredentialStore but are expired
    When custom_oauth_needs_refresh("my-custom") is called
    Then the script's auth_needs_refresh is invoked and returns true
    When custom_oauth_refresh("my-custom") is subsequently called
    Then the script's auth_refresh is invoked with the current tokens and returns new tokens
    Then the refreshed tokens replace the stored credentials in CredentialStore under "my-custom"

  Scenario: Built-in providers are used unchanged when no shadow script is registered
    Given no Rhai shadow config exists for the provider name "claude"
    When the dispatcher resolves the OAuth implementation for "claude"
    Then it selects the built-in claude_oauth NAPI binding and not custom_oauth_authorize

  Scenario: custom_oauth_clear removes stored tokens for a provider
    Given tokens for provider "my-custom" are present in CredentialStore
    When custom_oauth_clear("my-custom") is called
    Then the stored tokens for "my-custom" are removed from CredentialStore
