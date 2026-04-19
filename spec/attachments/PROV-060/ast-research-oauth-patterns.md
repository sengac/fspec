# AST Research: OAuth Pattern Inventory for PROV-060

## HttpClientExt Implementations (3)
- `codelet/providers/src/claude_refreshing_client.rs:211` — `impl rig::http_client::HttpClientExt for RefreshingClaudeClient`
- `codelet/providers/src/codex/refreshing_client.rs:235` — `impl rig::http_client::HttpClientExt for RefreshingCodexClient`
- `codelet/providers/src/copilot/refreshing_client.rs:134` — `impl rig::http_client::HttpClientExt for CopilotHttpClient`

## TokenMode Enums (2 nearly identical)
- `codelet/providers/src/codex/refreshing_client.rs:42` — `pub enum TokenMode { OAuth { token_state }, ApiKey }`
- `codelet/providers/src/claude_refreshing_client.rs:41` — `pub enum ClaudeTokenMode { OAuth { token_state }, ApiKey }`

## TokenState Structs (2 nearly identical)
- `codelet/providers/src/codex/refreshing_client.rs:31` — `pub struct TokenState { access_token, refresh_token, account_id, expires_at, issuer_url }`
- `codelet/providers/src/claude_refreshing_client.rs:31` — `pub struct ClaudeTokenState { access_token, refresh_token, token_endpoint_base, expires_at }`

## Credential Read Functions (3)
- `codelet/providers/src/copilot/auth.rs:172` — `pub async fn read_copilot_auth() -> Result<Option<CopilotAuthJson>>`
- `codelet/providers/src/copilot/auth.rs:189` — `pub fn read_copilot_auth_sync() -> Result<Option<CopilotAuthJson>>`
- `codelet/providers/src/codex/codex_auth.rs:114` — `pub fn read_codex_auth() -> Result<Option<CodexAuthJson>>`
- `codelet/providers/src/claude_auth.rs:46` — `pub async fn read_claude_auth() -> Result<Option<ClaudeAuthJson>>`
- `codelet/providers/src/claude_auth.rs:63` — `pub fn read_claude_auth_sync() -> Result<Option<ClaudeAuthJson>>`

## Credential Write Functions (3)
- `codelet/providers/src/copilot/auth.rs:207` — `pub async fn write_copilot_auth(auth: &CopilotAuthJson) -> Result<()>`
- `codelet/providers/src/codex/codex_auth.rs:126` — `pub fn write_codex_auth(auth: &CodexAuthJson) -> Result<()>`
- `codelet/providers/src/claude_auth.rs:76` — `pub async fn write_claude_auth(auth: &ClaudeAuthJson) -> Result<()>`

## Device Code Flows (2)
- `codelet/providers/src/copilot/oauth_device_code.rs:36` — `pub async fn request_device_code(host_url: &str) -> Result<CopilotDeviceCodeResponse>`
- `codelet/providers/src/copilot/oauth_polling.rs:38` — `pub async fn poll_device_token(config, device_code) -> ...`
- `codelet/providers/src/codex/codex_device_auth.rs:99` — `pub async fn request_device_code(issuer_url: &str) -> Result<DeviceCodeResponse>`
- `codelet/providers/src/codex/codex_device_auth.rs:131` — `pub async fn poll_device_token(config, device_code) -> ...`

## OAuth Callback Servers (2)
- `codelet/providers/src/codex/codex_oauth_server.rs` — Codex PKCE callback (389 lines)
- `codelet/providers/src/claude_oauth_server.rs` — Claude PKCE callback (467 lines)

## Credential Detection in credentials.rs (3 callers)
- `codelet/providers/src/credentials.rs:106` — `has_codex_auth()` calls `read_codex_auth()`
- `codelet/providers/src/credentials.rs:123` — `has_claude_auth()` calls `read_claude_auth_sync()`
- `codelet/providers/src/credentials.rs:137` — `has_github_copilot_auth()` calls `read_copilot_auth_sync()`

## Token Refresh Logic (shared pattern in 2 files)
- `codelet/providers/src/codex/refreshing_client.rs:121` — `async fn refresh_token_if_needed(token_state) -> ...`
- `codelet/providers/src/claude_refreshing_client.rs:120` — `async fn refresh_token_if_needed(token_state) -> ...`
Both use identical double-check locking: read-lock → check expiry → write-lock → refresh → persist
