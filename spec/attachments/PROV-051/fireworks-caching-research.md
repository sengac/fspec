# Fireworks.ai Prompt Caching Research

## Research Date: 2026-03-28

## Context

Investigation into how prompt caching works with Fireworks.ai (`https://app.fireworks.ai/models/fireworks/kimi-k2p5`) when accessed via codelet's OpenAI-compatible provider, and what changes are needed to maximize cache efficiency.

---

## Findings

### Three Layers of "Caching" in the Codebase

#### 1. Request-side `cache_control` (Anthropic-specific — NOT used for OpenAI provider)

`caching_client.rs` transforms request bodies to add `cache_control: {"type": "ephemeral"}` metadata blocks. This is **Anthropic-only**. The `should_transform_request()` function explicitly checks for `api.anthropic.com` and skips everything else. The OpenAI provider's `supports_caching()` returns `false`, which means this request-side transform is never applied for OpenAI-compatible endpoints. **This is correct — Fireworks doesn't use cache_control blocks.**

#### 2. Server-side Automatic Caching (Fireworks does this)

Per Fireworks docs (https://docs.fireworks.ai/guides/prompt-caching):
- **Enabled by default** for all models and deployments
- **Automatic** — works on exact prefix matches (no request-side annotations needed)
- **50% cheaper** for cached tokens on serverless
- **Cache lifetime**: several minutes to hours, LRU eviction
- **Cache scope**: Separate per organization for serverless, per deployment for dedicated

#### 3. Response-side Usage Reporting (Already handled)

The rig-core OpenAI completion code already deserializes cache tokens:

```rust
// In openai::completion::Usage (rig-core)
pub struct PromptTokensDetails {
    pub cached_tokens: usize,  // <-- This field exists
}

// Non-streaming path (completion/mod.rs line 813-815)
if let Some(details) = &usage.prompt_tokens_details {
    u.cache_read_input_tokens = Some(details.cached_tokens as u64);
}

// Streaming path (completion/streaming.rs line 84-86)
if let Some(details) = &self.usage.prompt_tokens_details {
    usage.cache_read_input_tokens = Some(details.cached_tokens as u64);
}
```

If Fireworks returns `prompt_tokens_details.cached_tokens` in their OpenAI-compatible response, the code already captures it.

---

### The Key Gap: `x-session-affinity` Header

Fireworks caching **only works within a single replica**. On serverless (multi-replica), requests may hit different replicas and miss cache. Fireworks provides two mechanisms for session affinity:

#### Option A: `x-session-affinity` HTTP header (recommended)
```
x-session-affinity: session-id-123
```

#### Option B: `user` field in request body
```json
{"user": "session-id-123", ...}
```

**Currently `OpenAIProvider::from_api_key_with_options()` does NOT set either.**

---

### How to Add the Header via rig-core

The rig-core `ClientBuilder` supports custom headers via `http_headers()`:

```rust
// client/mod.rs line 533-536
pub fn http_headers(self, headers: HeaderMap) -> Self {
    Self { headers, ..self }
}
```

These headers are applied to **every request** from that client (line 354-355):
```rust
if let Some(hs) = req.headers_mut() {
    hs.extend(self.headers.iter().map(|(k, v)| (k.clone(), v.clone())));
}
```

Implementation approach:
```rust
use http::{HeaderMap, HeaderName, HeaderValue};

let mut headers = HeaderMap::new();
headers.insert(
    HeaderName::from_static("x-session-affinity"),
    HeaderValue::from_str(&session_id_string).unwrap(),
);

let rig_client = openai::CompletionsClient::builder()
    .api_key(api_key)
    .base_url(url)
    .http_headers(headers)
    .build()?;
```

### Alternative: `user` field via `additional_params`

For the `user` field approach (doesn't require header manipulation):
```rust
builder = builder.additional_params(serde_json::json!({
    "user": session_id
}));
```

However, `additional_params` is set per-request via `CompletionRequestBuilder`, not per-client. This would need to be injected at the `create_rig_agent` or streaming level.

---

## Status Summary

| Aspect | Status | Action Needed? |
|--------|--------|----------------|
| **Caching works** | ✅ Automatic on Fireworks | No |
| **Response reporting** | ✅ `prompt_tokens_details.cached_tokens` deserialized | No |
| **`supports_caching()` returns false** | ⚠️ Technically inaccurate for Fireworks | Consider making configurable |
| **`x-session-affinity` header** | ❌ Not sent | **Yes — significantly improves cache hit rate** |
| **`cache_control` request transforms** | ✅ Correctly skipped for non-Anthropic | No |

---

## Key Files

- `codelet/providers/src/openai.rs` — OpenAI provider, `from_api_key_with_options()`, `create_rig_agent()`
- `codelet/patches/rig-core/src/providers/openai/client.rs` — rig OpenAI client builder
- `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` — `Usage`, `PromptTokensDetails`, `CompletionRequest`
- `codelet/patches/rig-core/src/providers/openai/completion/streaming.rs` — Streaming usage extraction
- `codelet/patches/rig-core/src/client/mod.rs` — `ClientBuilder::http_headers()`, header propagation
- `codelet/providers/src/caching_client.rs` — Anthropic-specific cache_control transforms
- `codelet/providers/src/cache_token_extractor.rs` — Anthropic SSE cache token extraction
- `codelet/providers/src/lib.rs` — `LlmProvider` trait, `supports_caching()`
- `codelet/providers/src/manager.rs` — Provider manager, `get_openai()`

---

## Fireworks Prompt Caching Best Practices (from docs)

1. **Prefix stability is critical** — even a single-token change invalidates cache from that point
2. **Static content first, dynamic content last** — system prompts, tool definitions, then conversation
3. **No timestamps at start of prompts** — kills cache hit rates
4. **Session affinity required for serverless** — use `x-session-affinity` header or `user` field
5. **Tools are part of the prompt** — tool definitions count as prompt prefix for caching
