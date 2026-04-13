# PROV-059 Research: Copilot x-initiator Header — Premium Quota Burn

## Source: OpenCode Issue #8030

**URL:** https://github.com/anomalyco/opencode/issues/8030
**Title:** "Copilot auth now sets far too many requests as 'user' consuming premium requests rapidly"
**Status:** Open (reopened), 210 comments, 86 reactions
**Assigned:** @rekram1-node

---

## The Problem

GitHub Copilot uses the `X-Initiator` HTTP header to classify billing:

| Header Value | Billing | Description |
|---|---|---|
| `X-Initiator: user` | **Premium** (counts against quota) | Genuine user-initiated requests |
| `X-Initiator: agent` | **Free** | Tool calls, agent continuations, subagent work |

When the header is incorrectly set to `"user"` for agent-initiated requests, every API call burns premium quota — sometimes consuming half a monthly quota in a single conversation.

---

## How OpenCode Was Affected

### Root Cause: Synthetic "user" Messages

**File:** `packages/opencode/src/session/message-v2.ts` — `toModelMessagesEffect()`

OpenCode creates synthetic `role: "user"` messages for:
1. **Tool attachments** — tool results wrapped as user messages
2. **Compaction** — `"What did we do so far?"` as a user message
3. **Subtasks** — `"The following tool was executed by the user"` as a user message
4. **Image reads** — media from tool results injected as user messages

### The Detection Logic

**File:** `packages/opencode/src/plugin/github-copilot/copilot.ts`

The `fetch` wrapper inspects the last message in the request body:
```typescript
const last = body.messages[body.messages.length - 1]
return { isAgent: last?.role !== "user" }
```

If the last message has `role: "user"` (even if synthetic), the request is classified as user-initiated (premium).

### Additional Quota Drains
- **Session title generation** — each user prompt triggers a separate title-generation API call
- **Subagent requests** — subordinate sessions' requests counted as user-initiated
- **Compaction model** — summarization using premium model

### OpenCode Fixes Applied
1. **`chat.headers` plugin hook** — overrides `x-initiator` to `"agent"` for compaction and subagent sessions
2. **Commit `19fe3e2`** — subagent fix (forces `x-initiator: agent` for child sessions)
3. **Commit `c284469`** — image read fix
4. **PR #8721** — improved detection logic checking if the last user message is synthetic
5. **VSCode reference implementation:** `userInitiatedRequest: iterationNumber === 0 && !isContinuation && !isSubagent` (from `toolCallingLoop.ts:469`)

---

## How fspec/codelet Is Affected — CRITICAL

### The Three-Layer Infrastructure (Built ✅, Connected ❌)

The codelet Copilot provider has a well-designed three-layer architecture for handling this exact problem:

#### Layer 1: Classifier
**File:** `codelet/providers/src/copilot/classifier.rs:91-96`
```rust
fn detect_agent_mode(body: &Value) -> bool {
    body.get("metadata")
        .and_then(|m| m.get("mode"))
        .and_then(Value::as_str)
        .is_some_and(|s| s == "agent")
}
```

#### Layer 2: Header Facade
**File:** `codelet/providers/src/copilot/header_facade.rs:83-91`
```rust
headers.insert(
    HEADER_X_INITIATOR,
    if classification.is_agent { INITIATOR_AGENT }   // "agent"
    else { INITIATOR_USER },                          // "user" ← ALWAYS THIS PATH
);
```

#### Layer 3: HTTP Middleware
**File:** `codelet/providers/src/copilot/refreshing_client.rs`
```rust
let (classification, new_body) = classify_and_cache_body(req.body().clone());
let req = inject_copilot_headers(req, &classification, &access_token);
```

### 🔴 CRITICAL BUG: `metadata.mode = "agent"` Is Never Set

The classifier checks for `body.metadata.mode == "agent"`, but **no production code ever sets this field**.

Exhaustive search confirms `metadata.mode = "agent"` only appears in **test files**:
- `classifier.rs:162` (unit test)
- `refreshing_client_tests.rs:49` (unit test)
- `copilot_http_middleware_routing_test.rs:322` (integration test)

The `CopilotProvider::complete_with_tools()` at `provider.rs:449-455`:
```rust
let mut builder = CompletionRequestBuilder::new(self.completion_model.clone(), prompt)
    .max_tokens(crate::copilot::MAX_OUTPUT_TOKENS as u64)
    .tools(rig_tools);
// ← Missing: .additional_params(json!({"metadata": {"mode": "agent"}}))
```

**Result: Every Copilot request gets `x-initiator: user` → every request burns premium quota.**

### All Affected Request Sources

| Request Type | File | Expected `x-initiator` | Actual | Impact |
|---|---|---|---|---|
| User typing a message | `provider.rs:449` | `user` | `user` ✅ | Correct |
| Tool call follow-up turn | `provider.rs:449` | `agent` | **`user`** ❌ | **Over-charged** |
| DeepSearch sub-agents | `deep_search_provider_config.rs:67` | `agent` | **`user`** ❌ | **Over-charged** |
| Compaction continuation | `session_manager.rs:5359` | `agent` | **`user`** ❌ | **Over-charged** |
| Subordinate sessions | `agent_manager_handler.rs` | `agent` | **`user`** ❌ | **Over-charged** |
| Scheduled jobs | `scheduler/agent_job.rs` | `agent` | **`user`** ❌ | **Over-charged** |

### DeepSearch Specifically

**File:** `codelet/napi/src/deep_search_provider_config.rs:60-70`
```rust
fn copilot_request_config(model_name: &str, system_prompt: &str) -> DeepSearchRequestConfig {
    let facade = select_copilot_facade(model_name);
    DeepSearchRequestConfig {
        preamble: facade.transform_preamble(system_prompt),
        additional_params: None,  // ← No metadata.mode = "agent" injected
        max_tokens: Some(SUB_AGENT_MAX_TOKENS),
    }
}
```

### rig Framework Message Roles

Tool results in rig are modeled as `UserContent::ToolResult` inside `Message::User`:
```rust
pub enum UserContent {
    Text(UserText),
    ToolResult(ToolResult),   // ← Tool results are "user" messages
    Image(Image),
    Document(Document),
}
```

For OpenAI/Copilot, these are extracted to `role: "tool"` on the wire (correct), but the **request itself** still isn't marked as agent-initiated.

---

## Proposed Fix Strategy

### Approach: Inject `metadata.mode` at the Right Level

The classifier infrastructure is sound — we just need to set the field it's looking for.

**Key Insight from VSCode Copilot Chat source:**
```typescript
userInitiatedRequest: iterationNumber === 0 && !isContinuation && !isSubagent
```

Only the **first iteration** of a user message (not a continuation, not a subagent) should be `x-initiator: user`.

### Fix Points

1. **`CopilotProvider::complete_with_tools()`** — Accept an `is_agent` flag and inject `additional_params(json!({"metadata": {"mode": "agent"}}))`

2. **`deep_search_provider_config.rs`** — Set `additional_params: Some(json!({"metadata": {"mode": "agent"}}))`

3. **Agent loop iteration tracking** — Track whether this is the first iteration (user-initiated) vs subsequent iterations (tool-call follow-ups = agent-initiated)

4. **Session-level flag** — Mark subordinate sessions and scheduled jobs as agent-initiated at creation time

### What NOT To Fix

- The rig framework's `UserContent::ToolResult` modeling is correct per OpenAI's API design
- The wire format transformation (tool results → `role: "tool"`) is already correct
- The classifier/facade/middleware layers don't need changes — they just need the input they're designed for
