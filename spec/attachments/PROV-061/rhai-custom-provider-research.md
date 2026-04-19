# Rhai-Scriptable Custom Provider Type: Research & Architecture

**Date:** 2026-04-17
**Work Unit:** PROV-061
**Depends on:** PROV-060 (Shared OAuth Building Blocks + Rhai Scripting)
**Research session:** 359b5d06-a6b4-41d1-a487-1dfdf37713e4

---

## Executive Summary

This document details the architecture for a fully Rhai-scriptable "custom" provider type that allows users to create providers for **any LLM API** without recompiling. A `.rhai` script + JSON config controls every aspect of the provider lifecycle: endpoint URLs, HTTP headers, request body construction, response deserialization, stop reason mapping, tool call extraction, streaming SSE chunk parsing, and error handling.

This builds on PROV-060's shared OAuth building blocks and Rhai engine infrastructure.

---

## 1. Current Provider Integration Points

Every provider in the codebase must implement **7 integration points**. A custom provider must make all 7 scriptable:

| # | Integration Point | What It Does | Current Lines Per Provider |
|---|-------------------|-------------|---------------------------|
| 1 | **Request Building** | Convert `CompletionRequest` → provider-specific JSON body | ~100-200 |
| 2 | **URL Construction** | Build the full endpoint URL (base + path + query params) | ~20-50 |
| 3 | **Header Injection** | Set auth, API version, custom headers per request | ~50-150 |
| 4 | **Response Parsing** | Deserialize JSON → extract text, tool calls, usage | ~80-150 |
| 5 | **Stop Reason Mapping** | Map provider-specific stop reasons → `StopReason` enum | ~20-30 |
| 6 | **Streaming SSE** | Parse SSE chunks → incremental text/tool deltas | ~150-300 |
| 7 | **Error Handling** | Map HTTP status codes + error bodies → `ProviderError` | ~30-50 |

**Total per provider: ~450-930 lines of Rust.** The goal is to replace all of this with a single Rhai script per custom provider.

---

## 2. The `LlmProvider` Trait (What Must Be Implemented)

The core trait every provider implements (`codelet/providers/src/lib.rs`):

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
    fn supports_caching(&self) -> bool;
    fn supports_streaming(&self) -> bool;

    async fn complete(&self, messages: &[Message]) -> Result<String, ProviderError>;
    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse, ProviderError>;
}
```

Plus rig's `CompletionModel` trait:

```rust
pub trait CompletionModel: Clone + Send + Sync {
    type Response: Serialize + DeserializeOwned + Send + Sync;

    async fn completion(&self, request: CompletionRequest) -> Result<CompletionResponse<Self::Response>>;
    async fn stream(&self, request: CompletionRequest) -> Result<StreamingCompletionResponse<Self::StreamingResponse>>;
}
```

---

## 3. Architecture: The `RhaiCustomProvider`

### 3.1 File Layout

```
codelet/providers/src/
  custom/
    ├── mod.rs                 — RhaiCustomProvider (LlmProvider impl)
    ├── completion_model.rs    — RhaiCompletionModel (CompletionModel impl)
    ├── http_client.rs         — RhaiHttpClient (HttpClientExt impl)
    ├── config.rs              — ProviderConfig JSON schema + loader
    ├── script_loader.rs       — Compile + cache .rhai ASTs
    ├── request_bridge.rs      — CompletionRequest ↔ Rhai Dynamic conversion
    ├── response_bridge.rs     — Rhai Dynamic → CompletionResponse conversion
    ├── stream_bridge.rs       — SSE chunk → Rhai → StreamEvent conversion
    └── error_bridge.rs        — Rhai error → ProviderError mapping
```

### 3.2 Provider Config (JSON)

Users define their provider with a JSON config file:

```json
{
  "name": "my-llm",
  "display_name": "My Custom LLM",
  "base_url": "https://api.my-llm.com",
  "script": "my_llm.rhai",

  "auth": {
    "type": "bearer",
    "env_var": "MY_LLM_API_KEY"
  },

  "models": {
    "my-llm-large": {
      "context_window": 128000,
      "max_output_tokens": 8192,
      "supports_caching": false,
      "supports_streaming": true,
      "supports_tools": true,
      "supports_vision": false
    },
    "my-llm-small": {
      "context_window": 32000,
      "max_output_tokens": 4096
    }
  },

  "defaults": {
    "temperature": 0.7,
    "context_window": 32000,
    "max_output_tokens": 4096,
    "supports_caching": false,
    "supports_streaming": true,
    "supports_tools": true,
    "supports_vision": false
  }
}
```

**Config locations** (searched in order):
1. `~/.fspec/providers/<name>.json` — user-global
2. `.fspec/providers/<name>.json` — project-local
3. Built-in providers — compiled into binary (Claude, Codex, Copilot, etc.)


### 3.3 The Rhai Script Interface

A custom provider's `.rhai` script must define **7 required functions** (for the HTTP request/response lifecycle) and may define **up to 5 optional functions** (for system prompts and tool facades). The Rust engine calls these at the appropriate points.

#### Function 1: `build_request(config, messages, tools, options) -> Map`

Transforms the generic completion request into provider-specific JSON:

```javascript
// my_llm.rhai

fn build_request(config, messages, tools, options) {
    let body = #{};
    body.model = options.model;

    // Convert messages to provider format
    let provider_messages = [];
    for msg in messages {
        if msg.role == "system" {
            body.system = msg.content;
        } else {
            provider_messages.push(#{
                role: msg.role,
                content: format_content(msg.content)
            });
        }
    }
    body.messages = provider_messages;

    // Convert tools
    if tools.len() > 0 {
        body.tools = [];
        for tool in tools {
            body.tools.push(#{
                type: "function",
                function: #{
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters
                }
            });
        }
    }

    // Provider-specific options
    if options.max_tokens != () {
        body.max_tokens = options.max_tokens;
    }
    if options.temperature != () {
        body.temperature = options.temperature;
    }

    #{
        endpoint: "/v1/chat/completions",
        method: "POST",
        body: body
    }
}

// Helper: format content (handles text + images + tool results)
fn format_content(content) {
    if type_of(content) == "string" {
        return content;
    }
    // Array of content parts
    let parts = [];
    for part in content {
        if part.type == "text" {
            parts.push(#{ type: "text", text: part.text });
        } else if part.type == "image" {
            parts.push(#{ type: "image_url", image_url: #{ url: part.data } });
        } else if part.type == "tool_result" {
            parts.push(#{ type: "text", text: part.output });
        }
    }
    parts
}
```

#### Function 2: `build_headers(config, auth_token, request_info) -> Map`

Returns custom headers for each request:

```javascript
fn build_headers(config, auth_token, request_info) {
    let headers = #{};

    // Auth
    headers["Authorization"] = `Bearer ${auth_token}`;

    // API version
    headers["X-API-Version"] = "2024-01-01";

    // Custom headers based on request content
    if request_info.has_vision {
        headers["X-Vision-Enabled"] = "true";
    }
    if request_info.has_tools {
        headers["X-Tool-Mode"] = "auto";
    }

    // User agent
    headers["User-Agent"] = `codelet/${config.version}`;

    headers
}
```

#### Function 3: `build_url(config, endpoint, options) -> String`

Constructs the full request URL:

```javascript
fn build_url(config, endpoint, options) {
    let url = `${config.base_url}${endpoint}`;

    // Add query parameters if needed
    let params = [];
    if config.api_version != () {
        params.push(`api-version=${config.api_version}`);
    }
    if options.beta {
        params.push("beta=true");
    }

    if params.len() > 0 {
        url = `${url}?${params.join("&")}`;
    }
    url
}
```

#### Function 4: `parse_response(config, status_code, body) -> Map`

Parses the provider's response JSON into the normalized format:

```javascript
fn parse_response(config, status_code, body) {
    if status_code < 200 || status_code >= 300 {
        return #{ error: true, message: body.error.message, code: status_code };
    }

    let choice = body.choices[0];
    let content = [];

    // Extract text content
    if choice.message.content != () {
        content.push(#{ type: "text", text: choice.message.content });
    }

    // Extract tool calls
    if choice.message.tool_calls != () {
        for tc in choice.message.tool_calls {
            content.push(#{
                type: "tool_use",
                id: tc.id,
                name: tc.function.name,
                input: json::parse(tc.function.arguments)
            });
        }
    }

    // Extract thinking/reasoning (if provider supports it)
    if choice.message.reasoning != () {
        content.push(#{ type: "thinking", text: choice.message.reasoning });
    }

    // Map stop reason
    let stop_reason = map_stop_reason(choice.finish_reason);

    // Extract usage
    let usage = #{
        input_tokens: body.usage.prompt_tokens,
        output_tokens: body.usage.completion_tokens,
        cache_read_tokens: body.usage.prompt_tokens_details.cached_tokens,
        cache_creation_tokens: 0
    };

    #{ content: content, stop_reason: stop_reason, usage: usage }
}

fn map_stop_reason(reason) {
    switch reason {
        "stop" | "end_turn" => "end_turn",
        "tool_calls" | "tool_use" | "function_call" => "tool_use",
        "length" | "max_tokens" => "max_tokens",
        _ => "end_turn"
    }
}
```

#### Function 5: `parse_stream_chunk(config, event_type, data) -> Map`

Parses a single SSE event into a normalized streaming chunk:

```javascript
fn parse_stream_chunk(config, event_type, data) {
    // Handle [DONE] sentinel
    if data == "[DONE]" {
        return #{ type: "done" };
    }

    let chunk = json::parse(data);
    let delta = chunk.choices[0].delta;

    // Text delta
    if delta.content != () {
        return #{ type: "text", text: delta.content };
    }

    // Tool call delta
    if delta.tool_calls != () {
        let tc = delta.tool_calls[0];
        return #{
            type: "tool_call_delta",
            index: tc.index,
            id: tc.id,
            name: tc.function.name,
            arguments: tc.function.arguments
        };
    }

    // Reasoning delta
    if delta.reasoning != () {
        return #{ type: "thinking_delta", text: delta.reasoning };
    }

    // Usage (final chunk)
    if chunk.usage != () {
        return #{
            type: "usage",
            input_tokens: chunk.usage.prompt_tokens,
            output_tokens: chunk.usage.completion_tokens
        };
    }

    // Stop reason
    if chunk.choices[0].finish_reason != () {
        return #{
            type: "stop",
            stop_reason: map_stop_reason(chunk.choices[0].finish_reason)
        };
    }

    #{ type: "ignore" }
}
```

#### Function 6: `build_stream_request(config, messages, tools, options) -> Map`

Like `build_request` but adds streaming-specific fields:

```javascript
fn build_stream_request(config, messages, tools, options) {
    let req = build_request(config, messages, tools, options);
    req.body.stream = true;
    // Some providers need stream_options for usage in stream
    req.body.stream_options = #{ include_usage: true };
    req
}
```

#### Function 7: `map_error(config, status_code, body) -> Map`

Maps HTTP errors to typed error categories:

```javascript
fn map_error(config, status_code, body) {
    let message = if body.error != () {
        body.error.message
    } else {
        `HTTP ${status_code}`
    };

    switch status_code {
        401 | 403 => #{
            type: "authentication",
            message: message,
            retryable: false
        },
        429 => #{
            type: "rate_limit",
            message: message,
            retryable: true,
            retry_after: body.error.retry_after
        },
        408 | 504 | 502 | 503 => #{
            type: "timeout",
            message: message,
            retryable: true
        },
        _ => #{
            type: "api",
            message: message,
            retryable: false
        }
    }
}
```

---

## 4. Rust-Side Implementation

### 4.1 `RhaiCustomProvider` (LlmProvider impl)

```rust
pub struct RhaiCustomProvider {
    config: ProviderConfig,
    model_id: String,
    model_config: ModelConfig,
    engine: Arc<Engine>,
    request_ast: Arc<AST>,
    auth_token: Arc<RwLock<String>>,       // Refreshable via PROV-060 OAuth
    http_client: reqwest::Client,
}

#[async_trait]
impl LlmProvider for RhaiCustomProvider {
    fn name(&self) -> &str { &self.config.name }
    fn model(&self) -> &str { &self.model_id }
    fn context_window(&self) -> usize { self.model_config.context_window }
    fn max_output_tokens(&self) -> usize { self.model_config.max_output_tokens }
    fn supports_caching(&self) -> bool { self.model_config.supports_caching }
    fn supports_streaming(&self) -> bool { self.model_config.supports_streaming }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse, ProviderError> {
        // 1. Serialize inputs to Rhai Dynamic
        let messages_dyn = rhai::serde::to_dynamic(&messages)?;
        let tools_dyn = rhai::serde::to_dynamic(&tools)?;
        let config_dyn = rhai::serde::to_dynamic(&self.config)?;
        let options_dyn = build_options_dynamic(&self.model_id, &self.model_config);

        // 2. Call build_request in spawn_blocking (Rhai is sync)
        let engine = self.engine.clone();
        let ast = self.request_ast.clone();
        let request_result = tokio::task::spawn_blocking(move || {
            let mut scope = Scope::new();
            engine.call_fn::<Dynamic>(
                &mut scope, &ast, "build_request",
                (config_dyn, messages_dyn, tools_dyn, options_dyn)
            )
        }).await??;

        // 3. Extract endpoint + body from Rhai result
        let endpoint: String = rhai::serde::from_dynamic(&request_result["endpoint"])?;
        let body: serde_json::Value = rhai::serde::from_dynamic(&request_result["body"])?;

        // 4. Build headers via Rhai
        let headers = self.build_headers_via_rhai(&request_result).await?;

        // 5. Build URL via Rhai
        let url = self.build_url_via_rhai(&endpoint).await?;

        // 6. Send HTTP request
        let response = self.http_client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::api(&self.config.name, e.to_string()))?;

        let status = response.status().as_u16();
        let body_bytes = response.bytes().await?;
        let body_value: serde_json::Value = serde_json::from_slice(&body_bytes)?;

        // 7. Parse response via Rhai
        let parsed = self.parse_response_via_rhai(status, body_value).await?;

        // 8. Convert Rhai result → CompletionResponse
        response_bridge::to_completion_response(parsed)
    }
}
```

### 4.2 `RhaiHttpClient` (HttpClientExt impl)

For rig integration, we also need an `HttpClientExt` implementation that applies Rhai-defined headers:

```rust
pub struct RhaiHttpClient {
    inner: reqwest::Client,
    engine: Arc<Engine>,
    ast: Arc<AST>,
    config_dyn: Dynamic,
    auth_token: Arc<RwLock<String>>,
}

impl HttpClientExt for RhaiHttpClient {
    async fn send<T, U>(&self, mut req: Request<T>) -> Result<Response<U>> {
        // 1. Get current auth token
        let token = self.auth_token.read().await.clone();

        // 2. Classify request (detect vision/tools)
        let request_info = classify_request(&req);

        // 3. Build headers via Rhai (spawn_blocking)
        let engine = self.engine.clone();
        let ast = self.ast.clone();
        let config = self.config_dyn.clone();
        let info_dyn = rhai::serde::to_dynamic(&request_info)?;
        let headers = tokio::task::spawn_blocking(move || {
            let mut scope = Scope::new();
            engine.call_fn::<Dynamic>(
                &mut scope, &ast, "build_headers",
                (config, token, info_dyn)
            )
        }).await??;

        // 4. Apply headers to request
        let header_map = dynamic_to_header_map(&headers)?;
        for (key, value) in header_map {
            req.headers_mut().insert(key, value);
        }

        // 5. Delegate to inner client
        self.inner.send(req).await
    }

    // send_streaming follows the same pattern
}
```

### 4.3 Streaming Bridge

Streaming is the most complex piece. The architecture:

```
SSE bytes → eventsource_stream (existing rig infra)
  → SSE Event { event_type, data }
    → Rhai: parse_stream_chunk(config, event_type, data)
      → Normalized chunk map { type, text/tool_call/usage/... }
        → Rust: stream_bridge::to_stream_item(chunk_map)
          → MultiTurnStreamItem (rig's stream type)
```

Key design decision: **Rhai is called per-SSE-event, not per-byte**. This means:
- The SSE frame parsing (splitting `event:`, `data:` lines) stays in Rust (rig's existing infra)
- Only the JSON interpretation of each `data:` payload is scriptable
- This keeps the hot path (byte parsing) in compiled Rust
- Rhai only runs ~10-100 times per completion (once per SSE event), not thousands of times per byte

```rust
pub struct RhaiStreamProcessor {
    engine: Arc<Engine>,
    ast: Arc<AST>,
    config_dyn: Dynamic,
}

impl RhaiStreamProcessor {
    /// Process a single SSE event into a normalized stream item
    pub fn process_event(&self, event_type: &str, data: &str)
        -> Result<StreamChunk, ProviderError>
    {
        // This runs in spawn_blocking context (caller ensures)
        let mut scope = Scope::new();
        let result = self.engine.call_fn::<Dynamic>(
            &mut scope, &self.ast, "parse_stream_chunk",
            (self.config_dyn.clone(), event_type.to_string(), data.to_string())
        )?;
        stream_bridge::dynamic_to_stream_chunk(result)
    }
}
```

### 4.4 ProviderManager Integration

```rust
// In manager.rs

pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
    ZAI,
    GitHubCopilot,
    Custom(String),  // NEW: name of the custom provider
}

impl ProviderManager {
    pub fn get_custom_provider(&self, name: &str) -> Result<Box<dyn LlmProvider>> {
        // 1. Load config from ~/.fspec/providers/<name>.json
        let config = load_provider_config(name)?;

        // 2. Load + compile Rhai script
        let (engine, ast) = load_provider_script(&config)?;

        // 3. Resolve auth token
        let auth_token = resolve_auth(&config)?;

        // 4. Build provider
        Ok(Box::new(RhaiCustomProvider {
            config,
            model_id: self.selected_model.clone(),
            model_config: self.resolve_model_config(&config)?,
            engine: Arc::new(engine),
            request_ast: Arc::new(ast),
            auth_token: Arc::new(RwLock::new(auth_token)),
            http_client: reqwest::Client::new(),
        }))
    }
}
```

---

## 5. Rhai Building Block Modules

These Rust functions are registered into the Rhai engine and callable from scripts. They form the "standard library" for custom providers:

### 5.1 `http` module (sync HTTP via ureq)

```rust
// Registered as http::post, http::get, etc.
fn http_post(url: String, body: String, headers: Dynamic) -> Dynamic {
    let mut req = ureq::post(&url);
    for (k, v) in headers.as_map()? {
        req = req.set(&k, &v.to_string());
    }
    let resp = req.send_string(&body)?;
    #{ status: resp.status(), body: resp.into_string()? }
}

fn http_get(url: String, headers: Dynamic) -> Dynamic {
    let mut req = ureq::get(&url);
    for (k, v) in headers.as_map()? {
        req = req.set(&k, &v.to_string());
    }
    let resp = req.call()?;
    #{ status: resp.status(), body: resp.into_string()? }
}
```

### 5.2 `json` module

```rust
fn json_parse(s: String) -> Dynamic { /* serde_json → Rhai Dynamic */ }
fn json_stringify(val: Dynamic) -> String { /* Rhai Dynamic → JSON string */ }
fn json_stringify_pretty(val: Dynamic) -> String { /* Pretty-printed */ }
```

### 5.3 `crypto` module

```rust
fn sha256(data: String) -> String { /* hex-encoded SHA-256 */ }
fn base64_encode(data: String) -> String { /* standard base64 */ }
fn base64url_encode(data: String) -> String { /* URL-safe base64 no padding */ }
fn base64_decode(data: String) -> String { /* decode base64 */ }
fn hmac_sha256(key: String, data: String) -> String { /* HMAC-SHA256 */ }
fn random_string(length: i64) -> String { /* cryptographic random */ }
fn uuid_v4() -> String { /* random UUID */ }
```

### 5.4 `oauth` module (from PROV-060)

```rust
fn generate_pkce() -> Dynamic { /* { verifier, challenge, method } */ }
fn generate_state() -> String { /* random state parameter */ }
fn urlencode(s: String) -> String { /* URL encoding */ }
fn urldecode(s: String) -> String { /* URL decoding */ }
fn parse_urlencoded(s: String) -> Dynamic { /* form body → map */ }
```

### 5.5 `time` module

```rust
fn now() -> i64 { /* Unix timestamp seconds */ }
fn now_ms() -> i64 { /* Unix timestamp milliseconds */ }
fn format_iso8601(ts: i64) -> String { /* ISO 8601 format */ }
fn parse_iso8601(s: String) -> i64 { /* Parse to timestamp */ }
```

### 5.6 `env` module (restricted)

```rust
// Only reads specific allowed env vars (declared in provider config)
fn get_env(name: String) -> String {
    // Validates against config.auth.env_var and config.allowed_env_vars
    // Rejects anything not in the allowlist
}
```

---

## 6. Example: Complete Custom Provider for a Hypothetical API

### Config: `~/.fspec/providers/my-llm.json`

```json
{
    "name": "my-llm",
    "display_name": "My Custom LLM",
    "base_url": "https://api.my-llm.example.com",
    "script": "my_llm.rhai",
    "auth": {
        "type": "bearer",
        "env_var": "MY_LLM_API_KEY"
    },
    "models": {
        "my-llm-large": {
            "context_window": 128000,
            "max_output_tokens": 8192,
            "supports_streaming": true,
            "supports_tools": true
        }
    }
}
```

### Script: `~/.fspec/providers/my_llm.rhai`

A complete, working example showing all 7 functions:

```javascript
// my_llm.rhai — Complete custom provider for OpenAI-compatible API

fn build_request(config, messages, tools, options) {
    let body = #{
        model: options.model,
        messages: [],
        temperature: options.temperature
    };

    // Convert messages
    for msg in messages {
        let converted = #{ role: msg.role };
        if type_of(msg.content) == "string" {
            converted.content = msg.content;
        } else {
            converted.content = [];
            for part in msg.content {
                switch part.type {
                    "text" => converted.content.push(#{ type: "text", text: part.text }),
                    "image" => converted.content.push(#{
                        type: "image_url",
                        image_url: #{ url: `data:${part.media_type};base64,${part.data}` }
                    }),
                    "tool_use" => {
                        // Tool calls go in a special field
                        if converted.tool_calls == () { converted.tool_calls = []; }
                        converted.tool_calls.push(#{
                            id: part.id,
                            type: "function",
                            function: #{ name: part.name, arguments: json::stringify(part.input) }
                        });
                    },
                    "tool_result" => {
                        converted.role = "tool";
                        converted.tool_call_id = part.tool_use_id;
                        converted.content = part.output;
                    }
                }
            }
        }
        body.messages.push(converted);
    }

    // Convert tools
    if tools.len() > 0 {
        body.tools = [];
        for tool in tools {
            body.tools.push(#{
                type: "function",
                function: #{
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters
                }
            });
        }
    }

    if options.max_tokens != () { body.max_tokens = options.max_tokens; }

    #{ endpoint: "/v1/chat/completions", method: "POST", body: body }
}

fn build_stream_request(config, messages, tools, options) {
    let req = build_request(config, messages, tools, options);
    req.body.stream = true;
    req.body.stream_options = #{ include_usage: true };
    req
}

fn build_headers(config, auth_token, request_info) {
    #{
        "Authorization": `Bearer ${auth_token}`,
        "Content-Type": "application/json",
        "User-Agent": "codelet/1.0"
    }
}

fn build_url(config, endpoint, options) {
    `${config.base_url}${endpoint}`
}

fn parse_response(config, status_code, body) {
    if status_code >= 400 {
        return map_error(config, status_code, body);
    }

    let choice = body.choices[0];
    let content = [];

    if choice.message.content != () && choice.message.content != "" {
        content.push(#{ type: "text", text: choice.message.content });
    }

    if choice.message.tool_calls != () {
        for tc in choice.message.tool_calls {
            content.push(#{
                type: "tool_use",
                id: tc.id,
                name: tc.function.name,
                input: json::parse(tc.function.arguments)
            });
        }
    }

    #{
        content: content,
        stop_reason: map_stop_reason(choice.finish_reason),
        usage: #{
            input_tokens: body.usage.prompt_tokens,
            output_tokens: body.usage.completion_tokens,
            cache_read_tokens: 0,
            cache_creation_tokens: 0
        }
    }
}

fn parse_stream_chunk(config, event_type, data) {
    if data == "[DONE]" { return #{ type: "done" }; }

    let chunk = json::parse(data);

    if chunk.choices.len() == 0 {
        if chunk.usage != () {
            return #{
                type: "usage",
                input_tokens: chunk.usage.prompt_tokens,
                output_tokens: chunk.usage.completion_tokens
            };
        }
        return #{ type: "ignore" };
    }

    let delta = chunk.choices[0].delta;

    if delta.content != () && delta.content != "" {
        return #{ type: "text", text: delta.content };
    }

    if delta.tool_calls != () {
        let tc = delta.tool_calls[0];
        return #{
            type: "tool_call_delta",
            index: tc.index,
            id: tc.id,
            name: if tc.function.name != () { tc.function.name } else { "" },
            arguments: if tc.function.arguments != () { tc.function.arguments } else { "" }
        };
    }

    if chunk.choices[0].finish_reason != () {
        return #{ type: "stop", stop_reason: map_stop_reason(chunk.choices[0].finish_reason) };
    }

    #{ type: "ignore" }
}

fn map_error(config, status_code, body) {
    let message = if body.error != () { body.error.message } else { `HTTP ${status_code}` };
    switch status_code {
        401 | 403 => #{ error: true, type: "authentication", message: message, retryable: false },
        429 => #{ error: true, type: "rate_limit", message: message, retryable: true },
        502 | 503 | 504 => #{ error: true, type: "timeout", message: message, retryable: true },
        _ => #{ error: true, type: "api", message: message, retryable: false }
    }
}

fn map_stop_reason(reason) {
    switch reason {
        "stop" | "end_turn" => "end_turn",
        "tool_calls" | "tool_use" => "tool_use",
        "length" | "max_tokens" => "max_tokens",
        _ => "end_turn"
    }
}
```

---

## 7. Streaming Architecture Deep Dive

### 7.1 The Problem

Streaming is the hardest part because:
1. SSE events arrive as a continuous byte stream
2. Each event's `data:` payload must be interpreted per-provider
3. Tool calls arrive incrementally (deltas) and must be accumulated
4. The stream loop needs to handle `select!` with interruption, stall timeout, etc.

### 7.2 The Solution: Rhai at the SSE-Event Level

The key insight: **split the streaming pipeline into Rust-owned and Rhai-owned layers**.

```
Layer 1 (Rust — rig's existing infra):
  HTTP response body → eventsource_stream → SSE frames
  Handles: byte buffering, line splitting, retry, reconnection

Layer 2 (Rhai — per-event interpretation):
  SSE frame { event_type, data } → parse_stream_chunk() → normalized chunk
  Handles: JSON parsing, field extraction, stop reason mapping

Layer 3 (Rust — stream_bridge):
  Normalized chunk → MultiTurnStreamItem
  Handles: tool call accumulation, usage aggregation, type conversion

Layer 4 (Rust — stream_loop):
  MultiTurnStreamItem → tokio::select! → StreamOutput::emit()
  Handles: interruption, stall timeout, tool execution, UI events
```

**Performance:** Rhai's `parse_stream_chunk()` is called ~10-100 times per completion (once per SSE event). At ~1μs per Rhai call, this adds <0.1ms total overhead — negligible compared to network latency.

### 7.3 Tool Call Accumulation

For streaming, tool calls arrive as deltas:
```
SSE: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_123","function":{"name":"Read"}}]}}]}
SSE: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"file"}}]}}]}
SSE: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"_path\":\"/tmp/x\"}"}}]}}]}
```

The Rhai script emits `tool_call_delta` chunks. The **Rust accumulator** (in stream_bridge) collects deltas by index and emits a complete `ToolCall` when `finish_reason: "tool_calls"` arrives. This accumulation logic stays in Rust — it's the same for all providers.

---

## 8. Auth Integration (Connection to PROV-060)

Custom providers support 4 auth types, all leveraging PROV-060's shared building blocks:

| Auth Type | Config | How It Works |
|-----------|--------|-------------|
| `bearer` | `{ type: "bearer", env_var: "API_KEY" }` | Read env var, set `Authorization: Bearer` |
| `api_key_header` | `{ type: "api_key_header", env_var: "API_KEY", header: "X-API-Key" }` | Read env var, set custom header |
| `oauth_device_code` | `{ type: "oauth_device_code", ... }` | PROV-060's generic DeviceCodeFlow |
| `oauth_pkce` | `{ type: "oauth_pkce", ... }` | PROV-060's generic OAuthCallbackServer |
| `custom` | `{ type: "custom", script: "my_auth.rhai" }` | Fully Rhai-scriptable auth flow |

For OAuth types, the Rhai script can also define custom token refresh logic via `refresh_token()` (from PROV-060).

---

## 9. Provider Discovery & Registration

### 9.1 Auto-Discovery

On startup, `ProviderManager` scans for custom provider configs:

```
1. ~/.fspec/providers/*.json          — user-global providers
2. .fspec/providers/*.json            — project-local providers
3. Built-in providers (compiled)      — Claude, Codex, Copilot, etc.
```

### 9.2 CLI Commands

```bash
# List all available providers (built-in + custom)
codelet providers list

# Show details of a custom provider
codelet providers show my-llm

# Validate a custom provider config + script
codelet providers validate my-llm

# Test a custom provider (sends a simple completion request)
codelet providers test my-llm --model my-llm-large

# Create a new custom provider from a template
codelet providers init my-new-provider --template openai-compatible
```

### 9.3 Model Selection

Custom providers integrate with the existing model selection:

```bash
# Select a custom provider model
codelet model my-llm/my-llm-large

# The provider name acts as the provider identifier
# Models are namespaced: <provider-name>/<model-id>
```

---

## 10. Estimated Effort

| Sub-task | Points | Phase |
|----------|--------|-------|
| `RhaiCustomProvider` (LlmProvider impl) | 5 | Core |
| `RhaiCompletionModel` (CompletionModel impl) | 5 | Core |
| `RhaiHttpClient` (HttpClientExt impl) | 3 | Core |
| `request_bridge` (CompletionRequest ↔ Dynamic) | 5 | Core |
| `response_bridge` (Dynamic → CompletionResponse) | 3 | Core |
| `stream_bridge` (SSE events → Rhai → StreamItems) | 8 | Streaming |
| `config.rs` (JSON schema + loader + validation) | 3 | Config |
| `script_loader` (compile + cache + hot-reload) | 3 | Config |
| Building block modules (http, json, crypto, time) | 5 | PROV-060 |
| ProviderManager integration + model selection | 3 | Integration |
| CLI commands (list, show, validate, test, init) | 5 | CLI |
| Example scripts + templates + documentation | 3 | Docs |
| **Total** | **51** | |

**Note:** Some of this work (building block modules, Rhai engine setup) is shared with PROV-060 and already estimated there. Net new work for PROV-061 is approximately **35-40 points** once PROV-060 is complete.

---

## 11. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Script errors crash the provider | Medium | High | Rhai errors caught and mapped to ProviderError; never panic |
| Streaming performance degradation | Low | Medium | Rhai only runs per-SSE-event (~1μs each), not per-byte |
| Provider quirks can't be expressed in Rhai | Medium | High | Escape hatch: `additional_params` passthrough; complex providers stay native |
| Message format complexity | High | Medium | Provide helper functions (format_content, format_tool_call) as building blocks |
| Security: malicious scripts | Low | High | Engine::new_raw sandbox; no FS/network beyond registered functions; env var allowlist |
| Hot-reload race conditions | Medium | Medium | AST swap behind Arc; in-flight requests use old AST; new requests get new AST |

---

## 12. Relationship to PROV-060

PROV-061 **depends on** PROV-060 for:

1. **Rhai engine infrastructure** — Engine::new_raw setup, sandboxing, operation limits
2. **Building block modules** — http, crypto, json, oauth modules registered in engine
3. **OAuth integration** — DeviceCodeFlow, OAuthCallbackServer for custom auth
4. **Shared traits** — CredentialStore<T>, TokenStrategy for auth token management

PROV-060 should be updated to note:
- The building block modules will be reused by PROV-061's custom provider scripts
- The engine setup should be designed for extensibility (easy to register new modules)
- The `oauth` module functions are a subset of the full module set PROV-061 needs

---

## 13. Rhai-Scriptable System Prompts

### 13.1 The Problem

Every provider has unique system prompt requirements:
- **Claude OAuth**: Requires `"You are Claude Code..."` prefix, uses array format with `cache_control` blocks
- **Claude API Key**: No prefix, array format with `cache_control`
- **Gemini**: Plain string, base prompt with examples, model-version-specific instructions (Gemini 3 tool silence)
- **OpenAI**: Plain string, no special transformation
- **ZAI/GLM**: Plain string with tool name references matching GLM facade names

Currently these are hardcoded Rust structs implementing `SystemPromptFacade`. A custom provider must be able to define its own system prompt formatting via Rhai.

### 13.2 The `SystemPromptFacade` Trait (What Must Be Scriptable)

```rust
pub trait SystemPromptFacade: Send + Sync {
    fn provider(&self) -> &'static str;
    fn identity_prefix(&self) -> Option<&'static str>;
    fn transform_preamble(&self, preamble: &str) -> String;
    fn format_for_api(&self, preamble: &str) -> Value;
}
```

A custom provider's Rhai script can define **3 optional system prompt functions**. If omitted, sensible defaults are used (plain string, no prefix, fspec guidance prepended).

### 13.3 Rhai Script Functions

#### Function 8: `identity_prefix(config) -> String|()` (optional)

Returns a prefix string prepended to all system prompts, or `()` for no prefix:

```javascript
fn identity_prefix(config) {
    // Some providers require an identity statement
    if config.identity != () {
        return config.identity;
    }
    // Return () (Rhai null) for no prefix
    ()
}
```

#### Function 9: `transform_preamble(config, preamble, fspec_guidance) -> String` (optional)

Transforms the raw preamble (project instructions like AGENTS.md) into the provider's expected format. Receives fspec workflow guidance as a separate argument so the script can place it where needed:

```javascript
fn transform_preamble(config, preamble, fspec_guidance) {
    let result = "";

    // Add identity prefix if configured
    if config.identity != () {
        result += config.identity + "\n\n";
    }

    // Add fspec guidance before project instructions
    result += fspec_guidance;

    // Add project-specific preamble
    if preamble != "" {
        result += "\n\n" + preamble;
    }

    // Add provider-specific instructions
    if config.custom_instructions != () {
        result += "\n\n" + config.custom_instructions;
    }

    result
}
```

#### Function 10: `format_system_prompt(config, preamble, fspec_guidance) -> Map|String` (optional)

Formats the complete system prompt for the provider's API. Returns either a plain string or a structured map (for providers that need array format with metadata like cache_control):

```javascript
fn format_system_prompt(config, preamble, fspec_guidance) {
    // Example: Array format with cache control (Claude-like)
    let blocks = [];

    // Identity prefix block (no cache_control — static)
    if config.identity != () {
        blocks.push(#{
            type: "text",
            text: config.identity
        });
    }

    // fspec guidance block (with cache_control — cacheable)
    blocks.push(#{
        type: "text",
        text: fspec_guidance,
        cache_control: #{ type: "ephemeral" }
    });

    // Project preamble block (with cache_control — variable content)
    if preamble != "" {
        blocks.push(#{
            type: "text",
            text: preamble,
            cache_control: #{ type: "ephemeral" }
        });
    }

    // Return map with format hint
    #{
        format: "array",
        blocks: blocks
    }
}
```

For simple providers that just need a string:

```javascript
fn format_system_prompt(config, preamble, fspec_guidance) {
    // Simple string format
    let prompt = fspec_guidance;
    if preamble != "" {
        prompt += "\n\n" + preamble;
    }
    prompt
}
```

### 13.4 Rust-Side: `RhaiSystemPromptFacade`

```rust
pub struct RhaiSystemPromptFacade {
    engine: Arc<Engine>,
    ast: Arc<AST>,
    config_dyn: Dynamic,
    provider_name: String,
}

impl SystemPromptFacade for RhaiSystemPromptFacade {
    fn provider(&self) -> &'static str {
        // Leak the string for 'static lifetime (provider lives for process lifetime)
        Box::leak(self.provider_name.clone().into_boxed_str())
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        // Call identity_prefix(config) in Rhai, if defined
        let result = self.call_optional_fn("identity_prefix", (self.config_dyn.clone(),));
        match result {
            Some(Dynamic::String(s)) => Some(Box::leak(s.into_boxed_str())),
            _ => None,
        }
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        if self.has_fn("transform_preamble") {
            let result = self.engine.call_fn::<Dynamic>(
                &mut Scope::new(), &self.ast, "transform_preamble",
                (self.config_dyn.clone(), preamble.to_string(), FSPEC_WORKFLOW_GUIDANCE.to_string())
            );
            match result {
                Ok(d) => d.to_string(),
                Err(_) => prepend_fspec_guidance(preamble), // fallback
            }
        } else {
            // Default: prepend fspec guidance
            prepend_fspec_guidance(preamble)
        }
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        if self.has_fn("format_system_prompt") {
            let result = self.engine.call_fn::<Dynamic>(
                &mut Scope::new(), &self.ast, "format_system_prompt",
                (self.config_dyn.clone(), preamble.to_string(), FSPEC_WORKFLOW_GUIDANCE.to_string())
            );
            match result {
                Ok(d) => dynamic_to_system_prompt_value(d),
                Err(_) => Value::String(prepend_fspec_guidance(preamble)), // fallback
            }
        } else {
            // Default: plain string format
            Value::String(self.transform_preamble(preamble))
        }
    }
}
```

### 13.5 Config Extension

The provider JSON config gains an optional `system_prompt` section:

```json
{
  "name": "my-llm",
  "script": "my_llm.rhai",
  "system_prompt": {
    "identity": "You are MyLLM, a helpful coding assistant.",
    "custom_instructions": "Always explain before running commands.",
    "format": "string"
  }
}
```

The `system_prompt.format` field hints at expected return type:
- `"string"` (default) — plain string system prompt
- `"array"` — structured array with metadata blocks (cache_control, etc.)

---

## 14. Rhai-Scriptable Tool Facades (Custom Tool Names & Mappings)

### 14.1 The Problem

Each LLM provider expects tools in a specific format:
- **Claude**: `Read`, `Write`, `Edit`, `Bash`, `Grep`, `Glob`, `Ls`, `WebSearch`
- **Gemini**: `read_file`, `write_file`, `replace`, `run_shell_command`, `search_file_content`, `find_files`, `list_directory`
- **ZAI/GLM**: `read_file`, `write_file`, `edit_file`, `run_command`, `grep_files`, `find_files`, `list_dir`
- **Codex**: `read_file`, `write_file`, `exec_command`, `shell`, `write_stdin`, `grep_files`, `list_dir`, `view_image`

Currently, each provider has **hardcoded Rust facade structs** (e.g., `ZAIReadFileFacade`, `GeminiReadFileFacade`, `CodexReadFileFacade`) that implement provider-specific traits and map parameters. This means adding a new provider requires writing hundreds of lines of Rust.

For custom providers, the Rhai script should be able to define **custom tool names, parameter schemas, and parameter mapping functions** — essentially scripting the entire facade layer.

### 14.2 Architecture Overview

```text
┌─────────────────────────────────────────────────────────────────────┐
│                     Provider Layer                                   │
├──────────┬──────────┬──────────┬──────────┬────────────────────────┤
│  Claude  │  Gemini  │   ZAI    │  Codex   │    Custom (Rhai)       │
│  (Rust)  │  (Rust)  │  (Rust)  │  (Rust)  │    (Scriptable)        │
├──────────┴──────────┴──────────┴──────────┴────────────────────────┤
│                    Facade Adapter Layer                              │
│  ┌─────────────────────────────────┐  ┌──────────────────────────┐ │
│  │ Hardcoded Trait Impls (Rust)    │  │ RhaiToolFacade (Dynamic) │ │
│  │ ZAIReadFileFacade, etc.         │  │ Calls Rhai map_params()  │ │
│  └─────────────────────────────────┘  └──────────────────────────┘ │
├────────────────────────────────────────────────────────────────────┤
│                  Internal Parameter Types                           │
│  InternalFileParams, InternalBashParams, InternalSearchParams, etc. │
├────────────────────────────────────────────────────────────────────┤
│                 Base Tool Implementations                           │
│  ReadTool, WriteTool, EditTool, BashTool, GrepTool, GlobTool, etc. │
└────────────────────────────────────────────────────────────────────┘
```

### 14.3 How It Works

A custom provider defines tool facades in its `.rhai` script via a `define_tools(config)` function that returns a list of tool definitions with embedded mapping logic:

#### Function 11: `define_tools(config) -> Array` (optional)

```javascript
fn define_tools(config) {
    [
        #{
            // The tool name the LLM sees
            name: "read_file",
            // Which internal tool this maps to
            maps_to: "file:read",
            // Description shown to LLM
            description: "Read file contents. Returns line-numbered text.",
            // JSON Schema for parameters
            parameters: #{
                type: "object",
                properties: #{
                    file_path: #{
                        type: "string",
                        description: "Absolute path to the file"
                    },
                    offset: #{
                        type: "integer",
                        description: "1-based line number to start reading from"
                    },
                    limit: #{
                        type: "integer",
                        description: "Number of lines to read"
                    }
                },
                required: ["file_path"],
                additionalProperties: false
            }
        },
        #{
            name: "write_file",
            maps_to: "file:write",
            description: "Write content to a file (creates or overwrites).",
            parameters: #{
                type: "object",
                properties: #{
                    file_path: #{ type: "string", description: "Absolute path" },
                    content: #{ type: "string", description: "Content to write" }
                },
                required: ["file_path", "content"],
                additionalProperties: false
            }
        },
        #{
            name: "edit_file",
            maps_to: "file:edit",
            description: "Edit file by replacing old_string with new_string.",
            parameters: #{
                type: "object",
                properties: #{
                    file_path: #{ type: "string", description: "Absolute path" },
                    old_string: #{ type: "string", description: "String to find" },
                    new_string: #{ type: "string", description: "Replacement" }
                },
                required: ["file_path", "old_string", "new_string"],
                additionalProperties: false
            }
        },
        #{
            name: "execute",
            maps_to: "bash",
            description: "Execute a shell command.",
            parameters: #{
                type: "object",
                properties: #{
                    command: #{ type: "string", description: "Shell command to run" },
                    cwd: #{ type: "string", description: "Working directory" }
                },
                required: ["command"],
                additionalProperties: false
            }
        },
        #{
            name: "search",
            maps_to: "search:grep",
            description: "Search file contents with regex.",
            parameters: #{
                type: "object",
                properties: #{
                    pattern: #{ type: "string", description: "Regex pattern" },
                    path: #{ type: "string", description: "Directory to search" },
                    include: #{ type: "string", description: "Glob filter" }
                },
                required: ["pattern"],
                additionalProperties: false
            }
        },
        #{
            name: "find",
            maps_to: "search:glob",
            description: "Find files by glob pattern.",
            parameters: #{
                type: "object",
                properties: #{
                    pattern: #{ type: "string", description: "Glob pattern" },
                    path: #{ type: "string", description: "Directory to search in" }
                },
                required: ["pattern"],
                additionalProperties: false
            }
        },
        #{
            name: "ls",
            maps_to: "ls",
            description: "List directory contents.",
            parameters: #{
                type: "object",
                properties: #{
                    path: #{ type: "string", description: "Directory path" }
                },
                additionalProperties: false
            }
        },
        #{
            name: "web_search",
            maps_to: "web_search:search",
            description: "Search the web.",
            parameters: #{
                type: "object",
                properties: #{
                    query: #{ type: "string", description: "Search query" }
                },
                required: ["query"],
                additionalProperties: false
            }
        }
    ]
}
```

#### Function 12: `map_tool_params(config, tool_name, maps_to, params) -> Map` (optional)

Custom parameter mapping when the Rhai script needs to transform parameter names/values beyond the default mapping. If not defined, or if it returns `()`, the default mapping is used (matching parameter names to `Internal*Params` fields):

```javascript
fn map_tool_params(config, tool_name, maps_to, params) {
    // Example: This provider uses "filepath" instead of "file_path"
    if maps_to == "file:read" {
        return #{
            file_path: params.filepath,
            offset: params.start_line,
            limit: params.num_lines
        };
    }

    // Example: This provider wraps commands differently
    if maps_to == "bash" {
        let cmd = params.command;
        if params.sudo == true {
            cmd = "sudo " + cmd;
        }
        return #{
            command: cmd,
            cwd: params.working_dir
        };
    }

    // Return () to use default mapping
    ()
}
```

### 14.4 Internal Tool Target Registry

The `maps_to` field uses a simple string identifier that maps to the internal tool type:

| `maps_to` Value | Internal Type | Base Tool |
|------------------|---------------|-----------|
| `file:read` | `InternalFileParams::Read` | `ReadTool` |
| `file:write` | `InternalFileParams::Write` | `WriteTool` |
| `file:edit` | `InternalFileParams::Edit` | `EditTool` |
| `bash` | `InternalBashParams::Execute` | `BashTool` |
| `search:grep` | `InternalSearchParams::Grep` | `GrepTool` |
| `search:glob` | `InternalSearchParams::Glob` | `GlobTool` |
| `ls` | `InternalLsParams::List` | `LsTool` |
| `web_search:search` | `InternalWebSearchParams::Search` | `WebSearchTool` |
| `web_search:open` | `InternalWebSearchParams::OpenPage` | `WebSearchTool` |
| `web_search:find` | `InternalWebSearchParams::FindInPage` | `WebSearchTool` |
| `web_search:screenshot` | `InternalWebSearchParams::CaptureScreenshot` | `WebSearchTool` |
| `fspec` | `InternalFspecParams` | `FspecTool` |
| `bridge` | `InternalBridgeParams` | `BridgeTool` |
| `exec:run` | `InternalExecParams::Run` | `UnifiedExecTool` |
| `exec:write` | `InternalExecParams::Write` | `UnifiedExecTool` |
| `hitl` | `InternalHitlParams::Request` | `RequestUserInputTool` |

### 14.5 Rust-Side: `RhaiToolFacadeAdapter`

A single generic Rust struct adapts **all** Rhai-defined tools:

```rust
pub struct RhaiToolFacadeAdapter {
    tool_def: RhaiToolDef,      // name, maps_to, description, parameters
    engine: Arc<Engine>,
    ast: Arc<AST>,
    config_dyn: Dynamic,
    provider_name: String,
    session_id: Uuid,
}

/// Parsed tool definition from Rhai's define_tools()
#[derive(Debug, Clone)]
pub struct RhaiToolDef {
    pub name: String,
    pub maps_to: String,        // e.g., "file:read", "bash", "search:grep"
    pub description: String,
    pub parameters: Value,      // JSON Schema
}

impl Tool for RhaiToolFacadeAdapter {
    const NAME: &'static str = "rhai_facade";
    type Error = ToolError;
    type Args = FacadeArgs;
    type Output = Value;

    fn name(&self) -> String {
        self.tool_def.name.clone()
    }

    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        RigToolDefinition {
            name: self.tool_def.name.clone(),
            description: self.tool_def.description.clone(),
            parameters: self.tool_def.parameters.clone(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 1. Pre-tool hook check
        check_pre_tool_hook(self.session_id, &self.name(), &args.0)?;

        // 2. Try custom mapping via Rhai first
        let mapped = self.try_rhai_mapping(&args.0)?;

        // 3. Convert to internal params
        let internal = if let Some(mapped) = mapped {
            self.mapped_to_internal(&mapped)?
        } else {
            // Default mapping: match param names directly
            self.default_to_internal(&args.0)?
        };

        // 4. Execute via the appropriate base tool
        self.execute_internal(internal).await
    }
}

impl RhaiToolFacadeAdapter {
    fn try_rhai_mapping(&self, params: &Value) -> Result<Option<Value>, ToolError> {
        // Call map_tool_params(config, tool_name, maps_to, params) if it exists
        if !self.has_fn("map_tool_params") {
            return Ok(None);
        }
        let result = tokio::task::block_in_place(|| {
            let mut scope = Scope::new();
            self.engine.call_fn::<Dynamic>(
                &mut scope, &self.ast, "map_tool_params",
                (
                    self.config_dyn.clone(),
                    self.tool_def.name.clone(),
                    self.tool_def.maps_to.clone(),
                    rhai::serde::to_dynamic(params).unwrap_or_default(),
                )
            )
        });
        match result {
            Ok(d) if d.is_unit() => Ok(None),   // () means "use default"
            Ok(d) => Ok(Some(rhai::serde::from_dynamic(&d)?)),
            Err(e) => Err(ToolError::execution(format!("Rhai map_tool_params error: {e}"))),
        }
    }

    fn default_to_internal(&self, params: &Value) -> Result<InternalToolParams, ToolError> {
        // Parse maps_to and extract standard field names from params
        match self.tool_def.maps_to.as_str() {
            "file:read" => Ok(InternalToolParams::File(InternalFileParams::Read {
                file_path: extract_required_string(params, "file_path")?,
                offset: extract_optional_uint(params, "offset"),
                limit: extract_optional_uint(params, "limit"),
                mode: None,
                indentation: None,
            })),
            "bash" => Ok(InternalToolParams::Bash(InternalBashParams::Execute {
                command: extract_required_string(params, "command")?,
                cwd: extract_optional_string(params, "cwd"),
                timeout_ms: None,
            })),
            // ... other mappings follow the same pattern
            _ => Err(ToolError::execution(format!(
                "Unknown maps_to target: {}", self.tool_def.maps_to
            ))),
        }
    }
}
```

### 14.6 Default Tool Sets

If `define_tools()` is not defined in the Rhai script, the custom provider falls back to a **default tool set** based on a configurable `tool_style` in the provider config:

```json
{
  "name": "my-llm",
  "tool_style": "openai"
}
```

| `tool_style` | Tool Names | Description |
|---------------|------------|-------------|
| `"claude"` (default) | `Read`, `Write`, `Edit`, `Bash`, `Grep`, `Glob`, `Ls`, `WebSearch` | Claude-native PascalCase names |
| `"openai"` | `read_file`, `write_file`, `edit_file`, `bash`, `grep`, `glob`, `ls`, `web_search` | OpenAI-style snake_case names |
| `"gemini"` | `read_file`, `write_file`, `replace`, `run_shell_command`, `search_file_content`, `find_files`, `list_directory` | Gemini-native names |
| `"codex"` | Uses Codex facade names | Full Codex compatibility |
| `"custom"` | Must define `define_tools()` | Fully custom — no defaults |

### 14.7 Tool Visibility Control

Custom providers can also control which tools are exposed to the LLM:

```json
{
  "name": "my-llm",
  "tools": {
    "enabled": ["file:read", "file:write", "file:edit", "bash", "search:grep", "ls"],
    "disabled": ["web_search:search", "bridge", "exec:run"]
  }
}
```

Or in Rhai via `define_tools()` — if a tool target isn't listed, it's simply not exposed.

### 14.8 Interaction with System Prompt Tool References

A key integration point: system prompts often reference tool names (e.g., Gemini's prompt mentions `search_file_content`, `find_files`, etc.). When tool facades are Rhai-scriptable, the system prompt must reference the **custom tool names**, not the internal names.

The `transform_preamble()` / `format_system_prompt()` functions receive the config, which includes the resolved tool names. The script can reference them:

```javascript
fn transform_preamble(config, preamble, fspec_guidance) {
    let tools = config.resolved_tools;  // Array of {name, maps_to, description}
    let tool_list = "";
    for t in tools {
        tool_list += "- `" + t.name + "`: " + t.description + "\n";
    }

    fspec_guidance + "\n\n# Available Tools\n\n" + tool_list + "\n\n" + preamble
}
```

### 14.9 Estimated Additional Effort

| Sub-task | Points | Phase |
|----------|--------|-------|
| `RhaiSystemPromptFacade` implementation | 3 | System Prompt |
| System prompt config + defaults | 2 | System Prompt |
| `RhaiToolFacadeAdapter` (generic Rhai→Internal mapper) | 5 | Tool Facades |
| Default tool sets (tool_style presets) | 3 | Tool Facades |
| `define_tools()` Rhai interface + validation | 3 | Tool Facades |
| `map_tool_params()` Rhai interface | 3 | Tool Facades |
| Tool visibility control | 2 | Tool Facades |
| Integration with ProviderManager tool registration | 3 | Integration |
| Tests for Rhai system prompts | 3 | Testing |
| Tests for Rhai tool facades | 5 | Testing |
| **Additional total** | **32** | |

**Revised PROV-061 total: ~67-72 points** (original 35-40 net + 32 new). This should be broken into sub-work-units.

---

## 15. Future Extensions (Out of Scope)

These are natural follow-ons but NOT part of PROV-061:

1. **Rhai script marketplace** — community-shared provider scripts
2. **Provider composition** — chain multiple providers (fallback, load balance)
3. **Response transformation** — post-process responses (e.g., strip thinking tags)

---

## 16. References

- **PROV-060 research:** `spec/attachments/PROV-060/shared-oauth-rhai-research.md`
- **Rhai documentation:** https://rhai.rs/book/
- **rig CompletionModel trait:** `codelet/patches/rig-core/src/completion/request.rs`
- **Provider adapter:** `codelet/providers/src/adapter.rs`
- **Stream loop:** `codelet/cli/src/interactive/stream_loop.rs`
- **SSE parsing:** `codelet/patches/rig-core/src/http_client/sse.rs`
- **Anthropic streaming:** `codelet/patches/rig-core/src/providers/anthropic/streaming.rs`
