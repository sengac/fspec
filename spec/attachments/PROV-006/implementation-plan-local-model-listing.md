# PROV-006 Implementation Plan: Local Model Listing

## Status: TESTING → Ready for IMPLEMENTING

**Date**: 2026-02-23  
**Work Unit**: PROV-006 - OpenAI-Compatible Local Model Support (vLLM, Ollama)

---

## Problem Summary

The original implementation correctly handled:
- ✅ Custom base URL for API calls (`OPENAI_BASE_URL`)
- ✅ Configurable context window (`OPENAI_CONTEXT_WINDOW`)
- ✅ Configurable max output tokens (`OPENAI_MAX_OUTPUT_TOKENS`)
- ✅ Any non-empty API key accepted for local servers
- ✅ Tool calling and streaming work with local models

**But failed to implement Business Rule #2:**
> Model list must be fetched from local endpoint (GET /v1/models) NOT from models.dev

The `ModelCache` always fetches from `https://models.dev/api.json`. There is **no code path** to fetch models from a local server's `/v1/models` endpoint.

---

## Implementation Tasks

### Task 1: Add `OpenAIProvider::list_local_models()` Method

**File**: `codelet/providers/src/openai.rs`

**Signature**:
```rust
impl OpenAIProvider {
    /// Fetch available models from a local OpenAI-compatible server
    ///
    /// Makes HTTP GET request to {base_url}/v1/models and parses the response.
    /// Returns a list of model IDs available on the server.
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the local server (e.g., "http://localhost:8888")
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of model IDs
    /// * `Err(ProviderError)` - If server is unreachable or response is invalid
    pub async fn list_local_models(base_url: &str) -> Result<Vec<String>, ProviderError> {
        // Implementation here
    }
}
```

**Implementation Details**:
1. Use `reqwest::Client` with 5-second timeout
2. Make GET request to `{base_url}/v1/models`
3. Parse JSON response:
   ```json
   {
     "object": "list",
     "data": [
       {"id": "Qwen/Qwen3-80B", "object": "model", "created": 1234567890, "owned_by": "vllm"},
       {"id": "mistral-7b", "object": "model", "created": 1234567891, "owned_by": "vllm"}
     ]
   }
   ```
4. Extract `id` from each item in `data` array
5. Return `Vec<String>` of model IDs
6. On error, include base_url in error message

**Test File**: `codelet/providers/tests/openai_local_provider_test.rs`
- `fetch_model_list_from_local_server::test_list_local_models_makes_http_request`
- `local_model_listing_handles_unreachable_server::test_list_local_models_unreachable_server`

---

### Task 2: Add NAPI Binding `models_list_local_openai()`

**File**: `codelet/napi/src/models.rs`

**Signature**:
```rust
/// List models from a local OpenAI-compatible server
///
/// Makes HTTP GET request to {base_url}/v1/models endpoint.
/// Used by TUI when OPENAI_BASE_URL is set.
///
/// # Arguments
/// * `base_url` - The base URL of the local server (e.g., "http://localhost:8888")
///
/// # Returns
/// Array of model ID strings
#[napi]
pub async fn models_list_local_openai(base_url: String) -> Result<Vec<String>> {
    OpenAIProvider::list_local_models(&base_url)
        .await
        .map_err(|e| Error::from_reason(format!("Failed to list local models: {}", e)))
}
```

**Dependencies**:
- Add `codelet_providers::OpenAIProvider` to imports in `napi/src/models.rs`

**Test File**: `codelet/napi/tests/local_model_listing_napi_test.rs`
- `napi_binding_exposes_local_model_listing::test_models_list_local_openai_function`

---

### Task 3: Export from NAPI Index

**File**: `codelet/napi/src/lib.rs`

Add to exports:
```rust
pub use models::models_list_local_openai;
```

**TypeScript Declaration** (auto-generated):
```typescript
export function modelsListLocalOpenai(baseUrl: string): Promise<string[]>
```

---

### Task 4: Wire Up TUI Integration (Documentation Only)

The TUI (TypeScript) needs to:
1. Check if `OPENAI_BASE_URL` environment variable is set
2. If set, call `modelsListLocalOpenai(baseUrl)` instead of `modelsListAll()`
3. Display returned model IDs directly (no capability badges - local servers don't expose capability info)

**Note**: TUI changes are out of scope for PROV-006. Document as follow-up work or separate story.

---

## Test Verification Checklist

Before moving to IMPLEMENTING, verify tests exist for:

| Scenario | Test File | Test Function |
|----------|-----------|---------------|
| Fetch model list from local server | `openai_local_provider_test.rs` | `test_list_local_models_makes_http_request` |
| NAPI binding exposes local model listing | `local_model_listing_napi_test.rs` | `test_models_list_local_openai_function` |
| Local model listing handles unreachable server | `openai_local_provider_test.rs` | `test_list_local_models_unreachable_server` |

All tests should currently **FAIL** with:
```
error[E0599]: no function or associated item named `list_local_models` 
              found for struct `OpenAIProvider`
```

---

## Dependencies

### Crate Dependencies (already in Cargo.toml)
- `reqwest` - HTTP client (already used by ModelCache)
- `serde_json` - JSON parsing (already used)
- `tokio` - Async runtime (already used)

### Test Dependencies
- `wiremock` - Mock HTTP server for tests
- `serial_test` - Serial test execution

Check if `wiremock` is in dev-dependencies:
```bash
grep wiremock codelet/providers/Cargo.toml
```

If not, add:
```toml
[dev-dependencies]
wiremock = "0.6"
```

---

## Coverage Mapping

After implementation, link coverage:

```bash
# Task 1: OpenAIProvider::list_local_models
fspec link-coverage openai-compatible-local-model-support-vllm-ollama \
  --scenario "Fetch model list from local server via OpenAIProvider" \
  --test-file codelet/providers/tests/openai_local_provider_test.rs \
  --test-lines <TBD> \
  --impl-file codelet/providers/src/openai.rs \
  --impl-lines <TBD>

# Task 1: Error handling
fspec link-coverage openai-compatible-local-model-support-vllm-ollama \
  --scenario "Local model listing handles unreachable server" \
  --test-file codelet/providers/tests/openai_local_provider_test.rs \
  --test-lines <TBD> \
  --impl-file codelet/providers/src/openai.rs \
  --impl-lines <TBD>

# Task 2: NAPI binding
fspec link-coverage openai-compatible-local-model-support-vllm-ollama \
  --scenario "NAPI binding exposes local model listing to TUI" \
  --test-file codelet/napi/tests/local_model_listing_napi_test.rs \
  --test-lines <TBD> \
  --impl-file codelet/napi/src/models.rs \
  --impl-lines <TBD>
```

---

## Estimated Effort

| Task | Effort |
|------|--------|
| Task 1: `list_local_models()` | 30 min |
| Task 2: NAPI binding | 15 min |
| Task 3: Export | 5 min |
| Test verification & coverage linking | 20 min |
| **Total** | ~1.5 hours |

---

## ACDD Workflow Next Steps

1. **Move to IMPLEMENTING**:
   ```bash
   fspec update-work-unit-status PROV-006 implementing
   ```

2. **Implement Task 1** - Make provider test pass

3. **Implement Task 2** - Make NAPI test pass

4. **Verify all tests pass**:
   ```bash
   cd codelet && cargo test -p codelet-providers openai_local
   cd codelet && cargo test -p codelet-napi local_model
   ```

5. **Link coverage** with actual line numbers

6. **Move to VALIDATING**:
   ```bash
   fspec update-work-unit-status PROV-006 validating
   fspec show-coverage openai-compatible-local-model-support-vllm-ollama
   ```

7. **Move to DONE** when 100% coverage achieved
