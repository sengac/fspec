# AST Research: RefreshingCodexClient Implementation Surface

**Date:** 2026-03-01
**Work Unit:** PROV-016

## 1. HttpClientExt Trait (to implement)

**File:** `codelet/patches/rig-core/src/http_client/mod.rs`

### Trait definition (line 101-129)
```rust
pub trait HttpClientExt: WasmCompatSend + WasmCompatSync {
    fn send<T, U>(&self, req: Request<T>) -> impl Future<Output = Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where T: Into<Bytes> + WasmCompatSend, U: From<Bytes> + WasmCompatSend + 'static;

    fn send_multipart<U>(&self, req: Request<MultipartForm>) -> impl Future<Output = Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where U: From<Bytes> + WasmCompatSend + 'static;

    fn send_streaming<T>(&self, req: Request<T>) -> impl Future<Output = Result<StreamingResponse>> + WasmCompatSend
    where T: Into<Bytes>;
}
```

### Existing implementations
- `impl HttpClientExt for reqwest::Client` — line 131 (reference implementation to mirror)
- `impl HttpClientExt for reqwest_middleware::ClientWithMiddleware` — line 276

### Key observations
- `send()` and `send_multipart()` return `'static` futures — cannot borrow `&self` across await, must clone Arc-wrapped state
- `send_streaming()` does NOT require `'static` — can borrow self
- reqwest impl decomposes `Request<T>` into `(parts, body)` then rebuilds with `self.request(parts.method, parts.uri.to_string()).headers(parts.headers).body(...)`
- Error check: `response.status().is_success()` → `Error::InvalidStatusCodeWithMessage`

## 2. CodexProvider Struct (to modify)

**File:** `codelet/providers/src/codex/mod.rs` — line 42

```rust
pub struct CodexProvider {
    completion_model: openai::completion::CompletionModel,        // = CompletionModel<reqwest::Client>
    rig_client: openai::CompletionsClient,                        // = CompletionsClient<reqwest::Client>
    model_name: String,
    auth_mode: CodexAuthMode,
}
```

**After PROV-016:** All fields become `CompletionModel<RefreshingCodexClient>` / `CompletionsClient<RefreshingCodexClient>`.

## 3. from_oauth_tokens() — line 151

Currently builds static headers and uses `.base_url(CODEX_API_ENDPOINT)`:
```rust
pub fn from_oauth_tokens(access_token: &str, account_id: &str, model: &str) -> Result<Self, ProviderError>
```

**After PROV-016:** Creates `RefreshingCodexClient` in OAuth mode, passes as `.http_client()` to builder. No `.base_url()` or `.http_headers()` needed — RefreshingCodexClient handles URL rewriting and headers dynamically.

**Signature change:** Must also accept `refresh_token` and `issuer_url` for token refresh support.

## 4. from_api_key() — line 119

```rust
pub fn from_api_key(api_key: &str, model: &str) -> Result<Self, ProviderError>
```

**After PROV-016:** Creates `RefreshingCodexClient` in ApiKey pass-through mode. Passes as `.http_client()` to builder.

## 5. create_rig_agent() — line 219

```rust
pub fn create_rig_agent(&self, session_id: uuid::Uuid, preamble: Option<&str>, _thinking_config: Option<serde_json::Value>)
    -> rig::agent::Agent<openai::completion::CompletionModel>
```

**After PROV-016:** Return type becomes `rig::agent::Agent<openai::completion::CompletionModel<RefreshingCodexClient>>`.

## 6. Token refresh function (to call)

**File:** `codelet/providers/src/codex/codex_oauth.rs` — line 297

```rust
pub async fn refresh_access_token_at(issuer_url: &str, refresh_token: &str) -> Result<TokenRefreshResponse>
```

Testable with wiremock (issuer_url parameter).

## 7. TokenRefreshResponse (to consume)

**File:** `codelet/providers/src/codex/codex_oauth.rs` — line 246

```rust
pub struct TokenRefreshResponse {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub expires_in: Option<u64>,   // None → default 3600s per Rule [12]
}
```

## 8. URL rewrite function (to call)

**File:** `codelet/providers/src/codex/codex_oauth.rs` — line 219

```rust
pub fn rewrite_codex_url(url: &str) -> String
```

Checks for `/v1/responses` or `/chat/completions` in URL → rewrites to `CODEX_API_ENDPOINT`.

## 9. Auth persistence (to call)

**File:** `codelet/providers/src/codex/codex_auth.rs` — line 126

```rust
pub fn write_codex_auth(auth: &CodexAuthJson) -> Result<()>
```

Note: This is a sync function doing file I/O. Will be called from within the async refresh path (inside `tokio::sync::RwLock` write guard). Since it's just file write (not async), this is fine — no `.await` needed.

## 10. ProviderManager.get_codex()

**File:** `codelet/providers/src/manager.rs` — line 416

```rust
pub fn get_codex(&self) -> Result<CodexProvider, ProviderError>
```

Calls `CodexProvider::new()`. No changes needed here — the internal type change is transparent.

## 11. ClientBuilder.http_client()

**File:** `codelet/patches/rig-core/src/client/mod.rs` — line 523

```rust
pub fn http_client<U>(self, http_client: U) -> ClientBuilder<Ext, ApiKey, U>
```

Changes the generic `H` of the builder. Used to pass `RefreshingCodexClient` as the HTTP backend.

## 12. ClientBuilder.build() constraint

**File:** `codelet/patches/rig-core/src/client/mod.rs` — line 559-592

```rust
impl<Ext, ExtBuilder, Key, H> ClientBuilder<ExtBuilder, Key, H>
where
    ExtBuilder: Clone + ProviderBuilder<Output = Ext, ApiKey = Key> + Default,
    Ext: Provider<Builder = ExtBuilder>,
    Key: ApiKey,
    H: Default,              // ← RefreshingCodexClient MUST implement Default
```

**Critical:** `H: Default` is required by `build()`. `RefreshingCodexClient` must implement `Default`.

## Required trait implementations for RefreshingCodexClient

Based on type chain analysis:
- `Clone` — required by `CompletionModel<T>` derive
- `Default` — required by `ClientBuilder.build()` (line 564)
- `Debug` — required by `CompletionModel::new()`
- `Send + Sync` — required by `WasmCompatSend + WasmCompatSync`
- `HttpClientExt` — required for actual HTTP calls
- `'static` — required by `send()` futures

## New file to create

**File:** `codelet/providers/src/codex/refreshing_client.rs`

Contains:
- `TokenState` struct (access_token, refresh_token, account_id, expires_at)
- `TokenMode` enum (OAuth vs ApiKey)
- `RefreshingCodexClient` struct (wraps reqwest::Client)
- `impl HttpClientExt for RefreshingCodexClient`
- Helper methods for token expiry check, refresh, URL rewrite, header injection
