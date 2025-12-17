# Custom HTTP Middleware for Anthropic Prompt Cache Control

## Overview

This document details the implementation plan for Option B: creating a custom HTTP client middleware layer that intercepts requests to Anthropic's `/v1/messages` endpoint and transforms the request body to enable prompt caching.

## Problem Statement

The `rig` library (v0.25.0+) sends system prompts as plain strings to Anthropic's API:

```json
{
  "system": "You are a helpful assistant...",
  "messages": [...]
}
```

However, Anthropic's prompt caching requires the system field to be an **array of content blocks** with `cache_control` metadata:

```json
{
  "system": [
    {
      "type": "text",
      "text": "You are Claude Code, Anthropic's official CLI...",
    },
    {
      "type": "text",
      "text": "Additional instructions...",
      "cache_control": { "type": "ephemeral" }
    }
  ],
  "messages": [...]
}
```

Additionally, rig's streaming implementation discards `cache_read_input_tokens` and `cache_creation_input_tokens` from Anthropic's usage response, preventing cache effectiveness measurement.

## Solution Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────────┐
│                      ClaudeProvider                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌─────────────────────┐    ┌────────────┐ │
│  │ rig::Client  │───▶│ CachingHttpClient   │───▶│ Anthropic  │ │
│  │              │    │ (Custom Middleware) │    │ API        │ │
│  └──────────────┘    └─────────────────────┘    └────────────┘ │
│                              │                         │        │
│                              ▼                         ▼        │
│                    ┌─────────────────┐      ┌────────────────┐ │
│                    │ Transform       │      │ Extract Cache  │ │
│                    │ Request Body    │      │ Tokens from    │ │
│                    │ (system→array)  │      │ Response       │ │
│                    └─────────────────┘      └────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Components

1. **CachingHttpClient** - Custom `reqwest::Client` wrapper with middleware
2. **RequestTransformer** - Transforms request body for cache control
3. **ResponseExtractor** - Extracts cache tokens from SSE response
4. **Integration Layer** - Wires custom client into rig's client builder

## Implementation Plan

### Phase 1: Custom HTTP Client Wrapper

#### 1.1 Create `CachingHttpClient` struct

**File**: `providers/src/caching_client.rs`

```rust
use reqwest::{Client, Request, Response};
use serde_json::Value;

/// Custom HTTP client that intercepts Anthropic API requests
/// to enable prompt caching by transforming request bodies.
pub struct CachingHttpClient {
    inner: Client,
    is_oauth: bool,
    oauth_prefix: Option<String>,
}

impl CachingHttpClient {
    pub fn new(inner: Client, is_oauth: bool, oauth_prefix: Option<String>) -> Self {
        Self { inner, is_oauth, oauth_prefix }
    }

    /// Intercept and potentially transform the request
    pub async fn execute(&self, mut request: Request) -> Result<Response, reqwest::Error> {
        // Only transform requests to /v1/messages
        if self.should_transform(&request) {
            request = self.transform_request(request).await?;
        }
        self.inner.execute(request).await
    }

    fn should_transform(&self, request: &Request) -> bool {
        request.url().path().contains("/v1/messages")
    }
}
```

#### 1.2 Implement Request Body Transformation

```rust
impl CachingHttpClient {
    async fn transform_request(&self, request: Request) -> Result<Request, reqwest::Error> {
        // Extract body
        let body = request.body()
            .and_then(|b| b.as_bytes())
            .map(|b| String::from_utf8_lossy(b).to_string());

        if let Some(body_str) = body {
            if let Ok(mut json) = serde_json::from_str::<Value>(&body_str) {
                // Transform system field from string to array with cache_control
                if let Some(system) = json.get("system") {
                    if system.is_string() {
                        let system_str = system.as_str().unwrap();
                        let transformed = self.build_cached_system(system_str);
                        json["system"] = transformed;
                    }
                }

                // Add cache_control to first user message content
                self.add_message_cache_control(&mut json);

                // Rebuild request with transformed body
                return self.rebuild_request(request, json);
            }
        }

        Ok(request)
    }

    fn build_cached_system(&self, system_text: &str) -> Value {
        if self.is_oauth {
            let prefix = self.oauth_prefix.as_deref()
                .unwrap_or("You are Claude Code, Anthropic's official CLI for Claude.");

            // OAuth: first block without cache_control, second with
            serde_json::json!([
                {
                    "type": "text",
                    "text": prefix
                },
                {
                    "type": "text",
                    "text": system_text.strip_prefix(prefix).unwrap_or(system_text).trim(),
                    "cache_control": { "type": "ephemeral" }
                }
            ])
        } else {
            // API key: single block with cache_control
            serde_json::json!([
                {
                    "type": "text",
                    "text": system_text,
                    "cache_control": { "type": "ephemeral" }
                }
            ])
        }
    }

    fn add_message_cache_control(&self, json: &mut Value) {
        // Add cache_control to first user message (often contains large context)
        if let Some(messages) = json.get_mut("messages").and_then(|m| m.as_array_mut()) {
            for (i, msg) in messages.iter_mut().enumerate() {
                if i == 0 && msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                    // Transform string content to array with cache_control
                    if let Some(content) = msg.get("content") {
                        if content.is_string() {
                            let text = content.as_str().unwrap();
                            msg["content"] = serde_json::json!([
                                {
                                    "type": "text",
                                    "text": text,
                                    "cache_control": { "type": "ephemeral" }
                                }
                            ]);
                        }
                    }
                    break;
                }
            }
        }
    }
}
```

### Phase 2: Cache Token Extraction from SSE

#### 2.1 Create Response Interceptor

**File**: `providers/src/cache_token_extractor.rs`

```rust
use serde::Deserialize;

/// Anthropic SSE MessageStart event with full usage including cache tokens
#[derive(Debug, Deserialize)]
pub struct MessageStartEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub message: MessageStartPayload,
}

#[derive(Debug, Deserialize)]
pub struct MessageStartPayload {
    pub usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u64,
    pub output_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}

/// Extract cache tokens from SSE stream
pub struct CacheTokenExtractor {
    cache_read_tokens: Option<u64>,
    cache_creation_tokens: Option<u64>,
}

impl CacheTokenExtractor {
    pub fn new() -> Self {
        Self {
            cache_read_tokens: None,
            cache_creation_tokens: None,
        }
    }

    /// Process SSE line and extract cache tokens if present
    pub fn process_sse_line(&mut self, line: &str) {
        // SSE format: data: {...}
        if let Some(json_str) = line.strip_prefix("data: ") {
            if let Ok(event) = serde_json::from_str::<MessageStartEvent>(json_str) {
                if event.event_type == "message_start" {
                    self.cache_read_tokens = event.message.usage.cache_read_input_tokens;
                    self.cache_creation_tokens = event.message.usage.cache_creation_input_tokens;
                }
            }
        }
    }

    /// Get extracted cache read tokens
    pub fn cache_read_tokens(&self) -> Option<u64> {
        self.cache_read_tokens
    }

    /// Get extracted cache creation tokens
    pub fn cache_creation_tokens(&self) -> Option<u64> {
        self.cache_creation_tokens
    }
}
```

#### 2.2 Streaming Response Wrapper

```rust
use futures::Stream;
use tokio::io::AsyncBufReadExt;

/// Wrapper that intercepts SSE stream to extract cache tokens
/// while passing through to rig's stream processor
pub struct CachingStreamWrapper<S> {
    inner: S,
    extractor: CacheTokenExtractor,
}

impl<S> CachingStreamWrapper<S>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            extractor: CacheTokenExtractor::new(),
        }
    }

    pub fn get_cache_tokens(&self) -> (Option<u64>, Option<u64>) {
        (
            self.extractor.cache_read_tokens(),
            self.extractor.cache_creation_tokens(),
        )
    }
}

// Implement Stream to pass through while extracting cache tokens
impl<S> Stream for CachingStreamWrapper<S>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<bytes::Bytes, reqwest::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match std::pin::Pin::new(&mut self.inner).poll_next(cx) {
            std::task::Poll::Ready(Some(Ok(bytes))) => {
                // Extract cache tokens from SSE data
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    for line in text.lines() {
                        self.extractor.process_sse_line(line);
                    }
                }
                std::task::Poll::Ready(Some(Ok(bytes)))
            }
            other => other,
        }
    }
}
```

### Phase 3: Integration with rig

#### 3.1 Modify ClaudeProvider to use Custom Client

**File**: `providers/src/claude.rs` (modifications)

```rust
use crate::caching_client::CachingHttpClient;

impl ClaudeProvider {
    /// Create a new ClaudeProvider with caching middleware
    pub fn new_with_caching() -> Result<Self> {
        let (api_key, auth_mode) = Self::detect_credentials()?;

        // Build base reqwest client
        let base_client = reqwest::Client::builder()
            .default_headers(Self::build_headers(&api_key, auth_mode))
            .build()?;

        // Wrap with caching middleware
        let caching_client = CachingHttpClient::new(
            base_client,
            auth_mode == AuthMode::OAuth,
            Some(CLAUDE_CODE_PROMPT_PREFIX.to_string()),
        );

        // Create rig client with custom HTTP client
        let rig_client = anthropic::Client::from_parts(
            "https://api.anthropic.com".to_string(),
            Self::build_rig_headers(&api_key, auth_mode),
            caching_client.into_reqwest_client(), // Need adapter
            Default::default(),
        );

        // ... rest of initialization
    }
}
```

#### 3.2 Create reqwest Client Adapter

```rust
/// Adapter to make CachingHttpClient usable where reqwest::Client is expected
impl CachingHttpClient {
    /// Convert to a reqwest::Client with middleware
    ///
    /// NOTE: This requires using reqwest-middleware crate or similar
    /// to create a client that intercepts requests
    pub fn into_reqwest_client(self) -> reqwest::Client {
        // Option 1: Use reqwest-middleware crate
        // Option 2: Create custom Service implementation
        // Option 3: Monkey-patch at runtime (not recommended)

        // For now, we may need to use a different approach - see Alternative below
        todo!("Implement adapter")
    }
}
```

### Alternative Approach: Override at rig Level

Since rig's `Client::from_parts()` accepts a `reqwest::Client`, we need a way to intercept at the request level. Options:

#### Option A: reqwest-middleware crate

```rust
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_middleware::Middleware;

struct CacheControlMiddleware {
    is_oauth: bool,
    oauth_prefix: Option<String>,
}

#[async_trait::async_trait]
impl Middleware for CacheControlMiddleware {
    async fn handle(
        &self,
        req: reqwest::Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        let transformed = self.transform_if_needed(req).await?;
        next.run(transformed, extensions).await
    }
}
```

#### Option B: Custom reqwest Client (Tower Service)

Use `tower` service pattern to wrap the HTTP client:

```rust
use tower::Service;
use http::{Request, Response};

struct CachingService<S> {
    inner: S,
    transformer: RequestTransformer,
}

impl<S> Service<Request<Body>> for CachingService<S>
where
    S: Service<Request<Body>, Response = Response<Body>>,
{
    // ... implement Service trait
}
```

#### Option C: Fork/Patch rig (Last Resort)

If no clean middleware solution works, may need to:
1. Fork rig crate
2. Modify `AnthropicCompletionRequest.system` to be `Value` instead of `String`
3. Add cache token fields to `PartialUsage`
4. Maintain fork or submit upstream PR

### Phase 4: Wire Cache Tokens to TokenTracker

#### 4.1 Update interactive.rs

```rust
// In run_agent_stream_with_interruption()

// After stream completes, extract cache tokens from wrapper
let (cache_read, cache_creation) = stream_wrapper.get_cache_tokens();

// Update session token tracker
if let Some(read) = cache_read {
    let current = session.token_tracker.cache_read_input_tokens.unwrap_or(0);
    session.token_tracker.cache_read_input_tokens = Some(current + read);
}
if let Some(create) = cache_creation {
    let current = session.token_tracker.cache_creation_input_tokens.unwrap_or(0);
    session.token_tracker.cache_creation_input_tokens = Some(current + create);
}
```

## File Structure

```
providers/
├── src/
│   ├── lib.rs                    # Add module exports
│   ├── claude.rs                 # Modify to use caching client
│   ├── caching_client.rs         # NEW: Custom HTTP client wrapper
│   ├── cache_token_extractor.rs  # NEW: SSE cache token extraction
│   └── request_transformer.rs    # NEW: Request body transformation
```

## Dependencies

Add to `providers/Cargo.toml`:

```toml
[dependencies]
# Existing
reqwest = { version = "0.12", features = ["json", "stream"] }
serde_json = "1.0"
futures = "0.3"
tokio = { version = "1", features = ["io-util"] }

# New - for middleware support
reqwest-middleware = "0.3"  # Optional: cleaner middleware pattern
tower = "0.4"               # Optional: service-based approach
```

## Testing Strategy

### Unit Tests

1. **Request transformation tests** (`tests/request_transform_test.rs`)
   - Test system string → array transformation
   - Test OAuth vs API key mode
   - Test message cache_control injection

2. **Cache token extraction tests** (`tests/cache_token_extraction_test.rs`)
   - Test SSE parsing
   - Test message_start event handling
   - Test missing cache fields (non-Anthropic providers)

### Integration Tests

1. **End-to-end caching test**
   - Make API call with caching enabled
   - Verify `cache_read_input_tokens` extracted
   - Verify `effective_tokens()` calculation

2. **Compaction with cache discount**
   - Trigger compaction threshold
   - Verify cache discount affects timing

## Acceptance Criteria

1. [ ] System prompts are sent as array format with `cache_control: { type: "ephemeral" }`
2. [ ] First user message content includes `cache_control` metadata
3. [ ] `cache_read_input_tokens` is extracted from Anthropic SSE response
4. [ ] `cache_creation_input_tokens` is extracted from Anthropic SSE response
5. [ ] `TokenTracker.effective_tokens()` returns discounted value when cache is active
6. [ ] Existing rig functionality (tools, streaming, multi-turn) continues to work
7. [ ] OAuth and API key modes both work correctly
8. [ ] Non-Anthropic providers (OpenAI, Codex) are unaffected

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| rig internal changes break middleware | Pin rig version, add integration tests |
| Performance overhead from body parsing | Only parse /v1/messages requests |
| Middleware approach doesn't work with rig | Fall back to Option C (fork) |
| SSE format changes | Use defensive parsing, log unknown events |

## References

- [Anthropic Prompt Caching Docs](https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching)
- [TypeScript codelet implementation](~/projects/codelet/src/agent/claude-provider.ts)
- [rig crate source](https://github.com/0xPlaygrounds/rig)
- [reqwest-middleware crate](https://docs.rs/reqwest-middleware)
- Existing blocked feature: `spec/features/anthropic-cache-control.feature`
