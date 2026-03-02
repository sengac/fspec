# AST Research: Claude Refreshing Client (PROV-023)

## Research Context

PROV-023 implements `RefreshingClaudeClient` - an HTTP middleware for Anthropic OAuth token management. This mirrors the existing Codex `RefreshingCodexClient` (PROV-016) but is **simpler** because Claude doesn't need URL rewriting or extra headers like `ChatGPT-Account-Id`.

## Key Structures Identified

### 1. Codex RefreshingClient (Template Pattern - codex/refreshing_client.rs)

```
RefreshingCodexClient {
    inner: reqwest::Client,
    mode: TokenMode { OAuth { token_state: Arc<RwLock<TokenState>> }, ApiKey },
}

TokenState {
    access_token: String,
    refresh_token: String,
    account_id: String,    // NOT needed for Claude
    expires_at: Instant,
    issuer_url: String,    // → token_endpoint_base for Claude
}
```

Key methods:
- `new_oauth(access_token, refresh_token, account_id, expires_in_secs, issuer_url)` → constructor
- `new_api_key()` → pass-through mode
- `is_token_expired()` → check with 30s buffer
- `ensure_fresh_token()` → double-check locking refresh
- `prepare_oauth_request()` → strips Auth header, injects Bearer + Codex headers + URL rewrite

Implements `rig::http_client::HttpClientExt` with:
- `send()` → T: Into<bytes::Bytes>
- `send_multipart()` → MultipartForm  
- `send_streaming()` → T: Into<bytes::Bytes>

### 2. Claude OAuth Functions (claude_oauth.rs - PROV-020)

```
refresh_access_token_at(base_url, refresh_token) → Result<ClaudeTokenResponse>
calculate_expiry(expires_in: u64) → u64 (ms since epoch)
```

ClaudeTokenResponse: `{ access_token, refresh_token, expires_in: u64 }`

Note: `expires_in` is NOT Optional for Claude (unlike Codex's `Option<u64>`)

### 3. Claude Auth Persistence (claude_auth.rs - PROV-021)

```
write_claude_auth(auth: &ClaudeAuthJson) → Result<()>  // ASYNC (tokio::fs)
read_claude_auth() → Result<Option<ClaudeAuthJson>>

ClaudeAuthJson { access_token, refresh_token, expires: u64 (ms) }
```

Key difference from Codex: `write_claude_auth` is **async** (uses tokio::fs), while `write_codex_auth` is synchronous. This means token persistence needs `tokio::spawn` instead of `std::thread::spawn`.

### 4. ClaudeProvider (claude.rs)

Current struct (needs modification):
```
ClaudeProvider {
    completion_model: anthropic::completion::CompletionModel,  // → CompletionModel<RefreshingClaudeClient>
    rig_client: anthropic::Client,                              // → Client<RefreshingClaudeClient>
    auth_mode: AuthMode,
    model_name: String,
}
```

Methods that need updates:
- `from_api_key_with_mode_and_model()` → must create RefreshingClaudeClient and pass via `.http_client()`
- `client()` → return type changes to `&anthropic::Client<RefreshingClaudeClient>`
- `create_rig_agent()` → return type becomes `Agent<CompletionModel<RefreshingClaudeClient>>`

### 5. Claude vs Codex Differences

| Aspect | Codex | Claude |
|--------|-------|--------|
| URL rewriting | Yes (/v1/responses, /chat/completions → chatgpt.com) | No (rig's build_uri handles ?beta=true) |
| Extra headers | ChatGPT-Account-Id, originator | None |
| Auth header | Bearer {access_token} | Bearer {access_token} |
| Token endpoint | form-encoded | JSON POST |
| expires_in | Option<u64> (defaults to 3600) | u64 (always present) |
| Persistence | Synchronous write_codex_auth | Async write_claude_auth (tokio::fs) |
| Persistence spawn | std::thread::spawn (sync) | tokio::spawn (async) |
| account_id | Yes (from JWT id_token) | No |

### 6. OpenCode Anthropic Plugin Pattern (index.mjs)

OpenCode's AnthropicAuthPlugin uses a custom `fetch` function:
```javascript
async fetch(input, init) {
    // 1. Check if token expired: auth.expires < Date.now()
    // 2. If expired, refresh via POST to console.anthropic.com/v1/oauth/token
    // 3. Persist refreshed tokens via client.auth.set()
    // 4. Set Authorization: Bearer, anthropic-beta (merge required + existing)
    // 5. Set user-agent, delete x-api-key
    // 6. Prefix tool names with mcp_ in request body
    // 7. Append ?beta=true to /v1/messages URLs
    // 8. Strip mcp_ prefix from tool names in streaming response
    // Return modified response
}
```

Our RefreshingClaudeClient is SIMPLER because:
- Tool name prefixing handled at a higher layer (not in HTTP middleware)
- URL rewriting done by rig's build_uri (AnthropicKey::is_oauth_token detection)
- Only handles: Bearer header injection + token refresh

## Test Strategy

Mirror the Codex test file (`codex_refreshing_client_test.rs`) pattern:
- Use wiremock for token endpoint mocking
- Use MockServer as backend to capture outgoing requests
- Test OAuth mode and ApiKey mode separately
- Test token refresh, expiry buffer, error propagation
- Test streaming path (send_streaming)
- Test persistence (write_claude_auth)
- Test Some(0) disk-loaded tokens
