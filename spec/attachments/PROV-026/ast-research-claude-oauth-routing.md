# AST Research: Claude OAuth Routing & Model Availability

## Summary

PROV-026 wires up Claude OAuth token detection into 5 touch points:
1. `claude_auth.rs` — add sync reader
2. `credentials.rs` — add `has_claude_auth()`
3. `manager.rs` — route `get_claude()` through OAuth
4. `resolver.rs` — fallback credential resolution from claude_auth.json
5. `modelInitializationService.ts` — check Claude OAuth tokens for model selector

## 1. claude_auth.rs — Current Functions

File: `codelet/providers/src/claude_auth.rs`

| Function | Line | Signature | Sync/Async |
|----------|------|-----------|------------|
| `get_codelet_home()` | 29 | `fn get_codelet_home() -> PathBuf` | sync |
| `get_claude_auth_path()` | 39 | `pub fn get_claude_auth_path() -> PathBuf` | sync |
| `read_claude_auth()` | 44 | `pub async fn read_claude_auth() -> Result<Option<ClaudeAuthJson>>` | **async** |
| `write_claude_auth()` | 57 | `pub async fn write_claude_auth(auth: &ClaudeAuthJson) -> Result<()>` | **async** |

**Need to add:** `pub fn read_claude_auth_sync() -> Result<Option<ClaudeAuthJson>>` using `std::fs::read_to_string()`.

**Reference pattern** — Codex's `read_codex_auth()` at `codex/codex_auth.rs:114`:
```rust
pub fn read_codex_auth() -> Result<Option<CodexAuthJson>> {
    #[cfg(target_os = "macos")]
    { if let Some(auth) = read_keychain_credentials()? { return Ok(Some(auth)); } }
    read_file_credentials()
}
```
Claude version is simpler (no keychain).

## 2. credentials.rs — Current Detection Logic

File: `codelet/providers/src/credentials.rs`

| Field | Current Detection | Line |
|-------|-------------------|------|
| `claude_available` | `env::var("ANTHROPIC_API_KEY").is_ok() \|\| env::var("CLAUDE_CODE_OAUTH_TOKEN").is_ok()` | 21-22 |
| `codex_available` | `has_codex_auth()` | 24 |

**Need to change:** `claude_available` to: `env::var("ANTHROPIC_API_KEY").is_ok() || env::var("CLAUDE_CODE_OAUTH_TOKEN").is_ok() || has_claude_auth()`

**Reference pattern** — `has_codex_auth()` at line 90:
```rust
fn has_codex_auth() -> bool {
    use crate::codex::codex_auth::read_codex_auth;
    if let Ok(Some(auth)) = read_codex_auth() {
        if auth.openai_api_key.is_some() { return true; }
        if let Some(tokens) = auth.tokens {
            return !tokens.refresh_token.is_empty() && !tokens.account_id.is_empty();
        }
    }
    false
}
```
Claude version simpler: check access_token and refresh_token non-empty.

## 3. manager.rs — get_claude() Current Implementation

File: `codelet/providers/src/manager.rs:381`

```rust
pub fn get_claude(&self) -> Result<ClaudeProvider, ProviderError> {
    if self.current_provider == ProviderType::Claude {
        ClaudeProvider::new_with_model(self.selected_model_id().as_deref())
    } else {
        Err(ProviderError::config("manager", "Current provider is not Claude"))
    }
}
```

**Need to change:** Check `read_claude_auth_sync()` first, if OAuth tokens found use `ClaudeProvider::from_oauth_tokens()` with `Some(0)`, fall back to `new_with_model()`.

**Reference pattern** — `CodexProvider::new()` at `codex/mod.rs:100`:
```rust
pub fn new() -> Result<Self, ProviderError> {
    let auth = codex_auth::read_codex_auth()...;
    if let Some(auth_data) = &auth {
        if let Some(tokens) = &auth_data.tokens {
            if !tokens.access_token.is_empty() && !tokens.account_id.is_empty() {
                return Self::from_oauth_tokens(
                    &tokens.access_token, &tokens.refresh_token, &tokens.account_id,
                    Some(0), // Force immediate refresh
                    codex_oauth::CODEX_ISSUER, &model_name,
                );
            }
        }
    }
    // fallback...
}
```

**ClaudeProvider::from_oauth_tokens() signature** at `claude.rs:334`:
```rust
pub fn from_oauth_tokens(
    access_token: &str, refresh_token: &str,
    expires_in_secs: Option<u64>,
    token_endpoint_base: &str, // "https://console.anthropic.com"
    model: &str,
) -> Result<Self, ProviderError>
```

**Token endpoint constant:** `CLAUDE_TOKEN_ENDPOINT` at `claude_oauth.rs:36`:
`"https://console.anthropic.com/v1/oauth/token"` — but `from_oauth_tokens` takes `token_endpoint_base` (no `/v1/oauth/token`), so pass `"https://console.anthropic.com"`.

## 4. resolver.rs — Current Credential Resolution

File: `codelet/napi/src/credentials/resolver.rs`

**resolve_credential()** at line 116: Priority chain — credentials store → env vars → .env file.
No OAuth fallback currently exists.

**Need to add:** After step 3 (.env file), check `read_claude_auth_sync()` for `anthropic` provider. If found, return `access_token` and set `CLAUDE_CODE_OAUTH_TOKEN` env var.

**Provider env var mapping** at line 17: `"anthropic" => Some(vec!["ANTHROPIC_API_KEY", "CLAUDE_CODE_OAUTH_TOKEN"])` — already includes CLAUDE_CODE_OAUTH_TOKEN.

## 5. modelInitializationService.ts — Current Codex Pattern

File: `src/tui/services/modelInitializationService.ts`

**checkCodexOAuthTokens()** at line 147:
```typescript
function checkCodexOAuthTokens(): boolean {
    try { const tokens = codexOauthGetTokens(); return tokens !== null && tokens !== undefined; }
    catch { return false; }
}
```

**buildCloudSections()** at line 103:
- Line 107: `const hasCodexOAuth = checkCodexOAuthTokens();`
- Line 115-116: `hasCredentials = registryEntry?.requiresApiKey === false || !!providerConfig.apiKey;`
- Line 136-138: If `hasCodexOAuth`, extract codex models into synthetic section

**Need to add:**
1. `async function checkClaudeOAuthTokens(): Promise<boolean>` using `claudeOauthGetTokens()` (async, unlike Codex sync)
2. In `buildCloudSections()`: check `hasClaudeOAuth`, override `hasCredentials=true` for anthropic section when true

**Key difference from Codex:** Claude doesn't need a synthetic section — models already under `anthropic` provider in models.dev. Just need to force `hasCredentials=true`.

**NAPI binding available:** `claudeOauthGetTokens()` declared in `codelet/napi/index.d.ts:371`:
```typescript
export declare function claudeOauthGetTokens(): Promise<NapiClaudeTokens | null>;
```

**Import already present at line 19:** `codexOauthGetTokens` — need to add `claudeOauthGetTokens`.

## 6. Existing Test Pattern Reference

**Codex test:** `src/tui/services/__tests__/codexModelInitialization.test.ts`
- Mocks `codexOauthGetTokens` (line 32) as vi.fn()
- Mocks `modelsListAll` (line 30) for model data
- Tests synthetic section creation, model path building, persisted model restoration
- Uses `setupTestDirectory()` helper

**Rust tests:** `codelet/providers/tests/` contains:
- `claude_oauth_test.rs` (PROV-020)
- `claude_oauth_server_test.rs` (PROV-021)
- `claude_headless_login_test.rs` (PROV-022)
- `claude_refreshing_client_test.rs` (PROV-023)

## 7. opencode Reference

opencode uses a plugin architecture (`AnthropicAuthPlugin` in `@opencode-ai/anthropic-auth`):
- `auth.loader(getAuth, provider)` checks OAuth type → returns custom `fetch` wrapper
- Custom fetch: refreshes tokens if expired, sets Bearer auth, merges beta headers, prefixes tool names
- `auth.methods[]` provides "Claude Pro/Max" (OAuth) and "Create an API Key" options
- Tokens stored in opencode's `auth.json` as `{type: "oauth", refresh, access, expires}`

Key difference: opencode's approach is entirely JS-based with a plugin system. Our approach is Rust-native with NAPI bindings — the token refresh and request modification happen in `RefreshingClaudeClient` (Rust), not in a JS fetch wrapper.
