# AST Research: Claude Headless Login Dependencies

## Research Date: 2026-03-02

## Objective
Identify reusable functions and types from PROV-020 (claude_oauth.rs) and PROV-021 (claude_auth.rs) for the headless login flow, plus the DeviceAuthConfig pattern from PROV-014 (codex_device_auth.rs).

---

## PROV-020: claude_oauth.rs — Functions to Reuse

### `build_authorize_url(pkce: &PkceCodes) -> String`
- **File**: codelet/providers/src/claude_oauth.rs:77-96
- Builds the authorize URL with PKCE params, state=verifier
- Used by headless login to generate the URL for the user to visit

### `parse_authorization_code(raw: &str) -> (String, Option<String>)`
- **File**: codelet/providers/src/claude_oauth.rs:107-112
- Splits code#state on `#` separator
- Used by headless login to parse the user-pasted code#state string

### `exchange_authorization_code(base_url, code, state, code_verifier) -> Result<ClaudeTokenResponse>`
- **File**: codelet/providers/src/claude_oauth.rs:124-145
- JSON POST to token endpoint with code, state, grant_type, client_id, redirect_uri, code_verifier
- Used by headless login after state validation

### `calculate_expiry(expires_in: u64) -> u64`
- **File**: codelet/providers/src/claude_oauth.rs:299-305
- Calculates ms timestamp from expires_in seconds
- Used to compute the `expires` field in ClaudeAuthJson

### `PkceCodes` (via codex_oauth)
- **Import**: `crate::codex::codex_oauth::{generate_pkce, PkceCodes}`
- Shared PKCE generation between Codex and Claude

---

## PROV-021: claude_auth.rs — Types and Persistence to Reuse

### `ClaudeAuthJson` struct
- **File**: codelet/providers/src/claude_auth.rs:18-24
- Fields: access_token, refresh_token, expires (u64 ms)
- Return type for headless login (identical to browser OAuth)

### `write_claude_auth(auth: &ClaudeAuthJson) -> Result<()>`
- **File**: codelet/providers/src/claude_auth.rs:56-67
- Writes to ~/.config/codelet/claude_auth.json (or CODELET_HOME)
- Reused for persistence in headless flow

---

## PROV-014: codex_device_auth.rs — Config Pattern to Mirror

### `DeviceAuthConfig` struct
- **File**: codelet/providers/src/codex/codex_device_auth.rs:41-55
- Pattern: issuer_url, timeout_ms, optional overrides, callback function
- **Adaptation**: ClaudeHeadlessLoginConfig will use:
  - `token_endpoint_base: String` (wiremock or production)
  - `timeout_ms: u64`
  - `pkce: Option<PkceCodes>` (inject for tests)
  - `code_entry_fn: Box<dyn FnOnce(String) -> Pin<Box<dyn Future<Output = Result<String>>>>>`

### `device_auth_login(config: DeviceAuthConfig) -> Result<CodexTokens>`
- **File**: codelet/providers/src/codex/codex_device_auth.rs:237-304
- Orchestrator pattern: request code → display → poll → exchange → persist → return
- **Adaptation**: claude_headless_login will be simpler: generate PKCE → invoke callback → validate → exchange → persist → return

---

## Module Registration

### lib.rs exports
- **File**: codelet/providers/src/lib.rs
- Line 41: `pub use claude_oauth_server::claude_browser_oauth_login;`
- Line 42: `pub use claude_auth::ClaudeAuthJson;`
- New module `claude_headless_login` must be added to lib.rs with `pub mod` and `pub use`

---

## Test Fixtures

### fixtures/mod.rs
- **File**: codelet/providers/tests/fixtures/mod.rs
- `setup_codelet_home()` — sets up temp CODELET_HOME for Claude auth tests
- `CodeletHomeGuard` — RAII guard for env var restoration
- No Claude-specific token response builder needed (Claude returns access_token, refresh_token, expires_in directly — no JWT/id_token)

---

## opencode Reference

### Anthropic Auth Plugin (opencode-anthropic-auth v0.0.13)
- **File**: /tmp/anthropic-auth-inspect/package/index.mjs
- Lines 106-136 (auth.ts): `authorize.method === "code"` → prompts user to paste code → calls `authorize.callback(code)`
- Lines 39-66 (index.mjs): `exchange(code, verifier)` → splits on `#`, JSON POST, returns {refresh, access, expires}
- This is exactly the headless pattern: display URL → user pastes code → exchange → done
