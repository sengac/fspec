# AST Research: Claude OAuth NAPI Bindings (PROV-024)

## Purpose
Analyze the Rust public API surface that the NAPI bindings must expose to TypeScript.
Mirrors PROV-015 (Codex OAuth NAPI) pattern but adapted for Claude OAuth.

## Key Differences from Codex NAPI (PROV-015)
- No id_token, no account_id, no JWT extraction — simpler token struct
- claude_auth uses async I/O (tokio::fs) → get_tokens and clear_tokens must be async NAPI
- No device polling — headless uses start+complete instead of start+poll
- Token exchange uses JSON POST (not form-encoded)

---

## claude_auth.rs — Persistence Layer (async)

### Struct: ClaudeAuthJson
```
pub struct ClaudeAuthJson {
    pub access_token: String,
    pub refresh_token: String,
    pub expires: u64,     // milliseconds since epoch
}
```

### Functions:
- `pub fn get_claude_auth_path() -> PathBuf` — sync, returns path to claude_auth.json
- `pub async fn read_claude_auth() -> Result<Option<ClaudeAuthJson>>` — async (tokio::fs)
- `pub async fn write_claude_auth(auth: &ClaudeAuthJson) -> Result<()>` — async (tokio::fs)

**NAPI implication**: get_tokens and clear_tokens MUST be async NAPI functions (unlike Codex which is sync).

---

## claude_oauth.rs — Core OAuth Primitives

### Struct: ClaudeTokenResponse
```
pub struct ClaudeTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}
```

### Functions needed by NAPI:
- `pub fn build_authorize_url(pkce: &PkceCodes) -> String` — for headless start
- `pub fn parse_authorization_code(raw: &str) -> (String, Option<String>)` — for headless complete
- `pub async fn exchange_authorization_code(base_url, code, state, code_verifier) -> Result<ClaudeTokenResponse>` — for headless complete
- `pub async fn refresh_access_token_at(base_url, refresh_token) -> Result<ClaudeTokenResponse>` — for token refresh
- `pub fn calculate_expiry(expires_in: u64) -> u64` — for converting expires_in to timestamp

### Constants:
- `CLAUDE_TOKEN_ENDPOINT: &str = "https://console.anthropic.com/v1/oauth/token"`

---

## claude_oauth_server.rs — Browser Login Server

### Functions:
- `pub async fn claude_browser_oauth_login() -> Result<ClaudeAuthJson>` — production entry point
- `pub async fn claude_browser_oauth_login_inner(config: ClaudeOAuthServerConfig) -> Result<ClaudeAuthJson>` — testable inner

**NAPI**: browser_login wraps `claude_browser_oauth_login()` directly.

---

## oauth_crypto.rs — Shared PKCE

### Functions:
- `pub fn generate_pkce() -> PkceCodes` — generates verifier + S256 challenge
- Struct `PkceCodes { verifier, challenge, challenge_method }`

---

## NAPI Module Design

### NapiClaudeTokens (#[napi(object)])
Maps to ClaudeAuthJson: access_token (String), refresh_token (String), expires (f64)

### NapiClaudeHeadlessStartResult (#[napi(object)])
authorize_url (String), pkce_verifier (String)

### NAPI Functions:
1. `claude_oauth_browser_login()` → async → Promise<NapiClaudeTokens>
2. `claude_oauth_headless_start()` → sync → NapiClaudeHeadlessStartResult
3. `claude_oauth_headless_complete(code_with_state, pkce_verifier)` → async → Promise<NapiClaudeTokens>
4. `claude_oauth_refresh_token(refresh_token)` → async → Promise<NapiClaudeTokens>
5. `claude_oauth_get_tokens()` → async → Promise<NapiClaudeTokens | null>
6. `claude_oauth_clear_tokens()` → async → Promise<void>

### Reference Implementation: codelet/napi/src/codex_oauth.rs (PROV-015, done)
