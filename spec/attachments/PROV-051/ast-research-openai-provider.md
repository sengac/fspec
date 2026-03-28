# AST Research: OpenAI Provider Session Affinity Integration Points

## Date: 2026-03-28

## Research Summary

### 1. OpenAIProvider struct (openai.rs:59)
```rust
pub struct OpenAIProvider {
    completion_model: openai::completion::CompletionModel,
    rig_client: openai::CompletionsClient,
    model_name: String,
    base_url: Option<String>,
    context_window: usize,
    max_output_tokens: usize,
}
```
- No session_id field currently stored
- `base_url: Option<String>` already tracks whether custom endpoint is used

### 2. from_api_key_with_options (openai.rs:130)
```
pub fn from_api_key_with_options(api_key, model, base_url) -> Result<Self, ProviderError>
```
- Current signature: `(api_key: &str, model: &str, base_url: Option<&str>)`
- Builds rig client with `.api_key()` and optional `.base_url()`
- **Gap**: Does NOT call `.http_headers()` — this is where session affinity header goes

### 3. get_openai (manager.rs:415)
```
pub fn get_openai(&self) -> Result<OpenAIProvider, ProviderError>
```
- Takes no arguments — needs session_id parameter added
- Called from 3 sites: cli/lib.rs:148, cli/interactive_helpers.rs:356, napi/deep_search_handler.rs:392

### 4. run_with_provider! macro (session_manager.rs:4174)
```
macro_rules! run_with_provider {
    ($inner:expr, $getter:ident, $input:expr, $images:expr, $session:expr, ...) => {
        match $inner.provider_manager_mut().$getter() {
```
- Calls `$getter()` with no args — needs to pass session_id
- `$session.id` (uuid::Uuid) is already available in scope

### 5. rig-core http_headers (patches/rig-core/src/client/mod.rs)
```
pub fn http_headers(self, headers: HeaderMap) -> Self
```
- Sets custom headers on all requests from the client
- Applied in request execution via `hs.extend(self.headers.iter()...)`
- Supports HeaderMap from http crate

## Integration Plan
1. Add `session_id: Option<uuid::Uuid>` to `from_api_key_with_options()`
2. When `base_url` is Some AND session_id is Some, build HeaderMap with `x-session-affinity`
3. Check `OPENAI_SESSION_AFFINITY` env var for override value
4. Add `session_id: uuid::Uuid` to `get_openai()` signature
5. Update macro to pass `$session.id` to getter
6. Update CLI call sites to pass session_id
