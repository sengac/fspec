@done
@PROV-060
Feature: Shared OAuth Building Blocks + Rhai Scripting for Custom Providers
  """
  New shared module at codelet/providers/src/oauth/ with sub-modules: mod.rs, credential_store.rs, http_middleware.rs, device_flow.rs, callback_server.rs, token_refresh.rs, engine.rs, building_blocks.rs, script_provider.rs
  Existing providers refactored in-place to use generic building blocks: copilot/refreshing_client.rs, codex/refreshing_client.rs, claude_refreshing_client.rs, copilot/auth.rs, codex/codex_auth.rs, claude_auth.rs, copilot/oauth_device_code.rs, codex/codex_device_auth.rs, codex/codex_oauth_server.rs, claude_oauth_server.rs
  Cargo.toml for codelet/providers adds rhai = { version = "1.24", features = ["sync", "serde"] } and ureq = { version = "2", features = ["json", "tls"] }
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A generic CredentialStore<T> must handle read/write/resolve for any provider's auth JSON file
  #   2. A generic RefreshingHttpClient<S: TokenStrategy> must unify the double-check locking token refresh pattern used by Codex and Claude
  #   3. A generic DeviceCodeFlow<P: DeviceCodeProvider> must unify the RFC 8628 device authorization grant polling loops used by Copilot and Codex
  #   4. A generic OAuthCallbackServer<H: CodeExchangeHandler> must unify the local HTTP PKCE callback servers used by Codex and Claude
  #   5. All existing provider tests (unit + integration) must continue to pass after refactoring — zero regressions
  #   6. New shared oauth/ module files must each be under 300 lines
  #   7. A sandboxed Rhai engine (Engine::new_raw) with operation limits, call depth limits, and registered building block modules must be provided for scripted OAuth flows
  #   8. Rhai building block modules (http, crypto, json, oauth) must be registered so scripts can call http::post, crypto::sha256, json::parse, oauth::generate_pkce etc.
  #   9. A ScriptedOAuthProvider that loads .rhai files and executes build_authorization_request, exchange_code, refresh_token, poll_for_token, needs_refresh must be provided
  #   10. Rhai scripts run synchronously inside tokio::task::spawn_blocking; HTTP calls in scripts use ureq (sync), not reqwest (async)
  #   11. The engine factory must accept extensible module lists (build_sandboxed_engine(modules)) to support PROV-061 adding time::, env:: etc.
  #   12. All clippy warnings and errors across the entire workspace must be resolved, regardless of origin
  #
  # EXAMPLES:
  #   1. Copilot, Codex, and Claude credential read/write all go through CredentialStore<T> with provider-specific T types, eliminating 3 separate read/write function pairs
  #   2. RefreshingCodexClient and RefreshingClaudeClient are replaced by RefreshingHttpClient<CodexTokenStrategy> and RefreshingHttpClient<ClaudeTokenStrategy> sharing the same double-check locking logic
  #   3. Copilot and Codex device code polling loops are replaced by DeviceCodeFlow<CopilotDeviceCode> and DeviceCodeFlow<CodexDeviceCode> sharing the same RFC 8628 polling logic
  #   4. A .rhai script can define build_authorization_request, exchange_code, refresh_token, poll_for_token, needs_refresh and the sandboxed engine executes them via spawn_blocking
  #   5. All 103+ unit tests and 49+ integration tests in codelet/providers continue to pass after refactoring
  #
  # ========================================
  Background: User Story
    As a developer
    I want to use shared OAuth building blocks and a Rhai scripting layer for custom OAuth flows
    So that new OAuth providers can be added via scripts without recompiling, and existing providers share deduplicated code

  @credential-store
  Scenario: Generic credential store reads and writes provider auth files
    Given a CredentialStore parameterized with a provider-specific auth type
    When credentials are written and then read back for Copilot, Codex, and Claude
    Then each provider's auth JSON file is correctly serialized and deserialized
    And the three separate read/write function pairs are replaced by the single generic implementation

  @http-middleware
  Scenario: Generic refreshing HTTP client unifies token refresh logic
    Given a RefreshingHttpClient parameterized with a TokenStrategy
    When a request is made with an expired token using CodexTokenStrategy
    Then the double-check locking pattern refreshes the token before sending
    And the same RefreshingHttpClient with ClaudeTokenStrategy exhibits identical refresh behavior

  @device-flow
  Scenario: Generic device code flow unifies RFC 8628 polling loops
    Given a DeviceCodeFlow parameterized with a DeviceCodeProvider
    When a device code poll cycle is executed for CopilotDeviceCode
    Then the RFC 8628 polling loop handles slow_down, authorization_pending, and expiry correctly
    And the same DeviceCodeFlow with CodexDeviceCode uses identical polling logic

  @callback-server
  Scenario: Generic OAuth callback server unifies PKCE flows
    Given an OAuthCallbackServer parameterized with a CodeExchangeHandler
    When a PKCE authorization code callback is received for Codex
    Then the server extracts the code and state, and exchanges for tokens via the handler
    And the same OAuthCallbackServer with a Claude handler supports multi-region via iss parameter

  @rhai-engine
  Scenario: Sandboxed Rhai engine enforces safety limits
    Given a Rhai engine created via build_sandboxed_engine with registered modules
    When a script exceeds the operation limit of 50000 operations
    Then the engine terminates the script with an error
    And scripts cannot access the filesystem, spawn processes, or make unregistered network calls

  @rhai-modules
  Scenario: Rhai building block modules provide OAuth primitives
    Given the http, crypto, json, and oauth modules are registered in the Rhai engine
    When a script calls oauth::generate_pkce and crypto::sha256 and json::parse and http::post
    Then each function returns the expected result type
    And the engine factory accepts an extensible module list for future modules

  @scripted-provider
  Scenario: Scripted OAuth provider executes Rhai flow functions
    Given a ScriptedOAuthProvider loaded from a .rhai script defining all five OAuth functions
    When build_authorization_request is called
    Then it returns an authorization URL with PKCE challenge and state
    And exchange_code, refresh_token, poll_for_token, and needs_refresh each execute correctly via spawn_blocking

  @regression
  Scenario: All existing provider tests pass after refactoring
    Given the shared OAuth building blocks have replaced provider-specific implementations
    When the full test suite is executed with cargo test in codelet/providers
    Then all unit tests pass with zero failures
    And all integration tests pass with zero failures
    And cargo clippy produces zero warnings across the entire workspace

  @file-size
  Scenario: All new shared module files comply with 300-line limit
    Given the new oauth/ module directory contains credential_store, http_middleware, device_flow, callback_server, token_refresh, engine, building_blocks, and script_provider
    When each file's line count is checked
    Then every file is under 300 lines
