# AST Research: Device Auth Flow Dependencies

## Async functions available in codex module

- `codex_oauth.rs:288` - `pub async fn refresh_access_token(refresh_token: &str) -> Result<TokenRefreshResponse>`
- `codex_oauth.rs:316` - `pub async fn exchange_authorization_code(issuer_url, code, code_verifier, redirect_uri) -> Result<TokenRefreshResponse>`
- `codex_oauth_server.rs:77` - `pub async fn browser_oauth_login() -> Result<CodexTokens>`
- `codex_oauth_server.rs:102` - `pub async fn browser_oauth_login_inner(config: OAuthServerConfig) -> Result<CodexTokens>`

## Key structs

- `codex_auth.rs:22` - `CodexAuthJson { openai_api_key, tokens, last_refresh }`
- `codex_auth.rs:34` - `CodexTokens { id_token, access_token, refresh_token, account_id }`
- `codex_oauth.rs:34` - `PkceCodes { verifier, challenge, challenge_method }`
- `codex_oauth.rs:246` - `TokenRefreshResponse { id_token, access_token, refresh_token, expires_in }`
- `codex_oauth_server.rs:55` - `OAuthServerConfig { issuer_url, listener, open_browser, timeout_ms, pkce, state }`

## Key functions to reuse

- `codex_oauth.rs` - `extract_account_id(id_token, access_token) -> Option<String>` - extracts account_id from JWT claims
- `codex_auth.rs` - `write_codex_auth(auth: &CodexAuthJson) -> Result<()>` - persists to auth.json
- `codex_oauth.rs:258` - `async fn post_to_token_endpoint(...)` - PRIVATE, cannot reuse; need pub(crate) or new function

## Test patterns (from codex_oauth_server_test.rs)

- Uses `wiremock::MockServer` for HTTP endpoint simulation
- Uses `fixtures::build_test_jwt()` for test JWTs
- Uses `fixtures::build_token_response_json()` for token responses
- Uses `fixtures::setup_codex_home()` for isolated CODEX_HOME
- Uses `serial_test::serial` for test isolation
- Uses `ephemeral_listener()` pattern for port-0 binding

## Device auth specific needs

- New file: `codex_device_auth.rs`
- New struct: `DeviceAuthConfig` (analogous to `OAuthServerConfig`)
- New functions: `request_device_code()`, `poll_device_token()`, `device_auth_login()`
- Token exchange without redirect_uri: need `post_to_token_endpoint` to be `pub(crate)` or new `exchange_device_code()`
- CODEX_CLIENT_ID and CODEX_ISSUER constants reused from codex_oauth.rs
