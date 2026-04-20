//! Tests for Shared OAuth Building Blocks (PROV-060)
//!
//! Feature: spec/features/shared-oauth-building-blocks.feature
//!
//! Tests map directly to Gherkin scenarios with @step comments.
//!
//! Scenario: Generic credential store reads and writes provider auth files
// @step Given a CredentialStore parameterized with a provider-specific auth type
// @step When credentials are written and then read back for Copilot, Codex, and Claude
// @step Then each provider's auth JSON file is correctly serialized and deserialized
// @step And the three separate read/write function pairs are replaced by the single generic implementation
//!
//! Scenario: Generic refreshing HTTP client unifies token refresh logic
// @step Given a RefreshingHttpClient parameterized with a TokenStrategy
// @step When a request is made with an expired token using CodexTokenStrategy
// @step Then the double-check locking pattern refreshes the token before sending
// @step And the same RefreshingHttpClient with ClaudeTokenStrategy exhibits identical refresh behavior
//!
//! Scenario: Generic device code flow unifies RFC 8628 polling loops
// @step Given a DeviceCodeFlow parameterized with a DeviceCodeProvider
// @step When a device code poll cycle is executed for CopilotDeviceCode
// @step Then the RFC 8628 polling loop handles slow_down, authorization_pending, and expiry correctly
// @step And the same DeviceCodeFlow with CodexDeviceCode uses identical polling logic
//!
//! Scenario: Generic OAuth callback server unifies PKCE flows
// @step Given an OAuthCallbackServer parameterized with a CodeExchangeHandler
// @step When a PKCE authorization code callback is received for Codex
// @step Then the server extracts the code and state, and exchanges for tokens via the handler
// @step And the same OAuthCallbackServer with a Claude handler supports multi-region via iss parameter
//!
//! Scenario: Sandboxed Rhai engine enforces safety limits
// @step Given a Rhai engine created via build_sandboxed_engine with registered modules
// @step When a script exceeds the operation limit of 50000 operations
// @step Then the engine terminates the script with an error
// @step And scripts cannot access the filesystem, spawn processes, or make unregistered network calls
//!
//! Scenario: Rhai building block modules provide OAuth primitives
// @step Given the http, crypto, json, and oauth modules are registered in the Rhai engine
// @step When a script calls oauth::generate_pkce and crypto::sha256 and json::parse and http::post
// @step Then each function returns the expected result type
// @step And the engine factory accepts an extensible module list for future modules
//!
//! Scenario: Scripted OAuth provider executes Rhai flow functions
// @step Given a ScriptedOAuthProvider loaded from a .rhai script defining all five OAuth functions
// @step When build_authorization_request is called
// @step Then it returns an authorization URL with PKCE challenge and state
// @step And exchange_code, refresh_token, poll_for_token, and needs_refresh each execute correctly via spawn_blocking
//!
//! Scenario: All existing provider tests pass after refactoring
// @step Given the shared OAuth building blocks have replaced provider-specific implementations
// @step When the full test suite is executed with cargo test in codelet/providers
// @step Then all unit tests pass with zero failures
// @step And all integration tests pass with zero failures
// @step And cargo clippy produces zero warnings across the entire workspace
//!
//! Scenario: All new shared module files comply with 300-line limit
// @step Given the new oauth/ module directory contains credential_store, http_middleware, device_flow, callback_server, token_refresh, engine, building_blocks, and script_provider
// @step When each file's line count is checked
// @step Then every file is under 300 lines

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod credential_store_tests;
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod rhai_engine_tests;
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod rhai_modules_tests;
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod scripted_provider_tests;
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod http_middleware_tests;
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod device_flow_tests;
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod callback_server_tests;
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod regression_tests;
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod file_size_tests;
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod cred_module_tests;
