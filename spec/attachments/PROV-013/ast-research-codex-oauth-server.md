# AST Research: Codex OAuth Server (PROV-013)

## Existing Public Functions in codex_oauth.rs

| Function | Signature | Used by Server? |
|----------|-----------|----------------|
| `generate_pkce()` | `-> PkceCodes` | ✅ Generate PKCE pair |
| `generate_state()` | `-> String` | ✅ Generate CSRF state |
| `build_authorize_url()` | `(redirect_uri, pkce, state) -> String` | ✅ Build browser URL |
| `validate_oauth_callback()` | `(callback_state, expected_state) -> Result<()>` | ✅ CSRF check |
| `extract_account_id()` | `(id_token, access_token) -> Option<String>` | ✅ Extract account_id from JWT |
| `parse_jwt_claims()` | `(token) -> Result<Value>` | Indirect via extract_account_id |
| `refresh_access_token()` | `async (refresh_token) -> Result<TokenRefreshResponse>` | ❌ Not needed for initial login |
| `html_error()` | `(error) -> String` | ✅ Error pages |
| `HTML_SUCCESS` | `const &str` | ✅ Success page |
| `rewrite_codex_url()` | `(url) -> String` | ❌ Not needed for server |
| `build_codex_headers()` | `(access_token, account_id) -> HashMap` | ❌ Not needed for server |

## Existing Functions in codex_auth.rs

| Function | Signature | Used by Server? |
|----------|-----------|----------------|
| `write_codex_auth()` | `(auth: &CodexAuthJson) -> Result<()>` | ✅ Persist tokens |
| `read_codex_auth()` | `-> Result<Option<CodexAuthJson>>` | ❌ Not needed |
| `get_auth_path()` | `-> PathBuf` | ❌ Not needed |

## Key Types

- `PkceCodes { verifier, challenge, challenge_method }` — from codex_oauth.rs
- `TokenRefreshResponse { id_token, access_token, refresh_token, expires_in }` — from codex_oauth.rs
- `CodexAuthJson { openai_api_key, tokens, last_refresh }` — from codex_auth.rs
- `CodexTokens { id_token, access_token, refresh_token, account_id }` — from codex_auth.rs
- `OAuthTimeout { timeout_ms }` — from codex_oauth.rs

## Constants

- `CODEX_CLIENT_ID = "app_EMoamEEZ73f0CkXaXp7hrann"`
- `CODEX_ISSUER = "https://auth.openai.com"`
- `OAUTH_PORT = 1455`
- `OAUTH_TIMEOUT_MS = 300000` (5 minutes)

## Missing Function (to add to codex_oauth.rs)

### `exchange_authorization_code()`

```rust
pub async fn exchange_authorization_code(
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<TokenRefreshResponse>
```

POST to `{CODEX_ISSUER}/oauth/token` with:
- `grant_type=authorization_code`
- `code={code}`
- `code_verifier={code_verifier}`
- `client_id={CODEX_CLIENT_ID}`
- `redirect_uri={redirect_uri}`

Returns `TokenRefreshResponse` (reuses existing struct).

## New Module: codex_oauth_server.rs

### Public API

```rust
pub async fn browser_oauth_login() -> Result<CodexTokens>
```

Single entry point. Orchestrates:
1. Bind hyper server to port 1455
2. Generate PKCE + state
3. Open browser to authorize URL via `open` crate
4. Await callback (5-min timeout)
5. Validate state
6. Exchange code for tokens
7. Extract account_id from JWT
8. Persist tokens via `write_codex_auth()`
9. Shut down server
10. Return `CodexTokens`

### Routes

- `GET /auth/callback?code=...&state=...` — Main OAuth callback
- `GET /cancel` — User cancellation
- Everything else → 404

### Dependencies to Add

- `hyper = { version = "1", features = ["server", "http1"] }`
- `hyper-util = { version = "0.1", features = ["tokio"] }`
- `open = "5"` (cross-platform browser opener)
