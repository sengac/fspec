# PROV-055 — AST Research: Copilot Middleware, Facade & Endpoint Routing Patterns

**Work Unit:** PROV-055 — GitHub Copilot HTTP middleware, facades & endpoint routing
**Date:** 2026-04-07
**Purpose:** Catalog the existing Rust patterns in `codelet/providers/` and `codelet/tools/src/facade/`
that PROV-055 must mirror. These patterns were located with `AstGrep` + `Grep` and are the
canonical templates for the new Copilot modules.

---

## 1. HTTP Middleware Pattern — `rig::http_client::HttpClientExt`

**Template file:** `codelet/providers/src/claude_refreshing_client.rs`

**AstGrep query:**
```
pattern: impl rig::http_client::HttpClientExt for $NAME { $$$BODY }
language: rust
```

**Match:**
- `codelet/providers/src/claude_refreshing_client.rs:211:1` — `impl rig::http_client::HttpClientExt for RefreshingClaudeClient`

**Required methods (from claude_refreshing_client.rs:211-294):**
1. `fn send<T, U>(...)` — standard request interception
2. `fn send_multipart<U>(...)` — multipart upload interception (unused for Copilot — image upload goes via chat/completions JSON body, not multipart)
3. `fn send_streaming<T>(...)` — SSE streaming interception (REQUIRED for Copilot — all chat/completions and /responses traffic is SSE)

**Shared mechanics:**
- `self.inner: reqwest::Client` wrapped for pass-through
- `self.mode: CopilotTokenMode` enum (OAuth with `Arc<RwLock<CopilotTokenState>>`, or pass-through)
- `ensure_fresh_token()` double-check locking pattern with 30 s expiry buffer
- `prepare_request()` strips existing `Authorization` and injects fresh Bearer + all Copilot headers
- `tokio::spawn` to persist refreshed tokens to `copilot_auth.json` outside the lock (best-effort)

**Difference from Claude:**
- Claude middleware only sets `Authorization`; static headers set at rig client build time
- Copilot middleware must inject 5 headers on EVERY request: `x-initiator`, `User-Agent`,
  `Authorization`, `Openai-Intent`, conditional `Copilot-Vision-Request`
- Copilot also does endpoint routing (chat/completions vs /responses) based on model ID — the
  URL rewrite must happen here or upstream of rig's URL builder

---

## 2. Header Facade Pattern — `CacheOptimizationFacade::build_headers`

**Template file:** `codelet/providers/src/cache_optimization.rs:96`

**AstGrep query:**
```
pattern: pub fn build_headers($$$PARAMS) -> HeaderMap { $$$BODY }
language: rust
```

**Match signature:**
```rust
pub fn build_headers(config: &SessionAffinityConfig) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if config.should_apply() {
        let value = config.affinity_value();
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            headers.insert(HeaderName::from_static("x-session-affinity"), header_value);
        }
    }
    headers
}
```

**Copilot analog:**
```rust
// codelet/providers/src/copilot/header_facade.rs
pub struct CopilotHeaderFacade;

impl CopilotHeaderFacade {
    pub fn build_headers(
        classification: &RequestClassification,
        access_token: &str,
    ) -> HeaderMap {
        let mut headers = HeaderMap::new();
        // x-initiator
        headers.insert(
            HeaderName::from_static("x-initiator"),
            HeaderValue::from_static(if classification.is_agent { "agent" } else { "user" }),
        );
        // User-Agent: codelet/<version>
        if let Ok(ua) = HeaderValue::from_str(&format!("codelet/{}", env!("CARGO_PKG_VERSION"))) {
            headers.insert(http::header::USER_AGENT, ua);
        }
        // Authorization: Bearer <token>
        if let Ok(auth) = HeaderValue::from_str(&format!("Bearer {access_token}")) {
            headers.insert(http::header::AUTHORIZATION, auth);
        }
        // Openai-Intent
        headers.insert(
            HeaderName::from_static("openai-intent"),
            HeaderValue::from_static("conversation-edits"),
        );
        // Copilot-Vision-Request (conditional)
        if classification.is_vision {
            headers.insert(
                HeaderName::from_static("copilot-vision-request"),
                HeaderValue::from_static("true"),
            );
        }
        headers
    }
}
```

**Test pattern** (from `cache_optimization.rs:140-226`): pure unit tests, no HTTP client —
construct `SessionAffinityConfig`, call `build_headers`, assert header presence/absence.

---

## 3. Behavior Facade Trait Pattern — `ThinkingConfigFacade`

**Template file:** `codelet/tools/src/facade/thinking_config.rs:76`

**Trait signature:**
```rust
pub trait ThinkingConfigFacade {
    fn provider(&self) -> &'static str;
    fn request_config(&self, level: ThinkingLevel) -> Value;
    fn is_thinking_part(&self, part: &Value) -> bool;
    fn extract_thinking_text(&self, part: &Value) -> Option<String>;
}
```

**Implementations in same file:** `Gemini3ThinkingFacade` (line 93), plus Claude/OpenAI variants.

**Copilot analog:**
```rust
// codelet/providers/src/copilot/behavior_facade.rs
pub trait CopilotBehaviorFacade: Send + Sync {
    fn family(&self) -> &'static str;
    fn reasoning_effort_variants(&self) -> &'static [&'static str];
    fn mutate_chat_params(&self, params: &mut Value);
    fn extract_reasoning_opaque(&self, response: &Value) -> Option<Value>;
    fn inject_reasoning_opaque(&self, next_request: &mut Value, blob: &Value);
}

pub struct CopilotGptBehaviorFacade;
pub struct CopilotClaudeBehaviorFacade;
pub struct CopilotGeminiBehaviorFacade;

impl CopilotBehaviorFacade for CopilotGptBehaviorFacade { /* ... */ }
impl CopilotBehaviorFacade for CopilotClaudeBehaviorFacade { /* ... */ }
impl CopilotBehaviorFacade for CopilotGeminiBehaviorFacade { /* ... */ }
```

---

## 4. Selector Function Pattern — `select_claude_facade`

**Template file:** `codelet/tools/src/facade/system_prompt.rs:427`

**Grep match:**
```
codelet/tools/src/facade/system_prompt.rs:427:pub fn select_claude_facade(is_oauth: bool) -> BoxedSystemPromptFacade {
```

**Signature:**
```rust
pub type BoxedSystemPromptFacade = Box<dyn SystemPromptFacade>;

pub fn select_claude_facade(is_oauth: bool) -> BoxedSystemPromptFacade {
    if is_oauth {
        Box::new(ClaudeOAuthSystemPromptFacade)
    } else {
        Box::new(ClaudeApiKeySystemPromptFacade)
    }
}
```

**Copilot analog:**
```rust
// codelet/providers/src/copilot/behavior_facade.rs
pub type BoxedCopilotBehaviorFacade = Box<dyn CopilotBehaviorFacade>;

pub fn select_copilot_behavior_facade(model_id: &str) -> BoxedCopilotBehaviorFacade {
    if model_id.starts_with("gpt-") {
        Box::new(CopilotGptBehaviorFacade)
    } else if model_id.starts_with("claude-") {
        Box::new(CopilotClaudeBehaviorFacade)
    } else if model_id.starts_with("gemini-") {
        Box::new(CopilotGeminiBehaviorFacade)
    } else {
        // Default to GPT for unknown prefixes (gpt-4o-copilot, etc.)
        Box::new(CopilotGptBehaviorFacade)
    }
}
```

**Test coverage:** `system_prompt.rs:433-448` — unit test asserts provider() and identity_prefix().

---

## 5. Endpoint Facade Pattern (NEW — no existing template, pure function)

```rust
// codelet/providers/src/copilot/endpoint.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopilotEndpoint {
    ChatCompletions,
    Responses,
}

pub struct CopilotEndpointFacade;

impl CopilotEndpointFacade {
    /// Select endpoint based on model ID per slice2 memo rule 5:
    ///   - gpt-N where N >= 5 AND not gpt-5-mini → Responses
    ///   - otherwise → ChatCompletions
    pub fn select(model_id: &str) -> CopilotEndpoint {
        // Explicit exclusion: gpt-5-mini → ChatCompletions
        if model_id == "gpt-5-mini" {
            return CopilotEndpoint::ChatCompletions;
        }
        // Match gpt-N pattern and extract N
        if let Some(rest) = model_id.strip_prefix("gpt-") {
            // Extract leading integer
            let n_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = n_str.parse::<u32>() {
                if n >= 5 {
                    return CopilotEndpoint::Responses;
                }
            }
        }
        CopilotEndpoint::ChatCompletions
    }
}
```

---

## 6. Request Classifier Pattern (NEW — pure function)

```rust
// codelet/providers/src/copilot/classifier.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestClassification {
    pub is_vision: bool,
    pub is_agent: bool,
}

pub struct CopilotRequestClassifier;

impl CopilotRequestClassifier {
    pub fn classify(body: &serde_json::Value) -> RequestClassification {
        let is_vision = detect_vision_content(body);
        let is_agent = detect_agent_mode(body);
        RequestClassification { is_vision, is_agent }
    }
}

fn detect_vision_content(body: &serde_json::Value) -> bool {
    // Walk "messages" array; for each message check "content" array for
    // items with "type" == "image_url" or "image" (chat/completions schema)
    // OR walk "input" array for "input_image" items (/responses schema)
    if let Some(messages) = body.get("messages").and_then(|v| v.as_array()) {
        for msg in messages {
            if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                for item in content {
                    if let Some(t) = item.get("type").and_then(|v| v.as_str()) {
                        if t == "image_url" || t == "image" || t == "input_image" {
                            return true;
                        }
                    }
                }
            }
        }
    }
    if let Some(input) = body.get("input").and_then(|v| v.as_array()) {
        for item in input {
            if let Some(t) = item.get("type").and_then(|v| v.as_str()) {
                if t == "input_image" {
                    return true;
                }
            }
        }
    }
    false
}

fn detect_agent_mode(body: &serde_json::Value) -> bool {
    // Per slice2 memo: agent mode is signaled by a metadata marker the TUI
    // injects when running autonomous workflows. Exact marker TBD from
    // session state, but classifier reads it from body["metadata"]["mode"]
    body.get("metadata")
        .and_then(|m| m.get("mode"))
        .and_then(|v| v.as_str())
        .map(|s| s == "agent")
        .unwrap_or(false)
}
```

---

## 7. Provider Registration Pattern — `ProviderType` enum

**Template file:** `codelet/providers/src/manager.rs:18-70`

**Current enum (manager.rs:20-26):**
```rust
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
    ZAI,
}
```

**Registration touchpoints to extend for Copilot:**
- `ProviderType::GitHubCopilot` variant (line 20-26)
- `FromStr::from_str` match arm: `"github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot)` (line 33-37)
- `as_str()` match arm: `ProviderType::GitHubCopilot => "github-copilot"` (line 50-55)
- `has_credentials()` match arm: `ProviderType::GitHubCopilot => credentials.has_copilot()` (line 62-67)
- `ProviderCredentials::has_copilot()` — new method in `credentials.rs` that checks `read_copilot_auth_sync()` (PROV-054 already persists the file)
- `map_provider_id_to_type()` for models.dev mapping (line 334-339)

---

## 8. Integration Test Pattern — wiremock

**Template file:** `codelet/providers/tests/claude_headless_login_test.rs` (shown via grep)

**Pattern:**
```rust
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_copilot_chat_request_routes_to_chat_completions() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("x-initiator", "user"))
        .and(header("openai-intent", "conversation-edits"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n"
        ))
        .expect(1)
        .mount(&mock_server)
        .await;
    // ... drive CopilotProvider against mock_server.uri() ...
}
```

---

## 9. Cargo Dependency Confirmation

**codelet/providers/Cargo.toml:68:** `wiremock = "0.6.5"` — already a dev-dependency, usable for PROV-055 tests.

Other relevant workspace deps already present:
- `rig-core.workspace = true`
- `reqwest.workspace = true`
- `http = "1"`
- `serde_json.workspace = true`
- `tokio.workspace = true`

---

## 10. Module Layout (target)

```
codelet/providers/src/copilot/
├── mod.rs                  # Re-exports (already exists from PROV-054)
├── auth.rs                 # (exists — PROV-054)
├── oauth.rs                # (exists — PROV-054)
├── refreshing_client.rs    # NEW — CopilotHttpClient middleware (template: claude_refreshing_client.rs)
├── header_facade.rs        # NEW — CopilotHeaderFacade (template: cache_optimization.rs:96)
├── classifier.rs           # NEW — CopilotRequestClassifier (pure fn)
├── endpoint.rs             # NEW — CopilotEndpointFacade (pure fn)
├── behavior_facade.rs      # NEW — CopilotBehaviorFacade trait + 3 impls + selector
└── provider.rs             # NEW — CopilotProvider (impl LlmProvider)
```

Plus edits to:
- `codelet/providers/src/manager.rs` — register `ProviderType::GitHubCopilot`
- `codelet/providers/src/credentials.rs` — add `has_copilot()` method
- `codelet/providers/src/lib.rs` — re-export `CopilotProvider`
- `codelet/tools/src/facade/system_prompt.rs` — add `CopilotResponsesSystemPromptFacade`

---

## Summary

All patterns required for PROV-055 exist in the codebase as direct templates. The implementation
is **structurally straightforward** — it is mechanical translation of the Claude/Codex/OpenAI
middleware and facade patterns into Copilot-specific variants, with two **novel** pure functions
(`CopilotEndpointFacade::select`, `CopilotRequestClassifier::classify`) that have no existing
template but are simple enough to define from first principles per the slice2 memo.

**Risk areas** identified during research:
1. **Agent mode detection** — the body-level marker for "is_agent" is not yet defined in the
   rest of the codebase; may need coordination with TUI layer to inject `metadata.mode = "agent"`.
2. **/responses streaming semantics** — GPT-5 `reasoning_opaque` round-trip requires intercepting
   the SSE stream, extracting the blob, and injecting it back on the next turn. This spans
   multiple turns, so state lives outside the middleware — likely in `codelet-core` session state.
