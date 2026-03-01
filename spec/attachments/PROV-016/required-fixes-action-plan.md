# PROV-016 Required Fixes — Action Plan

**Date:** 2026-03-01
**Status:** Must reopen to `implementing`

---

## Fix 1: Wire RefreshingCodexClient into CodexProvider struct

**File:** `codelet/providers/src/codex/mod.rs`

### Step 1a: Change CodexProvider struct fields (lines 43–48)

**Current (broken):**
```rust
pub struct CodexProvider {
    completion_model: openai::completion::CompletionModel,     // CompletionModel<reqwest::Client>
    rig_client: openai::CompletionsClient,                     // CompletionsClient<reqwest::Client>
    model_name: String,
    auth_mode: CodexAuthMode,
}
```

**Required:**
```rust
pub struct CodexProvider {
    completion_model: openai::completion::CompletionModel<RefreshingCodexClient>,
    rig_client: openai::CompletionsClient<RefreshingCodexClient>,
    model_name: String,
    auth_mode: CodexAuthMode,
}
```

Add `use refreshing_client::RefreshingCodexClient;` to the imports.

### Step 1b: Update `from_oauth_tokens()` signature and body (lines 152–194)

**Current (broken):** Takes only `access_token`, `account_id`, `model`. Builds a `CompletionsClient<reqwest::Client>` with static headers and static base URL. Token can never refresh.

**Required:**
- Add parameters: `refresh_token: &str`, `issuer_url: &str`, `expires_in_secs: Option<u64>`
- Construct `RefreshingCodexClient::new_oauth(access_token, refresh_token, account_id, expires_in_secs, issuer_url)`
- Pass the `RefreshingCodexClient` to the rig builder via `.http_client(refreshing_client)` instead of `.base_url()` + `.http_headers()`
- Remove the static `HeaderMap` construction — `RefreshingCodexClient` handles headers dynamically
- The builder still needs `.api_key("dummy")` (rig requires it) — `RefreshingCodexClient` strips and replaces it

### Step 1c: Update `from_api_key()` (lines 120–141)

**Current (broken):** Builds bare `CompletionsClient<reqwest::Client>`.

**Required:**
- Construct `RefreshingCodexClient::new_api_key()` (pass-through mode)
- Pass it to the rig builder via `.http_client(refreshing_client)`
- This gives unified type `CompletionsClient<RefreshingCodexClient>` for both modes

### Step 1d: Update call site in `CodexProvider::new()` (lines 96–100)

**Current:**
```rust
return Self::from_oauth_tokens(
    &tokens.access_token,
    &tokens.account_id,
    &model_name,
);
```

**Required:** Pass `refresh_token`, `issuer_url`, and `expires_in` from `CodexTokens`:
```rust
return Self::from_oauth_tokens(
    &tokens.access_token,
    &tokens.refresh_token,
    &tokens.account_id,
    None, // expires_in not stored in auth.json — defaults to 3600s
    codex_oauth::CODEX_ISSUER,
    &model_name,
);
```

### Step 1e: Update `create_rig_agent()` return type (line 225)

**Current:** `-> rig::agent::Agent<openai::completion::CompletionModel>`
**Required:** `-> rig::agent::Agent<openai::completion::CompletionModel<RefreshingCodexClient>>`

### Step 1f: Update `rig_response_to_completion()` signature (line 259)

The `response` parameter type must change to use `CompletionModel<RefreshingCodexClient>` instead of the default `CompletionModel<reqwest::Client>`.

### Step 1g: Verify rig builder has `.http_client()` method

The rig `ClientBuilder` should accept a custom HTTP client. Check `codelet/patches/rig-core/src/client/mod.rs` for the `.http_client()` builder method. If it doesn't exist, it may need to be added to the patched rig-core.

---

## Fix 2: Rewrite 5 stub tests to test actual behavior

**File:** `codelet/providers/tests/codex_refreshing_client_test.rs`

### Fix 2a: `test_existing_authorization_header_is_replaced` (line 486)

**Problem:** Client stored as `_client` (unused). Only tests string formatting. Comment says "stub passes through unchanged".

**Fix:** Send `make_request_with_dummy_auth()` through `client.send()` to a mock backend. Assert the mock backend received `Bearer {access_token}` NOT `Bearer dummy-api-key`.

```rust
// Mount backend
Mock::given(method("POST"))
    .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
    .mount(&backend)
    .await;

// Send through client (non-rewritable path to hit our mock)
let url = format!("{}/v1/models", backend.uri());
let req = make_request_with_dummy_auth(&url);
let _result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
    client.send(req).await;

// Verify dummy header was stripped and replaced
let received = backend.received_requests().await.unwrap();
assert_eq!(received[0].headers.get("authorization").unwrap(), &format!("Bearer {access_token}"));
```

### Fix 2b: `test_url_rewrite_for_v1_responses_path` (line 330)

**Problem:** Client stored as `_client` (unused). Only tests `rewrite_codex_url()` directly (already tested in `codex_oauth.rs`).

**Fix:** Send a request with `/v1/responses` path through the client. This requires a backend mock mounted at the Codex API endpoint, or assert via the rewrite happening inside `prepare_oauth_request()`. Since we can't easily mock `chatgpt.com`, verify via the client's internal URL rewrite by checking the backend doesn't receive the request (it was rewritten to a different host). Alternatively, test using a request to a known backend URL containing `/v1/responses` and verify the rewrite.

Simplest approach: Keep the `rewrite_codex_url` assertion but ALSO send a request through the client and verify it attempted to connect to the rewritten URL (the send will fail with a connection error to `chatgpt.com`, but that proves the rewrite happened). Or mount a mock at the rewritten URL path.

### Fix 2c: `test_url_rewrite_for_chat_completions_path` (line 360)

Same fix as 2b but for `/chat/completions` path.

### Fix 2d: `test_non_api_urls_pass_through_without_rewrite` (line 386)

**Problem:** Comment says "The stub doesn't set headers yet". Never verifies auth headers on non-rewritable URLs.

**Fix:** Send request through client to a backend at `/v1/models`. Assert the backend receives the request at the original URL AND auth headers are set:

```rust
let received = backend.received_requests().await.unwrap();
assert_eq!(received[0].headers.get("authorization").unwrap(), &format!("Bearer {access_token}"));
assert_eq!(received[0].headers.get("chatgpt-account-id").unwrap(), account_id);
assert_eq!(received[0].headers.get("originator").unwrap(), "codelet");
```

### Fix 2e: `test_codex_provider_uses_refreshing_client_for_oauth_mode` (line 526)

**Problem:** Never creates or tests `CodexProvider`. Only verifies `RefreshingCodexClient` trait bounds.

**Fix:** After Fix 1 is done, actually call `CodexProvider::from_oauth_tokens()` and verify it returns `Ok`. Verify the provider can construct a rig Agent. This is an integration test that proves the full type chain works:

```rust
let provider = CodexProvider::from_oauth_tokens(
    access_token, refresh_token, account_id, Some(3600),
    &mock_server.uri(), "gpt-5.1-codex",
).unwrap();
let _agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, None);
```

### Fix 2f: `test_api_key_mode_passes_requests_through_unchanged` (line 595)

**Problem:** Never sends a request through the client. URI assertion is on the raw `http::Request` builder output.

**Fix:** Send a request through `client.send()` to a mock backend. Assert the backend receives the request at the ORIGINAL URL with the ORIGINAL headers (no rewrite, no token injection):

```rust
let backend = MockServer::start().await;
Mock::given(method("POST"))
    .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
    .mount(&backend)
    .await;

let url = format!("{}/v1/chat/completions", backend.uri());
let req = make_request(&url);
let _result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
    client.send(req).await;

let received = backend.received_requests().await.unwrap();
assert_eq!(received.len(), 1);
// No ChatGPT-Account-Id header injected
assert!(received[0].headers.get("chatgpt-account-id").is_none());
// No originator header injected
assert!(received[0].headers.get("originator").is_none());
```

---

## Fix 3: Assert auth.json persistence in refresh test

**File:** `codelet/providers/tests/codex_refreshing_client_test.rs`
**Test:** `test_expired_token_is_automatically_refreshed_before_request` (line 253)

**Problem:** Step "And the refreshed tokens should be persisted to auth.json" verified only by comment.

**Fix:** Add assertion after send:

```rust
// @step And the refreshed tokens should be persisted to auth.json
let persisted = codex_auth::read_codex_auth().unwrap().unwrap();
let tokens = persisted.tokens.unwrap();
assert_eq!(tokens.access_token, "new_access_tok");
assert_eq!(tokens.refresh_token, "new_refresh_tok");
```

Add `use codelet_providers::codex::codex_auth;` to test imports.

---

## Fix 4: Move persist_tokens() outside write lock scope

**File:** `codelet/providers/src/codex/refreshing_client.rs`
**Function:** `ensure_fresh_token()` (lines 134–158)

**Problem:** `persist_tokens()` does synchronous `fs::write` while holding `tokio::sync::RwLock` write guard. Blocks all concurrent requests during file I/O.

**Fix:** Clone data, drop lock, then persist:

```rust
// Write lock: double-check and refresh if still expired
let persist_data = {
    let mut state = token_state.write().await;
    if Instant::now() + buffer < state.expires_at {
        return Ok(());
    }

    debug!("Codex access token expired, refreshing...");
    let response = refresh_access_token_at(&state.issuer_url, &state.refresh_token)
        .await
        .map_err(|e| {
            rig::http_client::Error::Instance(
                format!("Token refresh failed: {e}").into(),
            )
        })?;

    update_token_state(&mut state, &response);
    debug!("Codex access token refreshed successfully");

    // Clone for persistence outside the lock
    Some((state.clone(), response))
};
// Write lock dropped here

// Persist to auth.json outside the lock (best-effort)
if let Some((state, response)) = persist_data {
    persist_tokens(&state, &response);
}
```

---

## Execution Order

1. **Fix 1** first (wiring) — this is the core integration work
2. **Fix 2e** next (CodexProvider integration test) — depends on Fix 1
3. **Fixes 2a–2d, 2f** (remaining stub tests) — can be done in parallel
4. **Fix 3** (persistence assertion) — independent
5. **Fix 4** (lock optimization) — independent, low risk

---

## Verification Checklist

After all fixes:

- [ ] `cargo test -p codelet-providers --test codex_refreshing_client_test` — all 12 pass
- [ ] `cargo build -p codelet-providers` — compiles with new generic types
- [ ] `cargo build -p codelet-napi` — downstream compiles (type change propagates)
- [ ] `cargo build -p codelet-cli` — downstream compiles
- [ ] No `_client` unused variables in test file
- [ ] No "When implemented" or "stub" comments in test file
- [ ] `grep -rn "RefreshingCodexClient" codelet/providers/src/codex/mod.rs` shows actual usage, not just `pub mod`
