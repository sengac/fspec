# PROV-063: Custom Provider HTTP Request/Response Lifecycle

## Research Document

**Work Unit:** PROV-063  
**Date:** 2026-04-17  
**Status:** Research Complete  

---

## Table of Contents

1. [RhaiCustomProvider Struct](#1-rhaicustomprovider-struct)
2. [Request Bridge (CompletionRequest → Rhai Dynamic)](#2-request-bridge)
3. [Response Bridge (Rhai Dynamic → CompletionResponse)](#3-response-bridge)
4. [The 7 Required Script Functions](#4-the-7-required-script-functions)
5. [spawn_blocking Pattern](#5-spawn_blocking-pattern)
6. [Error Mapping](#6-error-mapping)
7. [RhaiHttpClient](#7-rhaihttpclient)

---

## 1. RhaiCustomProvider Struct

### 1.1 The LlmProvider Trait

The `LlmProvider` trait is defined in `codelet/providers/src/lib.rs:90-118`:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
    fn supports_caching(&self) -> bool;
    fn supports_streaming(&self) -> bool;

    async fn complete(
        &self,
        messages: &[codelet_common::Message],
    ) -> Result<String, ProviderError>;

    async fn complete_with_tools(
        &self,
        messages: &[codelet_common::Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse, ProviderError>;
}
```

### 1.2 CompletionResponse and StopReason

Defined in `codelet/providers/src/lib.rs:64-82`:

```rust
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: MessageContent,
    pub stop_reason: StopReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
}
```

### 1.3 How ClaudeProvider Implements It

From `codelet/providers/src/claude.rs:152-161`, ClaudeProvider stores:

```rust
#[derive(Clone)]
pub struct ClaudeProvider {
    completion_model: ClaudeCompletionModel,
    rig_client: ClaudeClient,
    auth_mode: AuthMode,
    model_name: String,
}
```

The `LlmProvider` impl is at `claude.rs:680-745`. Key patterns:

- `name()` returns `"claude"` (static)
- `model()` returns `&self.model_name`
- `context_window()` returns a constant (`200_000`)
- `max_output_tokens()` returns a constant (`8192`)
- `supports_caching()` and `supports_streaming()` return `true`
- `complete()` delegates to `complete_with_tools()` with empty tools
- `complete_with_tools()` extracts prompt data, converts tools to rig format, builds a `CompletionRequestBuilder`, sends it, then converts the rig response

### 1.4 ModelLimitsResolver Trait

From `codelet/providers/src/model_limits.rs:27-58`, providers also implement:

```rust
pub trait ModelLimitsResolver: Send + Sync {
    fn max_context_window(&self) -> Option<usize> { None }
    fn max_output_tokens_limit(&self) -> Option<usize> { None }
    fn default_context_window(&self) -> usize;
    fn default_max_output_tokens(&self) -> usize;
    fn should_send_max_output_tokens(&self) -> bool { true }
}
```

### 1.5 Proposed RhaiCustomProvider Struct

```rust
use std::sync::Arc;
use rhai::{Engine, AST};
use async_trait::async_trait;

/// Custom LLM provider backed by a Rhai script.
///
/// The script defines 7 functions that control the full HTTP
/// request/response lifecycle. The Rust side handles serialization
/// bridges and async/sync boundaries.
#[derive(Clone)]
pub struct RhaiCustomProvider {
    /// Provider name from script metadata
    provider_name: String,
    /// Model identifier from script metadata
    model_name: String,
    /// Context window size (from script's `get_context_window()`)
    context_window: usize,
    /// Max output tokens (from script's `get_max_output_tokens()`)
    max_output_tokens: usize,
    /// Sandboxed Rhai engine (Arc for Send + Sync + Clone)
    engine: Arc<Engine>,
    /// Compiled script AST (Arc for Send + Sync + Clone)
    ast: Arc<AST>,
    /// HTTP client for making requests
    http_client: reqwest::Client,
}
```

**Key design decisions:**

- `Engine` and `AST` are wrapped in `Arc` — Rhai's `Engine` is `Send + Sync` and `AST` is `Clone + Send + Sync`, but wrapping in `Arc` avoids expensive full clones when moving into `spawn_blocking` closures.
- `http_client` is `reqwest::Client` which is cheap to clone (uses `Arc` internally).
- Provider metadata (`provider_name`, `model_name`, `context_window`, `max_output_tokens`) are resolved at construction time by calling script functions once, avoiding repeated Rhai calls for simple getters.

### 1.6 ProviderAdapter Implementation

Following the pattern in `codelet/providers/src/adapter.rs:190-231`:

```rust
impl ProviderAdapter for RhaiCustomProvider {
    fn provider_name(&self) -> &'static str {
        // Note: Can't return &'static str from a runtime String.
        // Options: use a leaked &'static str, or change the trait.
        // For now, the custom provider can store a &'static str
        // via Box::leak for the provider name.
        "custom"
    }
}
```

---

## 2. Request Bridge (CompletionRequest → Rhai Dynamic)

### 2.1 CompletionRequest Structure

From `codelet/patches/rig-core/src/completion/request.rs:497-515`:

```rust
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub preamble: Option<String>,
    pub chat_history: OneOrMany<Message>,
    pub documents: Vec<Document>,
    pub tools: Vec<ToolDefinition>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    pub tool_choice: Option<ToolChoice>,
    pub additional_params: Option<serde_json::Value>,
}
```

However, for the custom provider, the Rust side does **not** use rig's `CompletionRequest` directly. Instead, `RhaiCustomProvider` implements `LlmProvider` which receives `codelet_common::Message` slices and `codelet_tools::ToolDefinition` slices. These must be converted to Rhai `Dynamic` values.

### 2.2 Input Types to Serialize

**codelet_common::Message** (from `codelet/common/src/types.rs:54-60`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
}
```

**MessageRole** (from `codelet/common/src/types.rs:9-18`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}
```

**MessageContent** (from `codelet/common/src/types.rs:22-28`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}
```

**ContentPart** (from `codelet/common/src/types.rs:31-51`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}
```

**ToolDefinition** (from `codelet/tools/src/lib.rs:216-223`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
```

### 2.3 rhai::serde::to_dynamic Behavior

From `/tmp/rhai/src/serde/ser.rs:83-86`:

```rust
pub fn to_dynamic<T: Serialize>(value: T) -> RhaiResult {
    let mut s = DynamicSerializer::new(Dynamic::UNIT);
    value.serialize(&mut s)
}
```

**Serialization mapping** (from `ser.rs:94-453`):

| Rust Type | Rhai Dynamic Type | Notes |
|-----------|------------------|-------|
| `bool` | `bool` | Direct |
| `i8..i64` | `INT` (i64) | Widened to i64 |
| `u8..u64` | `INT` (i64) | May overflow for large u64 |
| `f32, f64` | `FLOAT` (f64) | Direct |
| `char` | `char` | Direct |
| `&str, String` | `ImmutableString` | Rhai's string type |
| `None` | `()` (unit) | `serialize_none` → `Dynamic::UNIT` |
| `Some(T)` | `T` | Unwrapped, no wrapper |
| `Vec<T>` | `Array` | Recursive serialization |
| `HashMap<String, T>` | `Map` | Keys must be strings |
| Struct | `Map` | Field names become map keys |
| Unit variant enum | `String` | `serialize_unit_variant` → variant name as string |
| Newtype variant | `Map { variant: value }` | Wrapped in single-entry map |

**Critical behaviors for our bridge:**

1. **Enums with `#[serde(rename_all = "lowercase")]`**: `MessageRole::System` serializes to string `"system"`, `MessageRole::User` to `"user"`, etc.

2. **Enums with `#[serde(tag = "type")]`**: `ContentPart::Text { text: "hello" }` serializes to `Map { "type": "text", "text": "hello" }`.

3. **Enums with `#[serde(untagged)]`**: `MessageContent::Text("hello")` serializes to just the string `"hello"`. `MessageContent::Parts(vec)` serializes to an array.

4. **`serde_json::Value` fields** (`input_schema`, `input`): These serialize through serde's own `Serialize` impl, which `to_dynamic` handles recursively — JSON objects become Rhai Maps, JSON arrays become Rhai Arrays, etc.

5. **`Option<T>` fields**: `None` → `()` (unit), `Some(v)` → the value itself. This means missing optional fields in the Rhai map will be unit `()`, not absent.

### 2.4 Proposed Serialization Code

```rust
use rhai::serde::to_dynamic;
use rhai::Dynamic;

/// Convert messages + tools into a Rhai Dynamic map for the script.
fn build_request_dynamic(
    messages: &[codelet_common::Message],
    tools: &[codelet_tools::ToolDefinition],
) -> Result<Dynamic, ProviderError> {
    // Messages serialize via serde: Vec<Message> → Rhai Array of Maps
    let messages_dyn = to_dynamic(messages)
        .map_err(|e| ProviderError::api("custom", format!("Failed to serialize messages: {e}")))?;

    // Tools serialize via serde: Vec<ToolDefinition> → Rhai Array of Maps
    let tools_dyn = to_dynamic(tools)
        .map_err(|e| ProviderError::api("custom", format!("Failed to serialize tools: {e}")))?;

    // Build the top-level request map
    let mut request_map = rhai::Map::new();
    request_map.insert("messages".into(), messages_dyn);
    request_map.insert("tools".into(), tools_dyn);

    Ok(Dynamic::from_map(request_map))
}
```

### 2.5 What the Script Receives

After serialization, the Rhai script receives a map like:

```rhai
// The `request` parameter passed to build_request(request):
// #{
//   messages: [
//     #{ role: "system", content: "You are a helpful assistant" },
//     #{ role: "user", content: "Hello" },
//     #{ role: "assistant", content: [
//       #{ type: "text", text: "Hi there!" },
//       #{ type: "tool_use", id: "call_1", name: "read", input: #{ path: "/tmp/x" } }
//     ]},
//     #{ role: "user", content: [
//       #{ type: "tool_result", tool_use_id: "call_1", content: "file data", is_error: false }
//     ]}
//   ],
//   tools: [
//     #{ name: "read", description: "Read a file", input_schema: #{ ... } }
//   ]
// }
```

---

## 3. Response Bridge (Rhai Dynamic → CompletionResponse)

### 3.1 Target Types

The script's response must be convertible back to `CompletionResponse` (from `codelet/providers/src/lib.rs:64-82`):

```rust
pub struct CompletionResponse {
    pub content: MessageContent,       // Text or Parts
    pub stop_reason: StopReason,       // EndTurn, ToolUse, MaxTokens
}
```

Where `MessageContent::Parts(Vec<ContentPart>)` can contain:
- `ContentPart::Text { text }` — plain text
- `ContentPart::ToolUse { id, name, input }` — tool call
- `ContentPart::ToolResult { .. }` — (not used in responses)

### 3.2 rhai::serde::from_dynamic Behavior

From `/tmp/rhai/src/serde/de.rs:107-109`:

```rust
pub fn from_dynamic<'de, T: Deserialize<'de>>(value: &'de Dynamic) -> RhaiResultOf<T> {
    T::deserialize(DynamicDeserializer::new(value))
}
```

**Deserialization mapping** (from `de.rs:121-506`):

| Rhai Dynamic Type | Rust Target | Notes |
|-------------------|-------------|-------|
| `()` (unit) | `Option<T>` → `None` | `deserialize_option` → `visit_none` |
| Non-unit | `Option<T>` → `Some(T)` | `deserialize_option` → `visit_some` |
| `bool` | `bool` | Direct |
| `INT` (i64) | `i8..i64, u8..u64` | Via visitor |
| `FLOAT` (f64) | `f32, f64` | Direct |
| `ImmutableString` | `&str, String` | Borrowed or owned |
| `Array` | `Vec<T>` | Recursive deserialization |
| `Map` | `HashMap<K,V>`, struct | Keys iterated via `MapAccess` |

**Critical behaviors for our bridge:**

1. **Struct deserialization from Map**: `from_dynamic` deserializes a Rhai `Map` into a Rust struct by iterating map keys and matching them to struct field names (via `deserialize_struct` at `de.rs:438-464`).

2. **Enum deserialization**: For `#[serde(rename_all = "lowercase")]` enums like `StopReason`, a Rhai string `"end_turn"` deserializes to `StopReason::EndTurn`. For tagged enums like `ContentPart` with `#[serde(tag = "type")]`, a Rhai `Map` with a `"type"` key determines the variant.

3. **`serde_json::Value` fields**: These deserialize from Rhai `Dynamic` through serde's `Deserialize` impl. Rhai Maps become JSON objects, Arrays become JSON arrays, etc.

4. **Missing fields cause errors**: If a required struct field is missing from the Rhai Map, deserialization fails. The script MUST include all required fields.

### 3.3 Expected Script Response Format

The script's `parse_response` function must return a map matching `CompletionResponse`:

```rhai
// Simple text response:
fn parse_response(raw_response) {
    #{
        content: [
            #{ type: "text", text: raw_response.choices[0].message.content }
        ],
        stop_reason: "end_turn"
    }
}

// Tool use response:
fn parse_response(raw_response) {
    let parts = [];
    for tc in raw_response.choices[0].message.tool_calls {
        parts.push(#{
            type: "tool_use",
            id: tc.id,
            name: tc.function.name,
            input: json::parse(tc.function.arguments)
        });
    }
    #{
        content: parts,
        stop_reason: "tool_use"
    }
}
```

### 3.4 Proposed Deserialization Code

Rather than using `from_dynamic` directly on complex nested types (which can be fragile with tagged enums), a manual extraction approach is more robust:

```rust
use rhai::{Dynamic, Map};

fn parse_script_response(result: Dynamic) -> Result<CompletionResponse, ProviderError> {
    let map = result.try_cast::<Map>().ok_or_else(|| {
        ProviderError::api("custom", "parse_response must return a Map")
    })?;

    // Extract stop_reason
    let stop_reason_str = map.get("stop_reason")
        .and_then(|v| v.clone().into_string().ok())
        .unwrap_or_else(|| "end_turn".to_string());

    let stop_reason = match stop_reason_str.as_str() {
        "tool_use" => StopReason::ToolUse,
        "max_tokens" => StopReason::MaxTokens,
        _ => StopReason::EndTurn,
    };

    // Extract content parts
    let content_dyn = map.get("content").ok_or_else(|| {
        ProviderError::api("custom", "Response missing 'content' field")
    })?;

    let content = if let Ok(text) = content_dyn.clone().into_string() {
        // Simple text response
        MessageContent::Text(text)
    } else if content_dyn.is_array() {
        // Array of content parts
        let parts = content_dyn.clone()
            .into_typed_array::<Dynamic>()
            .map_err(|_| ProviderError::api("custom", "Invalid content array"))?;

        let mut content_parts = Vec::new();
        for part in &parts {
            let part_map = part.clone().try_cast::<Map>().ok_or_else(|| {
                ProviderError::api("custom", "Content part must be a Map")
            })?;

            let part_type = part_map.get("type")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();

            match part_type.as_str() {
                "text" => {
                    let text = part_map.get("text")
                        .and_then(|v| v.clone().into_string().ok())
                        .unwrap_or_default();
                    content_parts.push(ContentPart::Text { text });
                }
                "tool_use" => {
                    let id = part_map.get("id")
                        .and_then(|v| v.clone().into_string().ok())
                        .unwrap_or_default();
                    let name = part_map.get("name")
                        .and_then(|v| v.clone().into_string().ok())
                        .unwrap_or_default();
                    let input_dyn = part_map.get("input")
                        .cloned()
                        .unwrap_or(Dynamic::UNIT);
                    let input = dynamic_to_json_value(&input_dyn);
                    content_parts.push(ContentPart::ToolUse { id, name, input });
                }
                other => {
                    return Err(ProviderError::api(
                        "custom",
                        format!("Unknown content part type: {other}"),
                    ));
                }
            }
        }
        MessageContent::Parts(content_parts)
    } else {
        return Err(ProviderError::api(
            "custom",
            "Response 'content' must be a string or array",
        ));
    };

    Ok(CompletionResponse { content, stop_reason })
}
```

> **Note:** The `dynamic_to_json_value` helper already exists in `codelet/providers/src/oauth/building_blocks.rs:245-271`. It should be extracted to a shared module.

---

## 4. The 7 Required Script Functions

### Overview

The custom provider script must define 7 functions that control the complete HTTP request/response lifecycle. These are called by the Rust side via `engine.call_fn()`.

### 4.1 `provider_info()` → Map

**Purpose:** Return static provider metadata at load time.

**Rust calling pattern:**
```rust
let result: Dynamic = engine.call_fn(&mut scope, &ast, "provider_info", ())?;
let map = result.try_cast::<Map>().ok_or_else(|| {
    ProviderError::config("custom", "provider_info must return a Map")
})?;
```

**Expected Rhai return:**
```rhai
fn provider_info() {
    #{
        name: "my-provider",
        model: "my-model-v1",
        context_window: 128000,
        max_output_tokens: 4096,
        supports_caching: false,
        supports_streaming: false
    }
}
```

**Error handling:** Called at construction time. Failure prevents provider creation. Missing fields use sensible defaults (e.g., `supports_caching: false`).

---

### 4.2 `build_headers(config)` → Map

**Purpose:** Build HTTP headers for the API request. Called before every request.

**Rust calling pattern:**
```rust
let config = self.build_config_dynamic();
let result: Dynamic = engine.call_fn(&mut scope, &ast, "build_headers", (config,))?;
let headers_map = result.try_cast::<Map>().ok_or_else(|| {
    ProviderError::api("custom", "build_headers must return a Map")
})?;
// Convert Map to reqwest::header::HeaderMap
let mut header_map = HeaderMap::new();
for (key, value) in &headers_map {
    let name = HeaderName::from_str(key.as_str())
        .map_err(|e| ProviderError::api("custom", format!("Invalid header name '{key}': {e}")))?;
    let val_str = value.clone().into_string()
        .map_err(|_| ProviderError::api("custom", format!("Header '{key}' value must be a string")))?;
    let val = HeaderValue::from_str(&val_str)
        .map_err(|e| ProviderError::api("custom", format!("Invalid header value for '{key}': {e}")))?;
    header_map.insert(name, val);
}
```

**Expected Rhai return:**
```rhai
fn build_headers(config) {
    #{
        "Authorization": "Bearer " + config.api_key,
        "Content-Type": "application/json",
        "X-Custom-Header": "value"
    }
}
```

**Error handling:** Rhai errors map to `ProviderError::Api`. Invalid header names/values are caught on the Rust side.

---

### 4.3 `build_url(config)` → String

**Purpose:** Return the full API endpoint URL.

**Rust calling pattern:**
```rust
let config = self.build_config_dynamic();
let result: Dynamic = engine.call_fn(&mut scope, &ast, "build_url", (config,))?;
let url = result.into_string()
    .map_err(|_| ProviderError::api("custom", "build_url must return a string"))?;
```

**Expected Rhai return:**
```rhai
fn build_url(config) {
    config.base_url + "/v1/chat/completions"
}
```

**Error handling:** Must return a valid URL string. Rust side validates URL parsing.

---

### 4.4 `build_request(request)` → Map

**Purpose:** Transform the request data into the provider-specific API request body. This is the core serialization function.

**Rust calling pattern:**
```rust
// `request` is the Dynamic map built by build_request_dynamic()
// (see Section 2.4)
let result: Dynamic = engine.call_fn(
    &mut scope, &ast, "build_request", (request,)
)?;
let body_map = result.try_cast::<Map>().ok_or_else(|| {
    ProviderError::api("custom", "build_request must return a Map")
})?;
// Serialize to JSON for HTTP body
let body_json = dynamic_to_json_value(&Dynamic::from_map(body_map));
let body_string = serde_json::to_string(&body_json)
    .map_err(|e| ProviderError::api("custom", format!("JSON serialization failed: {e}")))?;
```

**Expected Rhai return:**
```rhai
fn build_request(request) {
    let messages = [];
    for msg in request.messages {
        if msg.role == "system" {
            // OpenAI-compatible format
            messages.push(#{
                role: "system",
                content: msg.content
            });
        } else if msg.role == "user" {
            messages.push(#{
                role: "user",
                content: msg.content
            });
        } else if msg.role == "assistant" {
            messages.push(#{
                role: "assistant",
                content: msg.content
            });
        }
    }

    let body = #{
        model: "my-model-v1",
        messages: messages,
        max_tokens: 4096
    };

    // Add tools if present
    if request.tools.len() > 0 {
        body.tools = request.tools;
    }

    body
}
```

**Error handling:** Script errors propagate as `ProviderError::Api`. The Rust side validates the result is a Map.

---

### 4.5 `parse_response(raw_response)` → Map

**Purpose:** Parse the API response body into the standard CompletionResponse format. This is the core deserialization function.

**Rust calling pattern:**
```rust
// `raw_response` is the parsed JSON response body as Dynamic
let raw_dyn = json_value_to_dynamic(&response_json);
let result: Dynamic = engine.call_fn(
    &mut scope, &ast, "parse_response", (raw_dyn,)
)?;
// Convert to CompletionResponse (see Section 3.4)
let response = parse_script_response(result)?;
```

**Expected Rhai return:**
```rhai
fn parse_response(raw) {
    let content = [];

    // Handle OpenAI-compatible response format
    let choice = raw.choices[0];
    let message = choice.message;

    // Text content
    if message.content != () {
        content.push(#{
            type: "text",
            text: message.content
        });
    }

    // Tool calls
    if message.contains("tool_calls") {
        for tc in message.tool_calls {
            content.push(#{
                type: "tool_use",
                id: tc.id,
                name: tc.function.name,
                input: json::parse(tc.function.arguments)
            });
        }
    }

    // Map stop reason
    let stop = switch choice.finish_reason {
        "stop" => "end_turn",
        "tool_calls" => "tool_use",
        "length" => "max_tokens",
        _ => "end_turn"
    };

    #{
        content: content,
        stop_reason: stop
    }
}
```

**Error handling:** Must return a Map with `content` (array or string) and `stop_reason` (string). Missing fields cause `ProviderError::Api`.

---

### 4.6 `parse_error(status_code, body)` → Map

**Purpose:** Parse error responses into structured error information.

**Rust calling pattern:**
```rust
let result: Dynamic = engine.call_fn(
    &mut scope, &ast, "parse_error",
    (status_code as i64, body_string)
)?;
let error_map = result.try_cast::<Map>().ok_or_else(|| {
    ProviderError::api("custom", "parse_error must return a Map")
})?;

let error_type = error_map.get("type")
    .and_then(|v| v.clone().into_string().ok())
    .unwrap_or_else(|| "api".to_string());

let message = error_map.get("message")
    .and_then(|v| v.clone().into_string().ok())
    .unwrap_or_else(|| format!("HTTP {status_code}"));

let retry_after = error_map.get("retry_after_secs")
    .and_then(|v| v.as_int().ok())
    .map(|v| v as u64);

match error_type.as_str() {
    "auth" => Err(ProviderError::auth("custom", message)),
    "rate_limit" => Err(ProviderError::rate_limit("custom", message, retry_after)),
    "timeout" => Err(ProviderError::Timeout {
        provider: "custom".to_string(), message
    }),
    _ => Err(ProviderError::api("custom", message)),
}
```

**Expected Rhai return:**
```rhai
fn parse_error(status_code, body) {
    let parsed = json::parse(body);

    if status_code == 401 || status_code == 403 {
        return #{
            type: "auth",
            message: "Authentication failed: " + parsed.error.message
        };
    }

    if status_code == 429 {
        return #{
            type: "rate_limit",
            message: "Rate limited: " + parsed.error.message,
            retry_after_secs: 30
        };
    }

    #{
        type: "api",
        message: `HTTP ${status_code}: ${parsed.error.message}`
    }
}
```

---

### 4.7 `needs_api_key()` → bool

**Purpose:** Indicate whether the provider requires an API key from the environment.

**Rust calling pattern:**
```rust
let result: Dynamic = engine.call_fn(&mut scope, &ast, "needs_api_key", ())?;
let needs_key = result.as_bool()
    .map_err(|_| ProviderError::config("custom", "needs_api_key must return a bool"))?;
```

**Expected Rhai return:**
```rhai
fn needs_api_key() {
    true
}
```

**Error handling:** If this returns `true` and no API key is found in the environment, the provider fails at construction time with `ProviderError::Authentication`.

---

### Summary Table

| # | Function | Args | Returns | When Called |
|---|----------|------|---------|-------------|
| 1 | `provider_info()` | none | Map | Construction |
| 2 | `build_headers(config)` | Map | Map | Every request |
| 3 | `build_url(config)` | Map | String | Every request |
| 4 | `build_request(request)` | Map | Map | Every request |
| 5 | `parse_response(raw)` | Dynamic | Map | Success (2xx) |
| 6 | `parse_error(status, body)` | (i64, String) | Map | Error (non-2xx) |
| 7 | `needs_api_key()` | none | bool | Construction |

---

## 5. spawn_blocking Pattern

### 5.1 Why spawn_blocking is Required

Rhai's `Engine::call_fn()` is **synchronous**. The `LlmProvider` trait is **async**. The Rhai engine must not block the tokio runtime's async worker threads.

The solution: `tokio::task::spawn_blocking` moves Rhai execution to a dedicated blocking thread pool.

### 5.2 Existing Pattern from ScriptedOAuthProvider

From `codelet/providers/src/oauth/script_provider.rs:108-124`:

```rust
pub async fn build_authorization_request(&self) -> Result<Map> {
    let engine = self.engine.clone();     // Arc<Engine> — cheap clone
    let ast = self.ast.clone();           // AST clone
    let config = self.config_map();       // Dynamic value

    tokio::task::spawn_blocking(move || -> Result<Map> {
        let mut scope = Scope::new();
        let result: Dynamic = engine
            .call_fn(&mut scope, &ast, "build_authorization_request", (config,))
            .map_err(|e| anyhow!("build_authorization_request failed: {e}"))?;
        result.try_cast::<Map>().ok_or_else(|| {
            anyhow!("build_authorization_request must return a Map")
        })
    })
    .await
    .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
}
```

**Key observations:**

1. **`engine` is `Arc<Engine>`** — cloned cheaply (reference count increment only).
2. **`ast` is `AST`** — cloned directly. This is a full clone but AST has internal Arc sharing so it's reasonably cheap.
3. **`Scope::new()` inside the closure** — a fresh scope is created per call. No state leaks between calls.
4. **Double `?` unwrapping** — `spawn_blocking` returns `JoinResult<Result<Map>>`, so two error layers are handled.

### 5.3 Proposed Pattern for RhaiCustomProvider

For `RhaiCustomProvider`, the pattern should use `Arc<AST>` instead of cloning `AST` directly, and map to `ProviderError` instead of `anyhow`:

```rust
impl RhaiCustomProvider {
    /// Call a Rhai function on the blocking thread pool.
    ///
    /// Handles the async/sync bridge and error mapping.
    async fn call_rhai_fn<F, T>(
        &self,
        fn_name: &'static str,
        args_builder: F,
    ) -> Result<T, ProviderError>
    where
        F: FnOnce() -> rhai::FnCallArgs + Send + 'static,
        // ... This is conceptual — see concrete version below
    {
        // ...
    }
}
```

In practice, because `call_fn` takes different argument tuple types, a helper per-arity is cleanest:

```rust
/// Call a Rhai function with one argument.
async fn call_rhai_fn1(
    &self,
    fn_name: &'static str,
    arg: Dynamic,
) -> Result<Dynamic, ProviderError> {
    let engine = self.engine.clone();   // Arc<Engine>
    let ast = self.ast.clone();         // Arc<AST>

    tokio::task::spawn_blocking(move || -> Result<Dynamic, ProviderError> {
        let mut scope = Scope::new();
        engine
            .call_fn(&mut scope, &ast, fn_name, (arg,))
            .map_err(|e| map_rhai_error("custom", fn_name, *e))
    })
    .await
    .map_err(|e| ProviderError::api("custom", format!("spawn_blocking join failed: {e}")))?
}

/// Call a Rhai function with no arguments.
async fn call_rhai_fn0(
    &self,
    fn_name: &'static str,
) -> Result<Dynamic, ProviderError> {
    let engine = self.engine.clone();
    let ast = self.ast.clone();

    tokio::task::spawn_blocking(move || -> Result<Dynamic, ProviderError> {
        let mut scope = Scope::new();
        engine
            .call_fn(&mut scope, &ast, fn_name, ())
            .map_err(|e| map_rhai_error("custom", fn_name, *e))
    })
    .await
    .map_err(|e| ProviderError::api("custom", format!("spawn_blocking join failed: {e}")))?
}

/// Call a Rhai function with two arguments.
async fn call_rhai_fn2(
    &self,
    fn_name: &'static str,
    arg1: Dynamic,
    arg2: Dynamic,
) -> Result<Dynamic, ProviderError> {
    let engine = self.engine.clone();
    let ast = self.ast.clone();

    tokio::task::spawn_blocking(move || -> Result<Dynamic, ProviderError> {
        let mut scope = Scope::new();
        engine
            .call_fn(&mut scope, &ast, fn_name, (arg1, arg2))
            .map_err(|e| map_rhai_error("custom", fn_name, *e))
    })
    .await
    .map_err(|e| ProviderError::api("custom", format!("spawn_blocking join failed: {e}")))?
}
```

### 5.4 The Complete HTTP Lifecycle Flow

```rust
#[async_trait]
impl LlmProvider for RhaiCustomProvider {
    async fn complete_with_tools(
        &self,
        messages: &[codelet_common::Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse, ProviderError> {
        // 1. Build config map (api_key, base_url, etc.)
        let config = self.build_config_dynamic();

        // 2. Call build_url(config) via spawn_blocking
        let url_dyn = self.call_rhai_fn1("build_url", config.clone()).await?;
        let url = url_dyn.into_string()
            .map_err(|_| ProviderError::api("custom", "build_url must return a string"))?;

        // 3. Call build_headers(config) via spawn_blocking
        let headers_dyn = self.call_rhai_fn1("build_headers", config.clone()).await?;
        let headers = self.dynamic_to_header_map(headers_dyn)?;

        // 4. Build request Dynamic from messages + tools
        let request_dyn = build_request_dynamic(messages, tools)?;

        // 5. Call build_request(request) via spawn_blocking
        let body_dyn = self.call_rhai_fn1("build_request", request_dyn).await?;
        let body_json = dynamic_to_json_value(&body_dyn);
        let body_string = serde_json::to_string(&body_json)
            .map_err(|e| ProviderError::api("custom", format!("JSON serialize failed: {e}")))?;

        // 6. Make HTTP request (async, on tokio runtime)
        let response = self.http_client
            .post(&url)
            .headers(headers)
            .body(body_string)
            .send()
            .await
            .map_err(|e| ProviderError::api("custom", format!("HTTP request failed: {e}")))?;

        let status = response.status();
        let response_body = response.text().await
            .map_err(|e| ProviderError::api("custom", format!("Failed to read response: {e}")))?;

        // 7. Check status and route to parse_response or parse_error
        if status.is_success() {
            let response_json: serde_json::Value = serde_json::from_str(&response_body)
                .map_err(|e| ProviderError::api("custom", format!("JSON parse failed: {e}")))?;
            let raw_dyn = json_value_to_dynamic(&response_json);

            // 8. Call parse_response(raw) via spawn_blocking
            let result_dyn = self.call_rhai_fn1("parse_response", raw_dyn).await?;
            parse_script_response(result_dyn)
        } else {
            // 9. Call parse_error(status, body) via spawn_blocking
            let status_dyn = Dynamic::from(status.as_u16() as i64);
            let body_dyn = Dynamic::from(response_body.clone());
            let error_dyn = self.call_rhai_fn2(
                "parse_error", status_dyn, body_dyn
            ).await?;
            let error_map = error_dyn.try_cast::<Map>().ok_or_else(|| {
                ProviderError::api("custom", "parse_error must return a Map")
            })?;
            Err(map_error_response("custom", error_map))
        }
    }
}
```

### 5.5 Thread Safety Analysis

- **`Arc<Engine>`**: `Engine` is `Send + Sync`. The `Arc` allows cheap sharing across threads.
- **`Arc<AST>`**: `AST` is `Send + Sync + Clone`. Using `Arc` avoids full clones per request.
- **`Scope::new()` per call**: Fresh scope prevents state leakage. No mutable shared state.
- **`reqwest::Client`**: Uses `Arc` internally, clone is cheap.
- **`Dynamic`**: Is `Send` (with `sync` feature) or needs careful handling without it. Our sandboxed engine should use Rhai's `sync` feature.

---

## 6. Error Mapping

### 6.1 ProviderError Variants

From `codelet/providers/src/error.rs:13-46`:

```rust
pub enum ProviderError {
    Authentication { provider: String, message: String },
    Api { provider: String, message: String },
    RateLimit { provider: String, message: String, retry_after_secs: Option<u64> },
    Configuration { provider: String, message: String },
    Model { provider: String, message: String },
    Content { provider: String, message: String },
    Timeout { provider: String, message: String },
}
```

### 6.2 Rhai EvalAltResult Variants

From `/tmp/rhai/src/types/error.rs:27-129`, the key variants:

```rust
pub enum EvalAltResult {
    ErrorSystem(String, Box<dyn Error>),
    ErrorParsing(ParseErrorType, Position),
    ErrorFunctionNotFound(String, Position),
    ErrorInFunctionCall(String, String, Box<Self>, Position),
    ErrorVariableNotFound(String, Position),
    ErrorRuntime(Dynamic, Position),
    ErrorTooManyOperations(Position),
    ErrorStackOverflow(Position),
    ErrorDataTooLarge(String, Position),
    ErrorTerminated(Dynamic, Position),
    // ... and many more
}
```

### 6.3 Proposed Error Mapping Function

```rust
/// Map a Rhai EvalAltResult to a ProviderError.
///
/// Categories:
/// - Script bugs (syntax, missing functions, type errors) → Configuration
/// - Runtime errors (thrown by script logic) → Api
/// - Resource limits (too many ops, stack overflow) → Timeout
/// - System errors → Api
fn map_rhai_error(
    provider: &str,
    fn_name: &str,
    error: rhai::EvalAltResult,
) -> ProviderError {
    match &error {
        // Missing function = misconfigured script
        rhai::EvalAltResult::ErrorFunctionNotFound(sig, _) => {
            ProviderError::config(
                provider,
                format!("Script missing required function: {sig}"),
            )
        }

        // Syntax/parse errors = misconfigured script
        rhai::EvalAltResult::ErrorParsing(_, _) => {
            ProviderError::config(
                provider,
                format!("Script syntax error in '{fn_name}': {error}"),
            )
        }

        // Variable not found = script bug
        rhai::EvalAltResult::ErrorVariableNotFound(var, _) => {
            ProviderError::config(
                provider,
                format!("Script variable not found in '{fn_name}': {var}"),
            )
        }

        // Resource limits = treat as timeout
        rhai::EvalAltResult::ErrorTooManyOperations(_) => {
            ProviderError::Timeout {
                provider: provider.to_string(),
                message: format!(
                    "Script '{fn_name}' exceeded max operations ({})",
                    super::engine::MAX_OPERATIONS
                ),
            }
        }

        rhai::EvalAltResult::ErrorStackOverflow(_) => {
            ProviderError::Timeout {
                provider: provider.to_string(),
                message: format!("Script '{fn_name}' stack overflow"),
            }
        }

        rhai::EvalAltResult::ErrorDataTooLarge(typ, _) => {
            ProviderError::Timeout {
                provider: provider.to_string(),
                message: format!("Script '{fn_name}' data too large: {typ}"),
            }
        }

        // Terminated (script kill) = timeout
        rhai::EvalAltResult::ErrorTerminated(_, _) => {
            ProviderError::Timeout {
                provider: provider.to_string(),
                message: format!("Script '{fn_name}' was terminated"),
            }
        }

        // Runtime errors from script throw() or general errors
        rhai::EvalAltResult::ErrorRuntime(msg, _) => {
            ProviderError::api(
                provider,
                format!("Script '{fn_name}' runtime error: {msg}"),
            )
        }

        // Errors inside called functions (nested)
        rhai::EvalAltResult::ErrorInFunctionCall(called_fn, _, inner, _) => {
            // Recurse to get the inner error mapped properly
            let inner_err = map_rhai_error(provider, called_fn, *inner.clone());
            // But wrap with context about the calling chain
            match inner_err {
                ProviderError::Configuration { message, .. } => {
                    ProviderError::config(provider, message)
                }
                ProviderError::Timeout { message, .. } => {
                    ProviderError::Timeout {
                        provider: provider.to_string(),
                        message,
                    }
                }
                other => other,
            }
        }

        // Type mismatch errors = script bug
        rhai::EvalAltResult::ErrorMismatchDataType(expected, actual, _) => {
            ProviderError::config(
                provider,
                format!(
                    "Script '{fn_name}' type error: expected {expected}, got {actual}"
                ),
            )
        }

        // All other errors → generic API error
        _ => {
            ProviderError::api(
                provider,
                format!("Script '{fn_name}' failed: {error}"),
            )
        }
    }
}
```

### 6.4 Error from parse_error Script Response

```rust
/// Map the parse_error script response to a ProviderError.
fn map_error_response(provider: &str, error_map: Map) -> ProviderError {
    let error_type = error_map.get("type")
        .and_then(|v| v.clone().into_string().ok())
        .unwrap_or_else(|| "api".to_string());

    let message = error_map.get("message")
        .and_then(|v| v.clone().into_string().ok())
        .unwrap_or_else(|| "Unknown error".to_string());

    let retry_after = error_map.get("retry_after_secs")
        .and_then(|v| v.as_int().ok())
        .map(|v| v as u64);

    match error_type.as_str() {
        "auth" => ProviderError::auth(provider, message),
        "rate_limit" => ProviderError::rate_limit(provider, message, retry_after),
        "timeout" => ProviderError::Timeout {
            provider: provider.to_string(),
            message,
        },
        "config" => ProviderError::config(provider, message),
        "model" => ProviderError::Model {
            provider: provider.to_string(),
            message,
        },
        "content" => ProviderError::Content {
            provider: provider.to_string(),
            message,
        },
        _ => ProviderError::api(provider, message),
    }
}
```

---

## 7. RhaiHttpClient

### 7.1 The HttpClientExt Trait

From `codelet/patches/rig-core/src/http_client/mod.rs:111-139`:

```rust
pub trait HttpClientExt: WasmCompatSend + WasmCompatSync {
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        T: Into<Bytes>,
        T: WasmCompatSend,
        U: From<Bytes>,
        U: WasmCompatSend + 'static;

    fn send_multipart<U>(
        &self,
        req: Request<MultipartForm>,
    ) -> impl Future<Output = Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        U: From<Bytes>,
        U: WasmCompatSend + 'static;

    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = Result<StreamingResponse>> + WasmCompatSend
    where
        T: Into<Bytes>;
}
```

### 7.2 Why We Don't Need a Custom HttpClientExt

For `RhaiCustomProvider`, we do **not** need to implement `HttpClientExt`. Here's why:

The `HttpClientExt` trait is part of rig's internal plumbing — it's used by rig's provider implementations (`anthropic::Client`, `openai::Client`, etc.) when they build requests through rig's `CompletionModel` trait. These providers use rig's builder pattern:

```
rig Client → CompletionModel → CompletionRequest → HttpClientExt::send()
```

But `RhaiCustomProvider` implements `LlmProvider` directly, bypassing rig's internal provider machinery entirely. The HTTP lifecycle is:

```
LlmProvider::complete_with_tools()
  → Rhai: build_url() + build_headers() + build_request()
  → reqwest::Client::post()  (direct HTTP, no rig Client)
  → Rhai: parse_response() or parse_error()
```

The `reqwest::Client` already implements `HttpClientExt` (see `http_client/mod.rs:141`), but we don't even use that trait. We use `reqwest` directly because:

1. The script controls URL, headers, and body format
2. There's no rig `Client` or `CompletionModel` in the picture
3. Direct reqwest gives us full control over the request/response cycle

### 7.3 Where HttpClientExt IS Relevant: RefreshingHttpClient

The existing `RefreshingHttpClient<S>` pattern from `codelet/providers/src/oauth/http_middleware.rs:142-238` shows how `HttpClientExt` is used as middleware:

```rust
impl<S: TokenStrategy> rig::http_client::HttpClientExt for RefreshingHttpClient<S> {
    fn send<T, U>(&self, req: http::Request<T>) -> impl Future<...> {
        let this = self.clone();
        let req = req.map(Into::into);
        async move {
            let req = this.ensure_and_prepare(req).await?;
            this.inner.send(req).await     // delegates to reqwest
        }
    }
    // ...
}
```

This pattern is useful when wrapping rig's built-in providers with custom HTTP behavior (token refresh, header injection). But for `RhaiCustomProvider`, we build the entire request from scratch in Rhai, so this middleware layer isn't needed.

### 7.4 Future Consideration: Rhai-Driven HttpClientExt

If in the future a `RhaiCustomProvider` needs to work WITH rig's provider machinery (e.g., wrapping an existing rig provider with Rhai-modified headers), we could implement `HttpClientExt` as an interceptor:

```rust
/// Hypothetical Rhai-driven HTTP middleware.
/// NOT needed for PROV-063 but documented for future reference.
#[derive(Clone)]
struct RhaiHttpMiddleware {
    inner: reqwest::Client,
    engine: Arc<Engine>,
    ast: Arc<AST>,
}

impl HttpClientExt for RhaiHttpMiddleware {
    fn send<T, U>(&self, req: Request<T>) -> impl Future<Output = Result<...>> {
        let engine = self.engine.clone();
        let ast = self.ast.clone();
        let inner = self.inner.clone();
        let req = req.map(Into::into);

        async move {
            // Call Rhai to modify headers
            let (parts, body) = req.into_parts();
            let mut headers_map = Map::new();
            for (name, value) in &parts.headers {
                if let Ok(v) = value.to_str() {
                    headers_map.insert(
                        name.as_str().into(),
                        Dynamic::from(v.to_string()),
                    );
                }
            }

            let modified = tokio::task::spawn_blocking(move || {
                let mut scope = Scope::new();
                engine.call_fn(
                    &mut scope, &ast,
                    "modify_headers",
                    (Dynamic::from_map(headers_map),),
                )
            }).await??;

            // Rebuild request with modified headers
            // ... (omitted for brevity)

            inner.send(modified_req).await
        }
    }
}
```

### 7.5 Recommended Approach for PROV-063

Use `reqwest::Client` directly. The complete_with_tools implementation (Section 5.4) shows the flow:

1. Rhai builds URL, headers, body → all returned as Dynamic values
2. Rust converts Dynamic headers to `reqwest::header::HeaderMap`
3. `reqwest::Client::post()` sends the request
4. Rhai parses the response or error

No `HttpClientExt` implementation needed.

---

## Appendix A: Engine Configuration

### A.1 Sandboxed Engine (from PROV-060)

From `codelet/providers/src/oauth/engine.rs:42-58`:

```rust
pub fn build_sandboxed_engine(modules: Vec<RhaiModule>) -> Engine {
    let mut engine = Engine::new_raw();

    // Safety limits
    engine.set_max_operations(50_000);
    engine.set_max_call_levels(32);
    engine.set_max_string_size(1_048_576);  // 1 MB
    engine.set_max_array_size(10_000);
    engine.set_max_map_size(10_000);

    for rhai_module in modules {
        engine.register_static_module(&rhai_module.name, rhai_module.module.into());
    }

    engine
}
```

### A.2 Available Building Block Modules

From `codelet/providers/src/oauth/building_blocks.rs:14-21`:

| Module | Functions | Source |
|--------|-----------|--------|
| `http` | `post(url, body, headers)`, `get(url, headers)` | `building_blocks.rs:26-93` |
| `crypto` | `sha256(data)`, `base64url_encode(data)` | `building_blocks.rs:96-126` |
| `json` | `parse(s)`, `stringify(value)` | `building_blocks.rs:129-165` |
| `oauth` | `generate_pkce()`, `generate_state()`, `urlencoded(s)` | `building_blocks.rs:168-213` |

### A.3 PROV-063 Engine Extension

The custom provider engine should extend the default modules with additional ones for LLM-specific operations:

```rust
pub fn build_custom_provider_engine() -> Engine {
    let mut modules = super::building_blocks::register_all_modules();

    // Additional modules for custom providers could include:
    // - base64 encoding/decoding for multimodal content
    // - timestamp helpers for request signing
    // But the existing http, json, crypto modules cover most needs

    build_sandboxed_engine(modules)
}
```

---

## Appendix B: json_value_to_dynamic / dynamic_to_json_value

These conversion helpers exist in `codelet/providers/src/oauth/building_blocks.rs:216-271` and should be extracted to a shared utility module since both the OAuth scripts and custom provider scripts need them.

**json_value_to_dynamic** (`building_blocks.rs:216-242`):

```rust
fn json_value_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Dynamic::from(i) }
            else if let Some(f) = n.as_f64() { Dynamic::from(f) }
            else { Dynamic::UNIT }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            Dynamic::from_array(arr.iter().map(json_value_to_dynamic).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_value_to_dynamic(v));
            }
            Dynamic::from_map(map)
        }
    }
}
```

**dynamic_to_json_value** (`building_blocks.rs:245-271`):

```rust
fn dynamic_to_json_value(value: &Dynamic) -> serde_json::Value {
    if value.is_unit() { serde_json::Value::Null }
    else if let Ok(b) = value.as_bool() { serde_json::Value::Bool(b) }
    else if let Ok(i) = value.as_int() { serde_json::Value::Number(i.into()) }
    else if let Ok(f) = value.as_float() {
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    }
    else if let Ok(s) = value.clone().into_string() { serde_json::Value::String(s) }
    else if value.is_array() {
        let arr = value.clone().into_typed_array::<Dynamic>().unwrap_or_default();
        serde_json::Value::Array(arr.iter().map(dynamic_to_json_value).collect())
    }
    else if value.is_map() {
        let map = value.clone().cast::<Map>();
        let mut obj = serde_json::Map::new();
        for (k, v) in &map { obj.insert(k.to_string(), dynamic_to_json_value(v)); }
        serde_json::Value::Object(obj)
    }
    else { serde_json::Value::Null }
}
```

---

## Appendix C: Complete Example Script

```rhai
// my-provider.rhai — Custom LLM provider for MyAPI

fn provider_info() {
    #{
        name: "my-provider",
        model: "my-model-v2",
        context_window: 128000,
        max_output_tokens: 4096,
        supports_caching: false,
        supports_streaming: false
    }
}

fn needs_api_key() {
    true
}

fn build_url(config) {
    config.base_url + "/v1/chat/completions"
}

fn build_headers(config) {
    #{
        "Authorization": "Bearer " + config.api_key,
        "Content-Type": "application/json"
    }
}

fn build_request(request) {
    let messages = [];
    for msg in request.messages {
        messages.push(#{
            role: msg.role,
            content: if type_of(msg.content) == "string" {
                msg.content
            } else {
                // Convert content parts to provider format
                let parts = [];
                for part in msg.content {
                    if part.type == "text" {
                        parts.push(#{ type: "text", text: part.text });
                    } else if part.type == "tool_use" {
                        parts.push(#{
                            type: "function",
                            id: part.id,
                            function: #{
                                name: part.name,
                                arguments: json::stringify(part.input)
                            }
                        });
                    } else if part.type == "tool_result" {
                        parts.push(#{
                            type: "tool",
                            tool_call_id: part.tool_use_id,
                            content: part.content
                        });
                    }
                }
                parts
            }
        });
    }

    let body = #{
        model: "my-model-v2",
        messages: messages,
        max_tokens: 4096
    };

    if request.tools.len() > 0 {
        let tools = [];
        for t in request.tools {
            tools.push(#{
                type: "function",
                function: #{
                    name: t.name,
                    description: t.description,
                    parameters: t.input_schema
                }
            });
        }
        body.tools = tools;
    }

    body
}

fn parse_response(raw) {
    let choice = raw.choices[0];
    let msg = choice.message;
    let content = [];

    if msg.content != () {
        content.push(#{ type: "text", text: msg.content });
    }

    if "tool_calls" in msg {
        for tc in msg.tool_calls {
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
        stop_reason: switch choice.finish_reason {
            "stop" => "end_turn",
            "tool_calls" => "tool_use",
            "length" => "max_tokens",
            _ => "end_turn"
        }
    }
}

fn parse_error(status_code, body) {
    let parsed = json::parse(body);
    let msg = if "error" in parsed {
        parsed.error.message
    } else {
        body
    };

    if status_code == 401 || status_code == 403 {
        return #{ type: "auth", message: msg };
    }
    if status_code == 429 {
        return #{ type: "rate_limit", message: msg, retry_after_secs: 30 };
    }
    #{ type: "api", message: `HTTP ${status_code}: ${msg}` }
}
```

---

## Appendix D: Source File Reference

| File | Location | Key Content |
|------|----------|-------------|
| LlmProvider trait | `codelet/providers/src/lib.rs:90-118` | Trait definition, CompletionResponse, StopReason |
| ClaudeProvider | `codelet/providers/src/claude.rs:152-745` | Reference implementation |
| ProviderError | `codelet/providers/src/error.rs:13-130` | Error variants and helpers |
| ProviderAdapter | `codelet/providers/src/adapter.rs:190-231` | Shared adapter trait |
| CompletionRequest | `codelet/patches/rig-core/src/completion/request.rs:497-515` | Rig request struct |
| AssistantContent | `codelet/patches/rig-core/src/completion/message.rs:63-69` | Rig response content types |
| HttpClientExt | `codelet/patches/rig-core/src/http_client/mod.rs:111-139` | HTTP client trait |
| RefreshingHttpClient | `codelet/providers/src/oauth/http_middleware.rs:56-238` | HTTP middleware pattern |
| ScriptedOAuthProvider | `codelet/providers/src/oauth/script_provider.rs:45-225` | Reference Rhai integration |
| Sandboxed Engine | `codelet/providers/src/oauth/engine.rs:42-58` | Engine factory |
| Building Blocks | `codelet/providers/src/oauth/building_blocks.rs:14-271` | Rhai modules |
| Message types | `codelet/common/src/types.rs:1-86` | Shared message types |
| ToolDefinition | `codelet/tools/src/lib.rs:216-223` | Tool definition struct |
| ModelLimitsResolver | `codelet/providers/src/model_limits.rs:27-58` | Limits trait |
| Rhai to_dynamic | `/tmp/rhai/src/serde/ser.rs:83-86` | Serialization entry point |
| Rhai from_dynamic | `/tmp/rhai/src/serde/de.rs:107-109` | Deserialization entry point |
| Rhai EvalAltResult | `/tmp/rhai/src/types/error.rs:27-129` | Error enum |
