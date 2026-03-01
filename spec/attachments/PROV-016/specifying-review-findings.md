# PROV-016 Specifying Review Findings

**Date:** 2026-03-01  
**Status:** 6 issues found — must be resolved before moving to testing

---

## Issue 1: Codex is NOT wired into the TUI agent loop — scope clarification needed

**Finding:** The TUI agent dispatch (`napi/src/session_manager.rs:5418-5422`) only handles `claude`, `openai`, `gemini`, and `zai`. There is **no `"codex"` branch** in the `run_with_provider!` calls.

```rust
let result = match current_provider.as_str() {
    "claude" => run_with_provider!(&mut inner_session, get_claude, ...),
    "openai" => run_with_provider!(&mut inner_session, get_openai, ...),
    "gemini" => run_with_provider!(&mut inner_session, get_gemini, ...),
    "zai"    => run_with_provider!(&mut inner_session, get_zai, ...),
    _ => Err(anyhow::anyhow!("Unsupported provider: {}", current_provider)),
};
```

**Impact:** The `@integration` scenario ("CodexProvider uses RefreshingCodexClient for OAuth mode") can be tested at the Rust level (unit/integration tests) but the full E2E path through the TUI won't work until the Codex branch is added. This is tracked by **PROV-017** (TUI OAuth Login Flow for Provider Settings).

**Decision:** This is **not a blocker** for PROV-016. Our scope is the Rust provider layer (RefreshingCodexClient + CodexProvider changes). The integration scenario should test Rust-level construction and type compatibility, not E2E agent loop dispatch.

**Action:** Update the `@integration` scenario description to clarify it tests Rust-level integration, not TUI dispatch.

---

## Issue 2: CodexProvider struct type mismatch — enum wrapper needed

**Finding:** Currently `CodexProvider` stores concrete `reqwest::Client`-typed fields:

```rust
pub struct CodexProvider {
    completion_model: openai::completion::CompletionModel,        // = CompletionModel<reqwest::Client>
    rig_client: openai::CompletionsClient,                        // = CompletionsClient<reqwest::Client>
    model_name: String,
    auth_mode: CodexAuthMode,
}
```

If `from_oauth_tokens()` uses `RefreshingCodexClient`, the fields become `CompletionModel<RefreshingCodexClient>` and `CompletionsClient<RefreshingCodexClient>` — a **completely different type**. Rust doesn't allow a struct field to hold two different concrete generic types without an enum or trait object.

The architecture note [1] says "ApiKey mode continues using reqwest::Client via an enum wrapper or by making CodexProvider generic" but doesn't commit to a solution.

**Analysis of rig type chain:**
- `openai::CompletionsClient<H>` = `client::Client<OpenAICompletionsExt, H>` 
- `CompletionModel<H>` stores `Client<H>` internally
- `Agent<M>` requires `M: CompletionModel` (trait)
- `create_rig_agent` returns `Agent<CompletionModel<H>>` — the `H` parameter propagates all the way

**Recommended solution:** Use `RefreshingCodexClient` for **ALL** CodexProvider modes:
- **OAuth mode:** Full refresh + URL rewrite + header injection
- **API key mode:** Pass-through mode — no refresh, no URL rewrite, just forward to reqwest with the API key headers

This gives one consistent type: `CompletionModel<RefreshingCodexClient>` always. The `RefreshingCodexClient` would have a `TokenMode` enum internally:

```rust
enum TokenMode {
    /// OAuth mode: refresh tokens, rewrite URLs, inject Bearer token
    OAuth {
        token_state: Arc<tokio::sync::RwLock<TokenState>>,
        issuer_url: String,
    },
    /// API key mode: pass-through to reqwest (no refresh, no URL rewrite)
    ApiKey,
}
```

**Action:** Update architecture note [1] and rule [7] to specify this unified approach.

---

## Issue 3: `tokio::sync::RwLock` required, not `std::sync::RwLock`

**Finding:** Rule [6] says `Arc<RwLock<TokenState>>` without specifying which `RwLock`. Since `refresh_access_token_at()` is `async` (it does HTTP calls), we **must** use `tokio::sync::RwLock`.

With `std::sync::RwLock`:
- Calling `refresh_access_token_at().await` while holding a `std::sync::RwLock` write guard would **block the tokio executor thread**, potentially deadlocking
- The `HttpClientExt::send()` return type is `impl Future<...> + Send + 'static` — so the future must be `Send`, which means it can't hold a `std::sync::RwLock` guard across an `.await` point

With `tokio::sync::RwLock`:
- Guards can be held across `.await` points safely
- The `OwnedWriteGuard` pattern allows moving the guard into the future

**Additional concern:** The `HttpClientExt` trait methods return futures that must be `'static`. This means we can't borrow `&self` across the async operation — we need to clone `Arc<TokenState>` and `Arc<reqwest::Client>` into the returned future.

**Action:** Update rule [6] to explicitly say `tokio::sync::RwLock`.

---

## Issue 4: Double-check locking pattern not specified

**Finding:** Rule [6] mentions thread safety via `Arc<RwLock<TokenState>>` but doesn't address the concurrent refresh race condition:

1. Thread A: read lock → token expired → drop read lock → write lock → refresh → update
2. Thread B: (was blocked on write) → gets write lock → **must re-check expiry** → already fresh → skip

Without the re-check in step 2, Thread B would also call `refresh_access_token_at()`, making a redundant network call and potentially racing on the token endpoint (e.g., OpenAI may invalidate the old refresh_token after first use).

Example 6 says "only one refresh occurs (RwLock ensures serialization)" but this is only true if we implement double-check locking:

```rust
// Correct pattern:
let state = self.token_state.read().await;
if state.is_expired() {
    drop(state);  // Release read lock
    let mut state = self.token_state.write().await;
    if state.is_expired() {  // Double-check after acquiring write lock
        // Actually refresh
        let tokens = refresh_access_token_at(&self.issuer_url, &state.refresh_token).await?;
        state.update(tokens);
    }
    // Use state.access_token for the request
}
```

**Action:** Add a new rule explicitly requiring double-check locking, or update rule [6] to include this pattern.

---

## Issue 5: `expires_in` might be `None` — no default specified

**Finding:** The `TokenRefreshResponse` struct has:

```rust
#[serde(default)]
pub expires_in: Option<u64>,
```

The OAuth server might not return `expires_in`. The spec doesn't define what happens in this case — does the token get treated as "never expires"? That would defeat the purpose of refresh.

**Recommendation:** Default to 3600 seconds (1 hour) if `expires_in` is not provided. This is the standard OAuth 2.0 convention.

**Action:** Add a rule specifying the default `expires_in` behavior.

---

## Issue 6: Feature file URL rewrite example is inconsistent with architecture

**Finding:** Example [3] and Scenario "URL rewrite for /v1/responses" say:

> "rig sends request to `https://chatgpt.com/backend-api/codex/responses/v1/responses`, RefreshingCodexClient rewrites to CODEX_API_ENDPOINT"

This URL is wrong. If we follow architecture note [3] ("No `.base_url()` needed since RefreshingCodexClient rewrites URLs itself"), then rig would use the default OpenAI base URL. The flow would be:

1. rig constructs request URL using default OpenAI base: `https://api.openai.com/v1/chat/completions`
2. `RefreshingCodexClient.send()` intercepts, calls `rewrite_codex_url()`, which detects `/chat/completions` → rewrites to `https://chatgpt.com/backend-api/codex/responses`
3. Forwards to reqwest with the rewritten URL

**But wait** — do we actually want to NOT set `base_url()`? If we don't set it, rig uses `https://api.openai.com`. The `rewrite_codex_url()` function checks for `/v1/responses` or `/chat/completions` in the URL, which would match `https://api.openai.com/v1/chat/completions`. That works.

**However**, for API key mode (pass-through), we DO want the default OpenAI base URL to go through unchanged. So the URL rewriting should only happen in OAuth mode, which aligns with the unified `TokenMode` enum approach from Issue 2.

**Current feature file line 82-83:**
```gherkin
Then the request URL should be rewritten to "https://chatgpt.com/backend-api/codex/responses"
```

This is correct for the end result, but Example [3] confusingly says the INPUT URL already has `chatgpt.com` in it.

**Action:** Fix Example [3] to show the correct input URL (`https://api.openai.com/v1/responses`), and update the feature scenario to clarify the full rewrite chain.

---

## Summary of Required Actions

| # | Issue | Severity | Action |
|---|-------|----------|--------|
| 1 | Codex not in TUI agent loop | Info | Clarify integration scenario scope |
| 2 | Type mismatch needs unified RefreshingCodexClient | Critical | Update arch notes: always use RefreshingCodexClient with TokenMode enum |
| 3 | Wrong RwLock type | High | Update rule [6]: `tokio::sync::RwLock` explicitly |
| 4 | No double-check locking | High | Add new rule for concurrent refresh pattern |
| 5 | No default for `expires_in: None` | Medium | Add rule: default to 3600s |
| 6 | URL rewrite example is wrong | Medium | Fix Example [3] input URL |

---

## Type System Trace (Reference)

For implementation reference, here's the complete type chain through rig:

```
openai::CompletionsClient<H>
  = client::Client<OpenAICompletionsExt, H>  (from client.rs:46)

openai::completion::CompletionModel<H>
  stores: Client<H> internally  (from completion/mod.rs:932-937)

rig::agent::Agent<M> where M: CompletionModel
  stores: Arc<M>  (from agent/completion.rs:51-76)

CodexProvider.create_rig_agent()
  returns: Agent<CompletionModel<H>>
  currently: Agent<CompletionModel<reqwest::Client>>
  after PROV-016: Agent<CompletionModel<RefreshingCodexClient>>

HttpClientExt trait requirements:
  - WasmCompatSend + WasmCompatSync (= Send + Sync on non-wasm)
  - send(): returns impl Future<...> + Send + 'static
  - send_multipart(): returns impl Future<...> + Send + 'static
  - send_streaming(): returns impl Future<...> + Send
```

```
RefreshingCodexClient must implement:
  - Clone (required by CompletionModel<T> derive)
  - Default + Debug + Clone + 'static (required by CompletionModel::new())
  - HttpClientExt (required for actual HTTP calls)
  - Send + Sync (required by WasmCompatSend/Sync)
```

---

## Codebase Cross-References

- **CodexProvider:** `codelet/providers/src/codex/mod.rs`
- **codex_oauth.rs:** `codelet/providers/src/codex/codex_oauth.rs` (rewrite_codex_url, build_codex_headers, refresh_access_token_at, TokenRefreshResponse)
- **codex_auth.rs:** `codelet/providers/src/codex/codex_auth.rs` (read_codex_auth, write_codex_auth, CodexAuthJson, CodexTokens)
- **HttpClientExt trait:** `codelet/patches/rig-core/src/http_client/mod.rs:101-129`
- **reqwest impl:** `codelet/patches/rig-core/src/http_client/mod.rs:131-272`
- **CompletionModel<T>:** `codelet/patches/rig-core/src/providers/openai/completion/mod.rs:932`
- **CompletionsClient type:** `codelet/patches/rig-core/src/providers/openai/client.rs:46`
- **ClientBuilder.http_client():** `codelet/patches/rig-core/src/client/mod.rs:523`
- **Agent<M>:** `codelet/patches/rig-core/src/agent/completion.rs:51`
- **RigAgent<M>:** `codelet/core/src/rig_agent.rs:22`
- **TUI dispatch (no codex branch):** `codelet/napi/src/session_manager.rs:5418-5422`
- **ProviderManager.get_codex():** `codelet/providers/src/manager.rs:416-425`
- **PROV-011 parent:** Browser+Device OAuth umbrella
- **PROV-013:** Browser OAuth (done)
- **PROV-014:** Device Auth (done)
- **PROV-015:** NAPI Bindings (done)
- **PROV-017:** TUI OAuth Login Flow (backlog — adds codex branch to dispatch)
