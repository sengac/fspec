# AST Research: Claude OAuth Core Patterns

## Research Date: 2026-03-02
## Work Unit: PROV-020

## 1. Codex OAuth Module Structure (codex_oauth.rs) — Reference for Claude OAuth

### Public Functions
```
codelet/providers/src/codex/codex_oauth.rs: generate_pkce() -> PkceCodes
codelet/providers/src/codex/codex_oauth.rs: generate_state() -> String
codelet/providers/src/codex/codex_oauth.rs: parse_jwt_claims(token: &str) -> Result<serde_json::Value>
codelet/providers/src/codex/codex_oauth.rs: extract_account_id_from_claims(claims: &serde_json::Value) -> Option<String>
codelet/providers/src/codex/codex_oauth.rs: extract_account_id(id_token: Option<&str>, access_token: Option<&str>) -> Option<String>
codelet/providers/src/codex/codex_oauth.rs: build_authorize_url(redirect_uri: &str, pkce: &PkceCodes, state: &str) -> String
codelet/providers/src/codex/codex_oauth.rs: validate_oauth_callback(callback_state: &str, expected_state: &str) -> Result<()>
codelet/providers/src/codex/codex_oauth.rs: rewrite_codex_url(url: &str) -> String
codelet/providers/src/codex/codex_oauth.rs: build_codex_headers(access_token: &str, account_id: &str) -> HashMap<String, String>
codelet/providers/src/codex/codex_oauth.rs: refresh_access_token(refresh_token: &str) -> Result<TokenRefreshResponse>
codelet/providers/src/codex/codex_oauth.rs: refresh_access_token_at(issuer_url: &str, refresh_token: &str) -> Result<TokenRefreshResponse>
codelet/providers/src/codex/codex_oauth.rs: exchange_authorization_code(issuer_url: &str, code: &str, code_verifier: &str, redirect_uri: Option<&str>) -> Result<TokenRefreshResponse>
```

### Structs
```
codelet/providers/src/codex/codex_oauth.rs:34: pub struct PkceCodes { verifier, challenge, challenge_method }
codelet/providers/src/codex/codex_oauth.rs:251: pub struct TokenRefreshResponse { id_token, access_token, refresh_token, expires_in }
codelet/providers/src/codex/codex_oauth.rs:366: pub struct OAuthTimeout { timeout_ms }
```

### Key Pattern: Token endpoint uses form-encoded POST
```rust
// post_to_token_endpoint uses .form(params) with Content-Type: application/x-www-form-urlencoded
// Claude OAuth MUST differ: use .json() with Content-Type: application/json
```

## 2. Claude Provider (claude.rs) — Existing OAuth infrastructure

### AuthMode Enum (already exists)
```rust
// claude.rs:139-145
pub enum AuthMode {
    ApiKey,
    OAuth,
}
```

### Existing OAuth Tests in claude.rs
```
codelet/providers/src/claude.rs:620: test_oauth_headers_are_set_correctly()
codelet/providers/src/claude.rs:658: test_oauth_url_includes_beta_query_param()
codelet/providers/src/claude.rs:681: test_api_key_url_does_not_include_beta_query_param()
```

## 3. OpenCode Reference (opencode-anthropic-auth npm package)

### Key findings from /tmp/package/index.mjs:
- CLIENT_ID: `9d1c250a-e61b-44d9-88ed-5944d1962f5e`
- Uses `@openauthjs/openauth/pkce` for PKCE generation
- Authorize URL: `https://claude.ai/oauth/authorize` (max mode) or `https://console.anthropic.com/oauth/authorize` (console mode)
- Redirect URI: `https://console.anthropic.com/oauth/code/callback`
- Scope: `org:create_api_key user:profile user:inference`
- State = PKCE verifier (not separate random value)
- Auth code format: `code#state` — split on `#`
- Token endpoint: JSON body POST to `https://console.anthropic.com/v1/oauth/token`
- Token refresh: JSON body with grant_type=refresh_token, refresh_token, client_id
- Required beta headers: `oauth-2025-04-20`, `interleaved-thinking-2025-05-14`
- User-Agent: `claude-cli/2.1.2 (external, cli)`
- x-api-key: explicitly removed
- Tool name prefix: `mcp_` added to tool.name in definitions and tool_use blocks
- Tool name strip: regex `/"name"\s*:\s*"mcp_([^"]+)"/g` → `"name": "$1"` in response stream
- URL rewriting: `/v1/messages` path gets `?beta=true` appended
- Expiry calculation: `Date.now() + json.expires_in * 1000`

## 4. Functions needed for claude_oauth.rs (NOT in codex_oauth.rs)

1. `parse_authorization_code(raw: &str) -> (String, Option<String>)` — NEW: split code#state
2. `build_authorize_url(pkce: &PkceCodes) -> String` — DIFFERENT: claude.ai base, code=true param, state=verifier
3. `build_oauth_headers(access_token, existing_beta)` — DIFFERENT: anthropic-beta merge, user-agent, remove x-api-key
4. `prefix_tool_name(name: &str) -> String` — NEW: mcp_ prefix
5. `strip_tool_name_prefix(name: &str) -> String` — NEW: strip mcp_ prefix  
6. `rewrite_claude_url(url: &str) -> String` — DIFFERENT: /v1/messages → ?beta=true
7. `calculate_expiry(expires_in: u64) -> u64` — NEW: ms calculation
8. `exchange_authorization_code(...)` — DIFFERENT: JSON body, different fields (includes state)
9. `refresh_access_token(...)` — DIFFERENT: JSON body, no id_token in response

### Can be reused from codex_oauth.rs:
- `generate_pkce()` — identical PKCE S256 logic
- `PkceCodes` struct and `from_verifier()` — identical
- `urlencoded()` helper — identical
