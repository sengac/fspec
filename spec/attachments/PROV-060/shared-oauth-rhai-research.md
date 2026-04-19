# Shared OAuth Architecture + Rhai Scripting: Research & Recommendations

**Date:** 2026-04-17
**Session:** 359b5d06-a6b4-41d1-a487-1dfdf37713e4
**Research agents:** b5933cbe (OAuth analysis), e538ad60 (Rhai analysis)

---

## Executive Summary

This document captures comprehensive research into extracting shared OAuth building blocks across the Copilot, Codex, and Claude providers, and integrating Rhai (a Rust scripting language) to enable user-defined custom OAuth providers. Three architecture options are evaluated; the **hybrid approach (Option C)** is recommended.

---

## 1. Current State Analysis

### 1.1 Provider Inventory

Three OAuth-enabled providers exist with substantial code duplication:

| Provider | Auth Files | Lines (Total) | OAuth Flow |
|----------|-----------|---------------|------------|
| **Copilot** | `copilot/*.rs` | 1,611 | GitHub device code → Copilot token exchange |
| **Codex** | `codex/*.rs` | 2,494 | Device code + PKCE callback server |
| **Claude** | `claude*.rs` | 2,194 | PKCE callback server + headless login |
| **Shared** | `oauth_*.rs`, `credentials.rs` | 322 | PKCE crypto, HTTP utils, credential detect |
| **Total** | | **6,621** | |

### 1.2 Shared Modules Already Extracted

Three shared modules already exist, proving the pattern works:

**`oauth_crypto.rs`** (78 lines) — Provider-agnostic PKCE (RFC 7636):
- `PkceCodes` struct with verifier + S256 challenge
- `generate_pkce()` — random verifier + challenge generation
- `urlencoded()` — query parameter encoding
- Used by both Codex and Claude OAuth flows

**`oauth_http_utils.rs`** (101 lines) — Shared HTTP utilities:
- `html_response()` — builds HTML responses for OAuth callback servers
- `parse_urlencoded_params()` — parses URL-encoded form bodies
- `urlencoded_decode()` — percent-decoding with `+` as space
- Used by both `codex_oauth_server.rs` and `claude_oauth_server.rs`

**`credentials.rs`** (143 lines) — Unified credential detection:
- `ProviderCredentials` struct with per-provider availability flags
- `detect()` checks env vars + auth files for all 6 providers
- `has_codex_auth()`, `has_claude_auth()`, `has_github_copilot_auth()` — per-provider file checks

### 1.3 Duplicated Patterns (Not Yet Unified)

| Pattern | Copilot | Codex | Claude | Est. Duplicated Lines |
|---------|---------|-------|--------|-----------------------|
| `HttpClientExt` middleware boilerplate | ✅ | ✅ | ✅ | ~300 |
| Credential file I/O (read/write/detect) | ✅ | ✅ | ✅ | ~150 |
| Device code polling loop | ✅ | ✅ | — | ~100 |
| Double-check locking token refresh | — | ✅ | ✅ | ~60 |
| Token mode enum (`OAuth`/`ApiKey`) | — | ✅ | ✅ | ~40 |
| OAuth callback server (local HTTP) | — | ✅ | ✅ | ~100 |
| **Total estimated duplication** | | | | **~750 lines** |


### 1.4 Detailed Pattern Comparison

#### HttpClientExt Middleware (3 implementations)

All three providers implement `rig::http_client::HttpClientExt` as a middleware wrapper:

**Copilot** (`copilot/refreshing_client.rs`, 218 lines):
- `CopilotHttpClient` — thin middleware, no token refresh needed
- Classifies requests via `CopilotRequestClassifier`
- Builds headers via `CopilotHeaderFacade`
- Strips stale `Authorization` header, applies fresh Copilot headers
- No `RwLock` state — access token never expires (`expires: 0`)

**Codex** (`codex/refreshing_client.rs`, 319 lines):
- `RefreshingCodexClient` — full refresh middleware
- `TokenMode::OAuth { token_state: Arc<RwLock<TokenState>> }` / `TokenMode::ApiKey`
- Double-check locking: read-lock → check expiry → write-lock → refresh → persist
- Rewrites URLs from OpenAI endpoints to Codex API endpoint
- Sets `Authorization`, `ChatGPT-Account-Id`, `originator` headers
- 30-second expiry buffer (`EXPIRY_BUFFER_SECS`)

**Claude** (`claude_refreshing_client.rs`, 295 lines):
- `RefreshingClaudeClient` — refresh middleware without URL rewriting
- `ClaudeTokenMode::OAuth { token_state: Arc<RwLock<ClaudeTokenState>> }` / `ClaudeTokenMode::ApiKey`
- Double-check locking: identical pattern to Codex
- Only handles `Authorization: Bearer` — no URL rewriting, no extra headers
- Same 30-second expiry buffer

**Commonality:** Codex and Claude share ~90% identical refresh logic. A generic `RefreshingHttpClient<S: TokenStrategy>` could unify them, with Copilot using a simpler non-refreshing variant.

#### Device Code Flows (2 implementations)

**Copilot** (`copilot/oauth_device_code.rs` + `copilot/oauth_polling.rs`):
- GitHub device code grant: POST to `https://github.com/login/device/code`
- Poll `https://github.com/login/oauth/access_token` with `device_code`
- Then exchange GitHub token for Copilot token via `api.github.com/copilot_internal/v2/token`
- Two-step: GitHub OAuth → Copilot token exchange

**Codex** (`codex/codex_device_auth.rs`, 308 lines):
- OpenAI device code grant: POST to `{issuer_url}/v1/device/authorize`
- Poll `{issuer_url}/v1/device/token` with `device_code`
- Single-step: device code → access token directly

**Commonality:** Both follow RFC 8628 device authorization grant. The polling loop (slow_down, authorization_pending, expiry check) is nearly identical. Could be unified into a generic `DeviceCodeFlow<P: DeviceCodeProvider>`.

#### OAuth Callback Servers (2 implementations)

**Codex** (`codex/codex_oauth_server.rs`, 389 lines):
- Local HTTP server on random port for PKCE authorization code flow
- Receives `?code=...&state=...` callback
- Exchanges code for tokens at `{issuer_url}/v1/device/token`

**Claude** (`claude_oauth_server.rs`, 467 lines):
- Nearly identical local HTTP server
- Same PKCE flow, different token endpoint
- Additional: handles `iss` parameter for multi-region support

**Commonality:** ~80% identical code. A generic `OAuthCallbackServer<H: CodeExchangeHandler>` would eliminate most duplication.

#### Credential File I/O (3 implementations)

Each provider has its own read/write functions for auth JSON files:

| Provider | Read Function | Write Function | File Location |
|----------|--------------|----------------|---------------|
| Copilot | `read_copilot_auth_sync()` | `write_copilot_auth()` | `~/.fspec/credentials/copilot_auth.json` |
| Codex | `read_codex_auth()` | `write_codex_auth()` | `~/.codex/auth.json` |
| Claude | `read_claude_auth_sync()` | `write_claude_auth()` | `~/.fspec/credentials/claude_auth.json` |

All three follow the same pattern: resolve path → read file → deserialize JSON → validate fields. Could be unified into a generic `CredentialStore<T: DeserializeOwned + Serialize>`.

---

## 2. Provider Trait Architecture

The provider system is built on **4 core traits** in `codelet/providers/src/adapter.rs`:

```rust
trait LlmProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn complete_stream(&self, request: CompletionRequest) -> Result<StreamingResponse>;
    fn provider_name(&self) -> &str;
    fn model_id(&self) -> &str;
}
```

Each provider implements this trait. The `ProviderManager` in `manager.rs` handles credential detection and provider instantiation. New providers must:
1. Implement `LlmProvider`
2. Add credential detection to `ProviderCredentials`
3. Register in `ProviderManager`

---

## 3. Rhai Analysis

### 3.1 What is Rhai?

Rhai is an embedded scripting language for Rust (https://github.com/rhaiscript/rhai). Key characteristics:

- **Purely synchronous** — no async/await in scripts
- **Sandboxable** — `Engine::new_raw()` creates a clean engine with no standard library
- **Type-safe bridge** — Rust functions registered via `register_fn()` are callable from scripts
- **Serde integration** — `rhai = { features = ["serde"] }` enables automatic conversion between Rhai `Dynamic` and Rust types
- **Module system** — group related functions into modules registered as `oauth::`, `http::`, `json::`, etc.
- **Operation limits** — `set_max_operations(50_000)` prevents infinite loops
- **Call depth limits** — `set_max_call_levels(32)` prevents stack overflow
- **AST compilation** — scripts compile once, execute many times

### 3.2 Key Rhai Capabilities for OAuth

**Function registration:**
```rust
let mut engine = Engine::new_raw();  // No standard library = sandboxed

// Register Rust functions callable from scripts
engine.register_fn("http_post", |url: String, body: String| -> Dynamic {
    // Synchronous HTTP via ureq (not reqwest)
    let resp = ureq::post(&url).send_string(&body)?;
    // Return Dynamic map with status + body
});

engine.register_fn("base64url_encode", |data: String| -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data.as_bytes())
});

engine.register_fn("sha256", |data: String| -> String {
    // ... hash and return hex
});

engine.register_fn("json_parse", |s: String| -> Dynamic {
    // Parse JSON string to Rhai map
});
```

**Module system:**
```rust
let mut oauth_module = Module::new();
oauth_module.set_native_fn("generate_pkce", || -> Dynamic { /* ... */ });
oauth_module.set_native_fn("generate_state", || -> Dynamic { /* ... */ });

engine.register_static_module("oauth", oauth_module.into());
// Script: let pkce = oauth::generate_pkce();
```

### 3.3 Async Constraint and Workaround

Rhai is **purely synchronous**. Our codebase is async (tokio). The solution:

```rust
// Wrap Rhai execution in spawn_blocking
let result = tokio::task::spawn_blocking(move || {
    let engine = build_sandboxed_engine();
    engine.eval_ast::<Dynamic>(&compiled_ast)
}).await?;
```

HTTP calls inside scripts use `ureq` (synchronous HTTP client) rather than `reqwest` (async). Since OAuth token exchanges are infrequent (once per login, once per refresh), the sync overhead is negligible.

### 3.4 Cargo Dependencies

```toml
rhai = { version = "1.24", features = ["sync", "serde"] }
ureq = { version = "2", features = ["json", "tls"] }
```

- `rhai` with `sync` feature: ~500KB added to binary
- `ureq`: lightweight sync HTTP client, ~200KB
- Combined: ~700KB — acceptable for the extensibility gained

### 3.5 Sandboxing Model

```rust
fn build_sandboxed_engine() -> Engine {
    let mut engine = Engine::new_raw();  // NO standard library
    
    // Safety limits
    engine.set_max_operations(50_000);      // Prevent infinite loops
    engine.set_max_call_levels(32);         // Prevent stack overflow
    engine.set_max_string_size(1_048_576);  // 1MB max string
    engine.set_max_array_size(10_000);      // Max array elements
    engine.set_max_map_size(10_000);        // Max map entries
    
    // Register ONLY the functions we want scripts to access
    register_http_module(&mut engine);   // http::post, http::get
    register_crypto_module(&mut engine); // crypto::sha256, crypto::base64url
    register_json_module(&mut engine);   // json::parse, json::stringify
    register_oauth_module(&mut engine);  // oauth::generate_pkce, oauth::generate_state
    
    // NO filesystem access, NO network access beyond registered functions
    engine
}
```

Scripts cannot:
- Read/write files (no `std::fs` exposed)
- Make arbitrary network requests (only via registered `http::post`/`http::get`)
- Access environment variables
- Spawn processes
- Import other scripts

---

## 4. Architecture Options

### Option A: Pure Rust Trait Extraction (No Rhai)

Extract shared patterns into generic Rust traits and structs:

```
codelet/providers/src/oauth/
  ├── mod.rs               — re-exports
  ├── credential_store.rs  — generic CredentialStore<T: Serialize + DeserializeOwned>
  ├── http_middleware.rs    — generic RefreshingHttpClient<S: TokenStrategy>
  ├── device_flow.rs       — generic DeviceCodeFlow<P: DeviceCodeProvider>
  ├── callback_server.rs   — generic OAuthCallbackServer<H: CodeExchangeHandler>
  └── token_refresh.rs     — shared double-check locking refresh logic
```

**Key traits:**
```rust
trait TokenStrategy: Send + Sync {
    async fn needs_refresh(&self, state: &TokenState) -> bool;
    async fn refresh(&self, state: &mut TokenState) -> Result<()>;
    fn apply_headers(&self, state: &TokenState, req: &mut Request) -> Result<()>;
}

trait DeviceCodeProvider: Send + Sync {
    fn device_authorize_url(&self) -> &str;
    fn token_url(&self) -> &str;
    fn client_id(&self) -> &str;
    fn scopes(&self) -> &[&str];
    async fn post_process_token(&self, token: TokenResponse) -> Result<FinalToken>;
}

trait CodeExchangeHandler: Send + Sync {
    fn token_endpoint(&self) -> &str;
    fn client_id(&self) -> &str;
    async fn exchange_code(&self, code: &str, verifier: &str) -> Result<TokenResponse>;
}
```

**Estimated impact:**
- Removes ~750 lines of duplication
- Each new native provider: ~50-100 lines (implement traits) vs ~500+ lines today

**Pros:**
- Type-safe, zero runtime overhead
- Catches errors at compile time
- No new dependencies
- Maximum performance

**Cons:**
- Adding a new provider requires Rust code changes + recompilation
- Not user-extensible without recompiling
- Doesn't address the "custom provider" use case

### Option B: Rhai Scripting for Custom Providers

Use Rhai to define OAuth flow logic, with Rust building blocks:

```
codelet/providers/src/oauth/
  ├── mod.rs
  ├── engine.rs            — Rhai engine setup (Engine::new_raw() + sandboxing)
  ├── building_blocks.rs   — Register http::, crypto::, json::, oauth:: modules
  ├── credential_store.rs  — Shared credential file R/W (Rust, not script)
  ├── http_middleware.rs    — Generic RefreshingHttpClient that calls Rhai for refresh
  └── script_provider.rs   — ScriptedOAuthProvider that loads .rhai files
```

**User `.rhai` scripts define 5 functions:**

```javascript
// my_provider.rhai

// 1. Build authorization URL (for browser-based flows)
fn build_authorization_request(config) {
    let pkce = oauth::generate_pkce();
    let state = oauth::generate_state();
    #{
        url: `${config.auth_url}?client_id=${config.client_id}&redirect_uri=${config.redirect_uri}&code_challenge=${pkce.challenge}&code_challenge_method=S256&state=${state}&scope=${config.scopes}`,
        pkce_verifier: pkce.verifier,
        state: state
    }
}

// 2. Exchange authorization code for tokens
fn exchange_code(config, code, pkce_verifier) {
    let body = `grant_type=authorization_code&code=${code}&client_id=${config.client_id}&redirect_uri=${config.redirect_uri}&code_verifier=${pkce_verifier}`;
    let resp = http::post(config.token_url, body, #{ "Content-Type": "application/x-www-form-urlencoded" });
    let tokens = json::parse(resp.body);
    #{
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in
    }
}

// 3. Refresh expired token
fn refresh_token(config, current_tokens) {
    let body = `grant_type=refresh_token&refresh_token=${current_tokens.refresh_token}&client_id=${config.client_id}`;
    let resp = http::post(config.token_url, body, #{ "Content-Type": "application/x-www-form-urlencoded" });
    let tokens = json::parse(resp.body);
    #{
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in
    }
}

// 4. Single device code poll iteration (Rust controls the loop)
fn poll_for_token(config, device_data) {
    let body = `grant_type=urn:ietf:params:oauth:grant-type:device_code&device_code=${device_data.device_code}&client_id=${config.client_id}`;
    let resp = http::post(config.token_url, body, #{ "Content-Type": "application/x-www-form-urlencoded" });
    let result = json::parse(resp.body);
    if result.contains("error") {
        #{ status: result.error, interval: device_data.interval }
    } else {
        #{ status: "success", access_token: result.access_token, refresh_token: result.refresh_token, expires_in: result.expires_in }
    }
}

// 5. Check if token needs refresh
fn needs_refresh(tokens) {
    tokens.expires_at < timestamp::now() + 30
}
```

**Provider config (JSON):**
```json
{
    "name": "my-custom-provider",
    "display_name": "My Custom OAuth Provider",
    "script": "my_provider.rhai",
    "auth_url": "https://auth.example.com/authorize",
    "token_url": "https://auth.example.com/token",
    "client_id": "my-app-id",
    "redirect_uri": "http://localhost:0/callback",
    "scopes": "read write",
    "flow": "authorization_code",
    "credential_file": "my_provider_auth.json"
}
```

**Pros:**
- Users can add custom OAuth providers without recompiling
- Sandboxed execution (Engine::new_raw, 50K op limit, 30s timeout)
- Provider-specific quirks stay in scripts, not compiled code
- Scripts are human-readable and version-controllable
- Hot-reloadable (re-parse script on change)

**Cons:**
- New dependency (~700KB combined rhai + ureq)
- Async↔sync boundary adds complexity
- Debugging script errors harder than Rust compiler errors
- Value cloning overhead across Rhai boundary
- Security review needed for registered functions

### Option C: Hybrid (Recommended) ⭐

**Phase 1:** Extract Rust building blocks (Option A) — pure deduplication, no new deps.
**Phase 2:** Add Rhai integration on top of shared building blocks — user extensibility.

Built-in providers (Claude, Codex, Copilot) continue using **native Rust** via shared traits — they never go through Rhai. Only user-added custom providers use Rhai scripts.

```
codelet/providers/src/oauth/
  ├── mod.rs
  │
  │  ── Phase 1: Shared Rust Building Blocks ──
  ├── credential_store.rs    — generic CredentialStore<T>
  ├── http_middleware.rs      — generic RefreshingHttpClient<S: TokenStrategy>
  ├── device_flow.rs          — generic DeviceCodeFlow<P: DeviceCodeProvider>
  ├── callback_server.rs      — generic OAuthCallbackServer<H: CodeExchangeHandler>
  ├── token_refresh.rs        — shared double-check locking
  │
  │  ── Phase 2: Rhai Scripting Layer ──
  ├── engine.rs               — sandboxed Engine setup
  ├── building_blocks.rs      — http::, crypto::, json::, oauth:: modules
  └── script_provider.rs      — ScriptedOAuthProvider (loads .rhai files)
```

**This gives you:**
- **Native providers:** Maximum performance, compile-time safety, zero Rhai overhead
- **Custom providers:** Full user extensibility via scripts
- **Shared building blocks:** Used by both native and scripted providers

---

## 5. Recommendation: Option C (Hybrid)

### Why Hybrid?

1. **Phase 1 is mandatory regardless** — the ~750 lines of duplication must be unified before adding more providers. This is a standalone improvement with zero risk and no new dependencies.

2. **Rhai is an excellent fit** for OAuth scripting because:
   - OAuth flows are inherently procedural (build URL → exchange code → parse response)
   - Each step is a pure function from inputs to outputs
   - `Engine::new_raw()` gives true sandboxing with no filesystem/network access except registered functions
   - Scripts compile to AST once, execute many times
   - The 30-second timeout and 50K operation limit prevent hung scripts

3. **The async limitation is manageable** because:
   - OAuth token exchanges happen infrequently (once per login, once per refresh every ~hour)
   - `tokio::task::spawn_blocking` is the standard pattern
   - Only the Rhai evaluation + sync HTTP calls run on the blocking pool
   - The polling loop is controlled by Rust, not the script

4. **Incremental delivery** — Phase 1 can ship independently and provides immediate value. Phase 2 can be evaluated after Phase 1 proves the shared trait approach works.

### Estimated Effort

| Phase | Scope | Estimate |
|-------|-------|----------|
| **Phase 1a** | Extract `CredentialStore<T>` generic | 3 points |
| **Phase 1b** | Extract `RefreshingHttpClient<S>` generic | 5 points |
| **Phase 1c** | Extract `DeviceCodeFlow<P>` generic | 3 points |
| **Phase 1d** | Extract `OAuthCallbackServer<H>` generic | 5 points |
| **Phase 2a** | Rhai engine setup + sandboxing | 3 points |
| **Phase 2b** | Building block modules (http, crypto, json, oauth) | 5 points |
| **Phase 2c** | `ScriptedOAuthProvider` + config loading | 5 points |
| **Phase 2d** | Example .rhai scripts for all 3 providers | 3 points |
| **Total** | | **32 points** |

### Suggested Epic Breakdown

```
Epic: shared-oauth-rhai
  ├── PROV-0XX: Extract generic CredentialStore<T>
  ├── PROV-0XX: Extract generic RefreshingHttpClient<S: TokenStrategy>
  ├── PROV-0XX: Extract generic DeviceCodeFlow<P: DeviceCodeProvider>
  ├── PROV-0XX: Extract generic OAuthCallbackServer<H: CodeExchangeHandler>
  ├── PROV-0XX: Rhai engine setup + sandboxing infrastructure
  ├── PROV-0XX: Rhai building block modules (http, crypto, json, oauth)
  ├── PROV-0XX: ScriptedOAuthProvider + config loader
  └── PROV-0XX: Example .rhai scripts + documentation
```

---

## 6. File-by-File Impact Assessment

### Files to Modify (Phase 1)

| File | Lines | Change |
|------|-------|--------|
| `copilot/refreshing_client.rs` | 218 | Refactor to use generic middleware |
| `codex/refreshing_client.rs` | 319 | Refactor to use `RefreshingHttpClient<CodexTokenStrategy>` |
| `claude_refreshing_client.rs` | 295 | Refactor to use `RefreshingHttpClient<ClaudeTokenStrategy>` |
| `copilot/auth.rs` | 462 | Extract credential I/O to `CredentialStore<CopilotAuth>` |
| `codex/codex_auth.rs` | 226 | Extract credential I/O to `CredentialStore<CodexAuth>` |
| `claude_auth.rs` | 87 | Extract credential I/O to `CredentialStore<ClaudeAuth>` |
| `copilot/oauth_device_code.rs` | — | Refactor to use `DeviceCodeFlow<CopilotDeviceCode>` |
| `codex/codex_device_auth.rs` | 308 | Refactor to use `DeviceCodeFlow<CodexDeviceCode>` |
| `codex/codex_oauth_server.rs` | 389 | Refactor to use `OAuthCallbackServer<CodexExchange>` |
| `claude_oauth_server.rs` | 467 | Refactor to use `OAuthCallbackServer<ClaudeExchange>` |
| `credentials.rs` | 143 | Simplify using `CredentialStore` |

### New Files (Phase 1)

| File | Est. Lines | Purpose |
|------|------------|---------|
| `oauth/mod.rs` | 20 | Re-exports |
| `oauth/credential_store.rs` | 120 | Generic credential file I/O |
| `oauth/http_middleware.rs` | 150 | Generic refreshing HTTP client |
| `oauth/device_flow.rs` | 100 | Generic device code polling |
| `oauth/callback_server.rs` | 150 | Generic OAuth callback server |
| `oauth/token_refresh.rs` | 80 | Shared double-check locking |

### New Files (Phase 2)

| File | Est. Lines | Purpose |
|------|------------|---------|
| `oauth/engine.rs` | 100 | Sandboxed Rhai engine factory |
| `oauth/building_blocks.rs` | 200 | Registered Rhai modules |
| `oauth/script_provider.rs` | 150 | ScriptedOAuthProvider impl |

---

## 7. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Generic traits too restrictive for provider quirks | Medium | High | Design trait escape hatches; allow provider-specific overrides |
| Rhai sync HTTP adds latency | Low | Low | Only used for infrequent token operations; `spawn_blocking` isolates |
| Rhai script errors hard to debug | Medium | Medium | Comprehensive error messages; example scripts; validation on load |
| Breaking existing providers during refactor | Medium | High | Feature flags; extensive test coverage; incremental migration |
| Rhai dependency size concerns | Low | Low | ~700KB acceptable for extensibility gained |

---

## 8. References

- **Rhai repository:** https://github.com/rhaiscript/rhai (cloned to `/tmp/rhai`)
- **Rhai book:** https://rhai.rs/book/
- **RFC 7636 (PKCE):** https://tools.ietf.org/html/rfc7636
- **RFC 8628 (Device Authorization Grant):** https://tools.ietf.org/html/rfc8628
- **Existing shared modules:** `oauth_crypto.rs`, `oauth_http_utils.rs`, `credentials.rs`
- **Provider analysis agent session:** `b5933cbe-944d-4ebf-a4ee-8d23eb07e1a1`
- **Rhai analysis agent session:** `e538ad60-9854-4b26-bb0b-45f3233da16a`

---

## 9. Relationship to PROV-061 (Rhai-Scriptable Custom Provider Type)

**PROV-061 depends on this work unit.** The building blocks and Rhai infrastructure created here are the foundation for the fully scriptable custom provider type.

### What PROV-061 Reuses from PROV-060

| PROV-060 Deliverable | PROV-061 Usage |
|----------------------|----------------|
| Rhai engine setup (`engine.rs`) | Same engine factory, extended with more modules |
| `http` building block module | Used by custom provider scripts for sync HTTP |
| `crypto` building block module | Used for auth token signing, hashing |
| `json` building block module | Core of request/response transformation |
| `oauth` building block module | Custom OAuth auth flows |
| `CredentialStore<T>` | Custom provider credential persistence |
| `DeviceCodeFlow<P>` | Custom providers with device code OAuth |
| `OAuthCallbackServer<H>` | Custom providers with PKCE OAuth |
| `RefreshingHttpClient<S>` | Token refresh for custom OAuth providers |

### Design Implications for PROV-060

The Rhai engine and building block modules should be designed with PROV-061's needs in mind:

1. **Module registration should be extensible** — PROV-061 adds `time::` and `env::` modules alongside PROV-060's `http::`, `crypto::`, `json::`, `oauth::` modules
2. **Engine factory should accept a module list** — `build_sandboxed_engine(modules: &[RhaiModule])` rather than hardcoding modules
3. **Building block functions should return `Dynamic` maps** — not provider-specific types, so Rhai scripts can manipulate them freely
4. **Error handling should be Rhai-friendly** — return `Result<Dynamic, Box<EvalAltResult>>` so scripts get meaningful error messages

### See Also

- **PROV-061 research:** `spec/attachments/PROV-061/rhai-custom-provider-research.md`
- **Dependency:** PROV-061 → depends on → PROV-060
