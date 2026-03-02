# PROV-020: Claude OAuth Core Flow — Continuation Context

## Status: SPECIFYING → Ready for TESTING

The Example Map and feature file are complete with 16 scenarios. Next steps:
1. Move to **testing** phase
2. Write Rust tests in `codelet/providers/src/codex/` (or a new `claude/` module)
3. Implement `claude_oauth.rs`
4. Wire into `lib.rs` module declarations

---

## What This Card Builds

A new Rust module `codelet/providers/src/claude_oauth.rs` that provides the **core OAuth primitives** for Anthropic Claude subscription authentication (Claude Pro/Max). This mirrors `codex_oauth.rs` but with Anthropic-specific endpoints, JSON body format, and tool name prefixing.

**NOT in scope** (handled by sibling cards):
- PROV-021: Browser callback server + CSRF (like `codex_oauth_server.rs`)
- PROV-022: Device auth flow (like `codex_device_auth.rs`)
- PROV-023: Token refresh client (like `refreshing_client.rs`)
- PROV-024: NAPI bindings (like `napi/src/codex_oauth.rs`)
- PROV-025: TUI provider settings
- PROV-026: Provider routing + model availability
- PROV-027: Parity + regression hardening

---

## Key Constants (from opencode-anthropic-auth npm package)

```rust
pub const CLAUDE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub const CLAUDE_AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
pub const CLAUDE_TOKEN_ENDPOINT: &str = "https://console.anthropic.com/v1/oauth/token";
pub const CLAUDE_REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";
pub const CLAUDE_SCOPE: &str = "org:create_api_key user:profile user:inference";
pub const CLAUDE_USER_AGENT: &str = "claude-cli/2.1.2 (external, cli)";
pub const REQUIRED_BETA_HEADERS: &[&str] = &["oauth-2025-04-20", "interleaved-thinking-2025-05-14"];
pub const TOOL_NAME_PREFIX: &str = "mcp_";
```

---

## Critical Differences from Codex OAuth (`codex_oauth.rs`)

| Aspect | Codex OAuth | Claude OAuth |
|--------|-------------|--------------|
| **Token endpoint body** | `application/x-www-form-urlencoded` (`.form()`) | `application/json` (`.json()`) |
| **Token response** | Has `id_token`, `access_token`, `refresh_token`, `expires_in` | Only `access_token`, `refresh_token`, `expires_in` (NO `id_token`) |
| **Account ID extraction** | Parse JWT from `id_token` / `access_token` | NOT needed — no JWT parsing |
| **Auth code format** | Standard `code` parameter from callback | `code#state` concatenated format (split on `#`) |
| **State parameter** | Separate random value from PKCE verifier | State = PKCE verifier (simplifies CSRF) |
| **Auth headers** | `Authorization: Bearer`, `ChatGPT-Account-Id`, `originator` | `Authorization: Bearer`, `anthropic-beta` (merged), `user-agent`, remove `x-api-key` |
| **URL rewriting** | `/v1/responses` → `chatgpt.com/backend-api/codex/responses` | `/v1/messages` → `/v1/messages?beta=true` |
| **Tool name prefix** | Not needed | `mcp_` prefix on all tool names |
| **Redirect URI** | `http://localhost:1455/callback` (local server) | `https://console.anthropic.com/oauth/code/callback` (Anthropic-hosted) |

---

## Functions to Implement

### Pure functions (no HTTP):
1. **`generate_pkce()`** — Same as Codex: 43-char verifier, S256 challenge. Can re-use or copy from `codex_oauth.rs`.
2. **`build_authorize_url(pkce: &PkceCodes)`** — Build `https://claude.ai/oauth/authorize?code=true&client_id=...&response_type=code&redirect_uri=...&scope=...&code_challenge=...&code_challenge_method=S256&state={verifier}`. Note: state = verifier.
3. **`parse_authorization_code(raw: &str) -> (String, Option<String>)`** — Split `code#state` on `#`. If no `#`, return code as-is with None state.
4. **`build_oauth_headers(access_token: &str, existing_beta_headers: Option<&str>) -> HashMap<String, String>`** — Build headers: `Authorization: Bearer {token}`, merge `anthropic-beta` (required + existing), set `user-agent`, remove `x-api-key`.
5. **`prefix_tool_name(name: &str) -> String`** — `"Bash"` → `"mcp_Bash"`
6. **`strip_tool_name_prefix(name: &str) -> String`** — `"mcp_Bash"` → `"Bash"` (only strips if prefix exists)
7. **`rewrite_claude_url(url: &str) -> String`** — If URL contains `/v1/messages`, append `?beta=true` (or `&beta=true` if query params exist). Non-messages URLs pass through unchanged.
8. **`calculate_expiry(expires_in: u64) -> u64`** — `now_ms + expires_in * 1000`

### Async HTTP functions:
9. **`exchange_authorization_code(code: &str, state: &str, code_verifier: &str) -> Result<ClaudeTokenResponse>`** — JSON POST to token endpoint.
10. **`refresh_access_token(refresh_token: &str) -> Result<ClaudeTokenResponse>`** — JSON POST to token endpoint with `grant_type=refresh_token`.

### Types:
```rust
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ClaudeTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}
```

---

## Token Exchange Request Body (JSON, NOT form-encoded)

```json
{
  "grant_type": "authorization_code",
  "code": "<extracted_code>",
  "state": "<extracted_state>",
  "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
  "redirect_uri": "https://console.anthropic.com/oauth/code/callback",
  "code_verifier": "<pkce_verifier>"
}
```

## Token Refresh Request Body (JSON)

```json
{
  "grant_type": "refresh_token",
  "refresh_token": "<refresh_token>",
  "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
}
```

---

## Authorize URL Example (Max mode)

```
https://claude.ai/oauth/authorize?code=true&client_id=9d1c250a-e61b-44d9-88ed-5944d1962f5e&response_type=code&redirect_uri=https%3A%2F%2Fconsole.anthropic.com%2Foauth%2Fcode%2Fcallback&scope=org%3Acreate_api_key+user%3Aprofile+user%3Ainference&code_challenge=<base64url_sha256>&code_challenge_method=S256&state=<pkce_verifier>
```

---

## File Layout

### New file:
- `codelet/providers/src/claude_oauth.rs` — Core OAuth module

### Modified files:
- `codelet/providers/src/lib.rs` — Add `pub mod claude_oauth;` declaration

### Existing reference files (READ ONLY for pattern):
- `codelet/providers/src/codex/codex_oauth.rs` — PKCE, authorize URL, token exchange (form-encoded), headers, URL rewriting
- `codelet/providers/src/codex/refreshing_client.rs` — Token state management, HTTP middleware
- `codelet/providers/src/codex/codex_oauth_server.rs` — Browser callback server (NOT in scope)
- `codelet/providers/src/codex/codex_device_auth.rs` — Device auth flow (NOT in scope)
- `codelet/napi/src/codex_oauth.rs` — NAPI bindings (NOT in scope, see PROV-024)
- `codelet/providers/src/claude.rs` — Existing Claude provider with `AuthMode::OAuth` variant already defined (line 140-145)

### Existing types already defined:
- `AuthMode` enum in `claude.rs` (line 140): `ApiKey` and `OAuth` variants already exist
- `beta_headers` module in `claude.rs` (line 52-60): Some beta header constants already defined

---

## Crate Dependencies (all already in Cargo.toml)

- `sha2` — SHA-256 for PKCE challenge
- `base64` — Base64URL encoding
- `rand` — Random string generation
- `reqwest` — HTTP client for token endpoint
- `serde`, `serde_json` — JSON serialization
- `anyhow` — Error handling

---

## Feature File

Located at: `spec/features/claude-oauth-core.feature`

Contains 16 scenarios covering:
1. PKCE code verifier meets RFC 7636 requirements
2. PKCE challenge is deterministic for a given verifier
3. Authorize URL contains all required parameters for Max mode
4. Authorization code in code-hash-state format is parsed correctly
5. Authorization code without hash separator is used as-is
6. Authorization code exchanged for tokens at token endpoint
7. Code exchange fails with invalid authorization code
8. Token refresh using refresh_token grant
9. OAuth headers built with required beta headers
10. OAuth headers preserve existing beta headers
11. Tool names prefixed with mcp_ in OAuth mode
12. Tool names stripped of mcp_ prefix from response
13. Messages URL rewritten with beta query parameter
14. Messages URL with existing query parameters gets beta appended
15. Non-messages URL is not rewritten
16. Token expiry calculated from expires_in seconds

---

## Test Strategy

Tests for pure functions can use standard `#[test]`. Tests for async HTTP functions (`exchange_authorization_code`, `refresh_access_token`) should use `wiremock` (already in `[dev-dependencies]`) to mock the token endpoint, same pattern as Codex OAuth tests.

### Test file location:
- Unit tests: inline `#[cfg(test)] mod tests` in `claude_oauth.rs` (matching `codex_oauth.rs` pattern)
- Integration tests: would go in `codelet/providers/tests/` if needed

### Running tests:
```bash
cd codelet/providers
cargo test claude_oauth
```

---

## Integration Points (WHO CALLS THIS?)

This module is a **standalone library** of pure functions + async HTTP calls. It does NOT need integration wiring in this card. The downstream consumers are:

1. **PROV-021** will call `build_authorize_url()`, `parse_authorization_code()`, `exchange_authorization_code()`
2. **PROV-023** will call `refresh_access_token()` from a `RefreshingClaudeClient`
3. **PROV-024** will expose functions via NAPI bindings
4. **PROV-026** will use `build_oauth_headers()`, `prefix_tool_name()`, `strip_tool_name_prefix()`, `rewrite_claude_url()` in the Claude provider's HTTP layer

The ONLY wiring needed in THIS card is adding `pub mod claude_oauth;` to `lib.rs`.

---

## Estimate: 5 story points

- Multiple pure functions (trivial individually)
- Two async HTTP functions requiring wiremock tests
- Type definitions
- Module wiring
- Moderate complexity, clear patterns from codex_oauth.rs to follow
