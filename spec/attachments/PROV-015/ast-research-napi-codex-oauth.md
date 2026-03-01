# AST Research: NAPI Codex OAuth Bindings (PROV-015)

## Underlying Rust Functions to Wrap

### codex_oauth_server.rs (PROV-013)
- `browser_oauth_login() -> Result<CodexTokens>` — Production entry point
- `browser_oauth_login_inner(config: OAuthServerConfig) -> Result<CodexTokens>` — Testable inner

### codex_device_auth.rs (PROV-014)
- `request_device_code(issuer_url: &str) -> Result<DeviceCodeResponse>` — Phase 1: get user_code
- `poll_device_token(config: &PollConfig, device_code: &DeviceCodeResponse) -> Result<PollResult>` — Phase 2: poll
- `device_auth_login(config: DeviceAuthConfig) -> Result<CodexTokens>` — Full orchestrator

### codex_oauth.rs (PROV-011)
- `refresh_access_token(refresh_token: &str) -> Result<TokenRefreshResponse>` — Token refresh
- `exchange_authorization_code(issuer_url, code, code_verifier, redirect_uri) -> Result<TokenRefreshResponse>`
- `extract_account_id(id_token, access_token) -> Option<String>`

### codex_auth.rs
- `read_codex_auth() -> Result<Option<CodexAuthJson>>` — Reads auth.json (sync)
- `write_codex_auth(auth: &CodexAuthJson) -> Result<()>` — Writes auth.json

## Key Structs

### Source (codelet-providers)
- `CodexTokens { id_token, access_token, refresh_token, account_id }` — All String
- `DeviceCodeResponse { device_auth_id, user_code, interval }` — String, String, u64
- `TokenRefreshResponse { id_token, access_token, refresh_token, expires_in }`
- `CodexAuthJson { openai_api_key, tokens, last_refresh }`

### Target NAPI Structs (to create)
- `NapiCodexTokens` — #[napi(object)], maps 1:1 to CodexTokens
- `NapiDeviceAuthStartResult` — #[napi(object)], { user_code: String, verification_url: String }

## NAPI Module Pattern (from lib.rs)
```rust
#[cfg(not(feature = "noop"))]
mod codex_oauth;
#[cfg(not(feature = "noop"))]
pub use codex_oauth::*;
```

## Existing NAPI Function Patterns (from session_manager.rs, git.rs, etc.)
- Sync functions: `pub fn func_name(args) -> napi::Result<T>`
- Async functions: `pub async fn func_name(args) -> napi::Result<T>` with tokio
- Error conversion: `Error::from_reason(format!("..."))`
- Object structs: `#[napi(object)] pub struct NapiType { pub field: Type }`

## Test Patterns (from codelet/napi/tests/)
- Use `wiremock` for HTTP endpoint mocking
- Use `serial_test::serial` for tests that modify env vars
- Use `fixtures` module for shared helpers (build_test_jwt, setup_codex_home)
- Tests call underlying Rust functions directly (not NAPI FFI)
- Struct conversion tests verify serialization/deserialization
