# AST Research: Claude OAuth Server Patterns (PROV-021)

## Research Goal
Identify the Codex OAuth server patterns (PROV-013) to mirror for Claude's browser OAuth flow,
and the PROV-020 Claude OAuth core functions to reuse.

## 1. Codex OAuth Server Architecture (codex_oauth_server.rs)

### Public entry points
```
codex_oauth_server.rs:77:1: pub async fn browser_oauth_login() -> Result<CodexTokens>
codex_oauth_server.rs:102:1: pub async fn browser_oauth_login_inner(config: OAuthServerConfig) -> Result<CodexTokens>
```

### Config struct
```
codex_oauth_server.rs:55:1: pub struct OAuthServerConfig {
    issuer_url: String,
    listener: TcpListener,
    open_browser: bool,
    timeout_ms: u64,
    pkce: Option<PkceCodes>,
    state: Option<String>,
}
```

### Callback result enum
```
codex_oauth_server.rs:43:1: enum CallbackResult {
    Success { code, _state },
    Cancelled,
    AuthError { error, description },
    CsrfError { expected, received },
}
```

### Routes
- `/auth/callback` — receives OAuth redirect with `?code=&state=` query params
- `/cancel` — user aborts
- `_` (404) — does not shut down server

### Key pattern: serve_until_done loop
- Shared `oneshot::Sender<CallbackResult>` behind `Arc<Mutex<Option<>>>`
- `Arc<Notify>` for signaling server shutdown
- Terminal routes (callback, cancel) send through channel then notify done
- Non-terminal routes (404) respond but don't notify

### Token exchange flow (after callback)
1. State validated at HTTP layer (in handle_request)
2. exchange_authorization_code() called with issuer URL
3. extract_account_id() from JWT
4. write_codex_auth() persists tokens
5. Returns CodexTokens

## 2. Claude OAuth Core Functions (claude_oauth.rs — PROV-020 DONE)

### Available functions to reuse (NO duplication):
```
claude_oauth.rs:77:1:  pub fn build_authorize_url(pkce: &PkceCodes) -> String
claude_oauth.rs:107:1: pub fn parse_authorization_code(raw: &str) -> (String, Option<String>)
claude_oauth.rs:124:1: pub async fn exchange_authorization_code(base_url, code, state, code_verifier) -> Result<ClaudeTokenResponse>
claude_oauth.rs:154:1: pub async fn refresh_access_token_at(base_url, refresh_token) -> Result<ClaudeTokenResponse>
claude_oauth.rs:299:1: pub fn calculate_expiry(expires_in: u64) -> u64
```

### ClaudeTokenResponse struct
```rust
pub struct ClaudeTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}
```

### Key differences from Codex:
- No `id_token` in response — just access_token, refresh_token, expires_in
- No `account_id` extraction needed
- Token exchange uses JSON POST (not form-encoded)
- State = PKCE verifier (not separate)
- Redirect URI is remote (console.anthropic.com), not localhost

## 3. Codex Auth Persistence Pattern (codex_auth.rs)

```
codex_auth.rs:22:1: pub struct CodexAuthJson { openai_api_key, tokens, last_refresh }
codex_auth.rs:34:1: pub struct CodexTokens { id_token, access_token, refresh_token, account_id }
```

### Functions
- `get_auth_path()` → `~/.codex/auth.json`
- `read_codex_auth()` → reads from keychain (macOS) or file
- `write_codex_auth(auth)` → writes JSON to file

### Claude equivalent (new claude_auth.rs):
- `ClaudeAuthJson { access_token, refresh_token, expires }`
- Path: `~/.fspec/credentials/claude_auth.json`
- `write_claude_auth()` / `read_claude_auth()`
- No keychain support needed initially
- No `id_token` or `account_id` — simpler than Codex

## 4. HTML Templates (reuse from codex_oauth.rs)

```
codex_oauth.rs:395: pub const HTML_SUCCESS
codex_oauth.rs:416: pub const HTML_CANCELLED
codex_oauth.rs:436: pub fn html_error(error)
```

Claude server will define its own HTML templates (Claude-branded) but follow the same pattern.
Additionally needs a form page HTML template for code paste (new, unique to Claude flow).

## 5. Key Architectural Differences (Claude vs Codex Server)

| Aspect | Codex (PROV-013) | Claude (PROV-021) |
|--------|------------------|-------------------|
| Callback mechanism | Browser redirect to localhost | User pastes code from remote callback |
| Routes | GET /auth/callback, GET /cancel | GET / (form), POST /submit, GET /cancel |
| Port | Fixed 1455 | Ephemeral (port 0) |
| State validation | At HTTP layer from query params | Parse code#state from form input |
| Token response | id_token + access_token + refresh_token | access_token + refresh_token + expires_in |
| Persistence | ~/.codex/auth.json | ~/.fspec/credentials/claude_auth.json |
| Account ID | Extracted from JWT | Not needed |

## 6. Test Infrastructure (from codex_oauth_server_test.rs)

### Shared fixtures (tests/fixtures/mod.rs):
- `build_test_jwt(account_id)` — not needed for Claude
- `build_token_response_json()` — needs Claude variant (no id_token)
- `setup_codex_home()` — needs Claude equivalent (FSPEC_HOME)

### Test patterns to replicate:
- `ephemeral_listener()` — bind to port 0
- wiremock for token endpoint
- `#[serial]` for env var isolation
- HTTP client to hit server routes
- spawned tokio task for server + sleep for startup
- Timeout test with 100ms instead of 5 min
