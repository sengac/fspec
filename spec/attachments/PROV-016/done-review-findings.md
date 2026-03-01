# PROV-016 Done Review — Critical Findings

**Date:** 2026-03-01
**Reviewer:** Claude Code (critical review requested by user)
**Status:** ❌ **NOT properly completed per spec — must reopen**

---

## Executive Summary

The `RefreshingCodexClient` implementation (`refreshing_client.rs`) is well-structured and internally correct. However, it is a **dead module** — nothing in the production codebase uses it. The entire purpose of PROV-016 was to wire this middleware into `CodexProvider`, and that integration was never done. Additionally, 5 of 12 tests are stubs that don't test the behavior described by their scenarios, and 1 scenario step is verified only by a code comment ("verified by write_codex_auth call in the implementation") with no actual assertion.

---

## 🔴 CRITICAL: RefreshingCodexClient is NOT wired into CodexProvider

### What the spec requires

| Source | Requirement |
|--------|-------------|
| Rule [6] | `CodexProvider::from_oauth_tokens()` changes from `CompletionsClient<reqwest::Client>` to `CompletionsClient<RefreshingCodexClient>` |
| Rule [12] | RefreshingCodexClient is used for ALL CodexProvider modes. In API key mode it operates in pass-through mode. This gives one consistent type: `CompletionModel<RefreshingCodexClient>` |
| Arch Note [3] | CodexProvider struct is UNIFIED: both OAuth and API key modes use RefreshingCodexClient. Fields become `CompletionModel<RefreshingCodexClient>` and `CompletionsClient<RefreshingCodexClient>`. No enum wrapper or generics needed on CodexProvider itself. |

### What the code actually has

**`codelet/providers/src/codex/mod.rs` lines 43–48** — still uses default `reqwest::Client`:
```rust
pub struct CodexProvider {
    completion_model: openai::completion::CompletionModel,     // = CompletionModel<reqwest::Client>
    rig_client: openai::CompletionsClient,                     // = CompletionsClient<reqwest::Client>
    model_name: String,
    auth_mode: CodexAuthMode,
}
```

**`from_oauth_tokens()` (lines 152–194)** — still uses the OLD static-headers approach that the specifying review specifically identified as the problem to solve:
```rust
pub fn from_oauth_tokens(
    access_token: &str,
    account_id: &str,
    model: &str,
) -> Result<Self, ProviderError> {
    // ...
    let rig_client = openai::CompletionsClient::builder()
        .api_key(access_token)                        // ← static token, frozen at construction
        .base_url(codex_oauth::CODEX_API_ENDPOINT)    // ← static URL, no dynamic rewrite
        .http_headers(headers)                        // ← static headers, frozen at construction
        .build()
```

**`from_api_key()` (lines 120–141)** — also still uses bare `reqwest::Client`:
```rust
let rig_client = openai::CompletionsClient::builder()
    .api_key(api_key)
    .build()
```

### Proof it's unused

```
$ grep -rn "RefreshingCodexClient" codelet/providers/src/codex/mod.rs
22:pub mod refreshing_client;    ← module declared but never imported/used
```

No import of `RefreshingCodexClient` exists anywhere in `mod.rs`. The module is publicly declared so tests can reach it, but no production code references it.

### What this means

- ❌ **Token refresh NEVER happens** — the access token baked in at construction time is used until the session ends
- ❌ **Dynamic URL rewriting NEVER happens** — `base_url` is hard-coded to `CODEX_API_ENDPOINT` at construction
- ❌ **Dynamic header injection NEVER happens** — headers are frozen at construction
- ❌ **The 30-second expiry buffer, double-check locking, default 3600s expiry** — all of it is dead code
- ❌ **API key pass-through mode** — not utilized; `from_api_key()` still builds a bare `reqwest::Client`

### What should exist

```rust
pub struct CodexProvider {
    completion_model: openai::completion::CompletionModel<RefreshingCodexClient>,
    rig_client: openai::CompletionsClient<RefreshingCodexClient>,
    model_name: String,
    auth_mode: CodexAuthMode,
}
```

And `from_oauth_tokens()` should accept `refresh_token`, `issuer_url`, and `expires_in` (currently missing from the signature), construct a `RefreshingCodexClient::new_oauth(...)`, and pass it as the HTTP client via the rig builder's `.http_client()` method.

And `from_api_key()` should construct `RefreshingCodexClient::new_api_key()` and pass that as the HTTP client, giving one unified type.

### Additional: `from_oauth_tokens()` signature is missing required parameters

Current signature:
```rust
pub fn from_oauth_tokens(access_token: &str, account_id: &str, model: &str)
```

Missing parameters needed by `RefreshingCodexClient::new_oauth()`:
- `refresh_token: &str` — needed for token refresh
- `issuer_url: &str` — needed for refresh endpoint (defaults to `CODEX_ISSUER` but must be configurable for testability)
- `expires_in_secs: Option<u64>` — needed for initial expiry tracking

The call site in `CodexProvider::new()` (line 96) has access to `tokens.refresh_token` from `CodexTokens` but doesn't pass it.

### Additional: `create_rig_agent()` return type must change

Currently returns `Agent<openai::completion::CompletionModel>` which is `Agent<CompletionModel<reqwest::Client>>`. After wiring, it must return `Agent<CompletionModel<RefreshingCodexClient>>`.

---

## 🔴 CRITICAL: 5 of 12 tests are stubs masquerading as passes

### Test #1: `test_existing_authorization_header_is_replaced` (line 486)

**Scenario says:** _"When the client sends the request → Then the dummy Authorization header should be stripped → And replaced with Bearer {current_access_token}"_

**What the test actually does:**
```rust
let _client = build_oauth_client(/* ... */);  // ← stored as _client, NEVER USED
let req = make_request_with_dummy_auth("https://api.openai.com/v1/chat/completions");

// Verify the RAW request has dummy header (not useful — we're testing our own test helper)
assert_eq!(req.headers().get("authorization").unwrap(), "Bearer dummy-api-key");

// This is STRING FORMATTING, not client behavior:
assert_eq!(format!("Bearer {}", access_token), "Bearer real_bearer_token");
```

**The comment on line 512–513 says:**
> "When implemented, RefreshingCodexClient will strip the dummy and inject the real token. The stub passes through unchanged."

This is stub language for a work unit marked "done". The request is **never sent through the client**. No header replacement is verified.

**What it should do:** Send `make_request_with_dummy_auth()` through `client.send()` to a mock backend, then assert the mock backend received `Bearer real_bearer_token` instead of `Bearer dummy-api-key`.

---

### Test #2: `test_url_rewrite_for_v1_responses_path` (line 330)

**What the test does:**
```rust
let _client = build_oauth_client(/* ... */);  // ← UNUSED (underscore prefix!)
let rewritten = rewrite_codex_url("https://api.openai.com/v1/responses");
assert_eq!(rewritten, CODEX_API_ENDPOINT);
```

**Comment on line 351–352:**
> "When RefreshingCodexClient is implemented, it will call rewrite_codex_url internally. This test verifies the rewrite logic that the client will use."

This tests the free function `rewrite_codex_url()` directly. It does NOT test that `RefreshingCodexClient` actually rewrites URLs when processing requests. The function was already tested in `codex_oauth.rs::tests::test_url_rewrite_v1_responses()`. This is a duplicate of an existing unit test, not a test of the scenario.

---

### Test #3: `test_url_rewrite_for_chat_completions_path` (line 360)

Same issue as Test #2. Client created as `_client` (unused), only tests `rewrite_codex_url()` directly. Duplicate of `codex_oauth.rs::tests::test_url_rewrite_chat_completions()`.

---

### Test #4: `test_non_api_urls_pass_through_without_rewrite` (line 386)

**Partial stub.** Tests `rewrite_codex_url()` correctly for the non-rewrite case, but does NOT verify the scenario step:
> "And the auth headers should still be set correctly"

**Comment on line 407–408:**
> "The stub doesn't set headers yet - this drives implementation."

Again, stub language on a "done" work unit. The test should send a request to a non-rewritable URL through the client and verify auth headers are present on the forwarded request.

---

### Test #5: `test_codex_provider_uses_refreshing_client_for_oauth_mode` (line 526)

**Scenario says:** _"When CodexProvider::from_oauth_tokens() is called → Then a RefreshingCodexClient should be created with OAuth TokenMode → And it should be passed as the HTTP client to rig CompletionsClient<RefreshingCodexClient>"_

**What the test does:**
```rust
// Creates a standalone RefreshingCodexClient — never involves CodexProvider
let client = RefreshingCodexClient::new_oauth(/* ... */);
let _clone = client.clone();
let _default = RefreshingCodexClient::default();
let _debug = format!("{:?}", client);
```

**Comment on line 552:**
> "The actual CodexProvider::from_oauth_tokens() change is part of implementation."

`CodexProvider` is never created, never tested. This test only verifies `RefreshingCodexClient` implements `Clone + Default + Debug` (compile-time check) and `is_token_expired()`. It's completely decoupled from the scenario it claims to cover.

---

### Test #6: `test_api_key_mode_passes_requests_through_unchanged` (line 595)

**Scenario says:** _"When the client sends a request → Then the request URL should NOT be rewritten → And no token refresh should occur → And the original headers from rig should be preserved → And the request should be forwarded to reqwest as-is"_

**What the test does:**
```rust
let client = RefreshingCodexClient::new_api_key();
assert!(!client.is_token_expired().await);  // Only checks expiry flag

let req = make_request("https://api.openai.com/v1/chat/completions");
assert_eq!(req.uri().to_string(), "...");  // Checks the RAW request URI, never sent through client
```

No request is sent through the client. The URI assertion is on the raw `http::Request` builder output — it proves nothing about ApiKey pass-through behavior. None of the 4 "Then" steps are actually verified through the client.

---

## 🟡 MEDIUM: auth.json persistence not verified in tests

### Feature step:
> "And the refreshed tokens should be persisted to auth.json" (line 69)

### Test (line 253–254):
```rust
// @step And the refreshed tokens should be persisted to auth.json
// (verified by write_codex_auth call in the implementation)
```

This comment-only "verification" doesn't actually assert anything. The test sets up `CODEX_HOME` via `setup_codex_home()` and the implementation does call `write_codex_auth()`, so in theory the file is written. But no assertion reads back `auth.json` to confirm the persisted data is correct (right access_token, right refresh_token, right account_id).

The happy-path test (`test_expired_token_is_automatically_refreshed_before_request`) and streaming test both call `setup_codex_home()`, which provides a temp directory. A proper assertion would be:

```rust
let auth = codex_auth::read_codex_auth().unwrap().unwrap();
let tokens = auth.tokens.unwrap();
assert_eq!(tokens.access_token, "new_access_tok");
assert_eq!(tokens.refresh_token, "new_refresh_tok");
```

---

## 🟡 MEDIUM: Synchronous file I/O inside write lock

In `ensure_fresh_token()` (line 155), `persist_tokens()` is called while holding the `tokio::sync::RwLock` write guard:

```rust
{
    let mut state = token_state.write().await;
    // ... refresh ...
    update_token_state(&mut state, &response);
    persist_tokens(&state, &response);  // ← sync file I/O under write lock
}
```

`persist_tokens()` calls `write_codex_auth()` which does:
```rust
let content = serde_json::to_string_pretty(auth)?;
fs::write(&auth_path, content)?;  // ← blocking I/O
```

This means ALL concurrent requests are blocked during synchronous file I/O (`fs::write`). In a high-concurrency scenario, this could cause noticeable latency spikes.

**Fix:** Clone the data needed for persistence, drop the write lock, then persist outside the lock scope:

```rust
{
    let mut state = token_state.write().await;
    // ... refresh ...
    update_token_state(&mut state, &response);
    // Clone for persistence outside the lock
    let persist_state = state.clone();
    let persist_response = response.clone();
    drop(state); // or just let the block end here

    persist_tokens(&persist_state, &persist_response);
}
```

**Severity:** Medium. Correctness is fine (best-effort persistence doesn't fail the request). But it violates the principle of holding locks for the minimum duration.

---

## 🟡 MEDIUM: Concurrent refresh scenario has no test coverage

Rule [10] specifies:
> Concurrent refresh uses double-check locking: read lock → check expired → drop read → write lock → RE-CHECK expired → refresh only if still expired. This prevents redundant refresh calls when multiple requests detect expiry simultaneously

Example map example [5]:
> Concurrent requests during refresh: Two requests arrive while token is expired, only one refresh occurs

The implementation correctly implements double-check locking (lines 126–158). However, there is **no test** for this behavior. The feature file mentions it in the example mapping comments (line 37) but there is no Gherkin scenario for concurrent refresh — it was captured as Example [5] but not elevated to a scenario.

This is acceptable (the example mapping correctly chose which examples to elevate), but it means the double-check locking pattern is untested. A test using `tokio::spawn` to fire two concurrent requests through an expired client would verify only one refresh call is made.

---

## 🟢 What was done well

### refreshing_client.rs is architecturally sound

The module itself is well-designed and internally correct:

1. **Correct double-check locking** with `tokio::sync::RwLock` (not `std::sync::RwLock`) — required because `refresh_access_token_at()` is async and the guard must be held across `.await` points (Rule [9])
2. **Clean `TokenMode` enum** — `OAuth { token_state }` vs `ApiKey` for unified type (Rule [12])
3. **Proper expiry buffer** — 30-second proactive refresh (Rule [11], `EXPIRY_BUFFER_SECS`)
4. **Default expiry fallback** — `DEFAULT_EXPIRY_SECS = 3600` when `expires_in` is `None` (Rule [11])
5. **`prepare_oauth_request()` is a clean generic helper** — handles URL rewrite + header stripping + header injection in one composable function
6. **`update_token_state()` and `persist_tokens()` are properly extracted** — SOLID single-responsibility functions
7. **All three `HttpClientExt` methods** (`send`, `send_multipart`, `send_streaming`) are implemented with the same pattern
8. **Best-effort persistence** — `persist_tokens()` logs failure but doesn't fail the request

### Tests that DO test real behavior are good

The happy-path tests are well-written:

- `test_request_with_valid_token_passes_through_with_correct_headers` — uses two wiremock servers (auth + backend), verifies received headers on the backend
- `test_expired_token_is_automatically_refreshed_before_request` — verifies refresh call count AND that the backend receives the new token
- `test_token_refresh_failure_propagates_error` — verifies error propagation AND that backend receives zero requests (`.expect(0)`)
- `test_streaming_request_with_expired_token_refreshes_before_streaming` — tests `send_streaming()` path specifically

### Test fixtures are reusable and clean

- `build_oauth_client()` / `build_expired_oauth_client()` — clean builder helpers
- `mount_successful_refresh()` / `mount_failed_refresh()` — wiremock setup is shared
- `setup_codex_home()` with RAII guard for env var cleanup

---

## DRY/SOLID Assessment

### DRY ✅ (acceptable)

The three `HttpClientExt` methods have a repeated match pattern:
```rust
match &mode {
    TokenMode::OAuth { token_state } => {
        Self::ensure_fresh_token(token_state).await?;
        let state = token_state.read().await;
        prepare_oauth_request(req, &state.access_token, &state.account_id)
    }
    TokenMode::ApiKey => req,
};
```

This is duplicated 3 times but is largely unavoidable due to Rust's type system — each method has a different body type (`T`, `MultipartForm`, `T`). The shared logic (`ensure_fresh_token` and `prepare_oauth_request`) IS properly extracted.

### SOLID ✅ (good)

- **S** — `RefreshingCodexClient` has one responsibility: HTTP middleware for token management
- **O** — `TokenMode` enum is open for extension (new modes can be added)
- **L** — `Default` impl creates ApiKey mode (pass-through), which is the least-surprise default
- **I** — Only implements `HttpClientExt` (the minimal required interface)
- **D** — Depends on `refresh_access_token_at()` function (injectable via `issuer_url`), not on concrete OAuth implementation

---

## Summary of Required Fixes

| Priority | Issue | Fix |
|----------|-------|-----|
| 🔴 Critical | RefreshingCodexClient not wired into CodexProvider | Change `CodexProvider` struct fields to use `RefreshingCodexClient` as the generic `H` type parameter. Rewrite `from_oauth_tokens()` (add missing params) and `from_api_key()`. Update `create_rig_agent()` return type. |
| 🔴 Critical | `from_oauth_tokens()` missing `refresh_token`, `issuer_url`, `expires_in` params | Update signature and the call site in `CodexProvider::new()` |
| 🔴 Critical | 5 stub tests (auth header, 2x URL rewrite, CodexProvider integration, API key pass-through) | Rewrite each to send requests through the client and assert on actual received data at mock backends |
| 🟡 Medium | auth.json persistence not asserted | Add assertions that read back the temp `auth.json` and verify contents |
| 🟡 Medium | Sync file I/O under write lock | Clone data and persist after dropping lock |
| 🟡 Medium | No test for concurrent double-check locking | Consider adding a `tokio::spawn` concurrency test (optional — not a scenario) |

### Recommendation

Move PROV-016 back to `implementing` to complete the integration work. The `refreshing_client.rs` module is ready — it just needs to be connected to `CodexProvider` and the stub tests need to be replaced with real integration tests.
