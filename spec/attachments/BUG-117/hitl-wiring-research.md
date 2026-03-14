# BUG-117 Research: HITL request_user_input Handler Wiring

## Summary

TOOL-017 and BUG-116 implemented the HITL tool infrastructure and Codex facade, but the runtime handler was never registered in the session manager. The tool is registered with all 5 providers but always returns `"request_user_input is unavailable in the current session mode"` because no `HitlHandler` is set at session startup.

---

## 1. What Exists Today

### 1.1 Rust Tool Layer (`codelet/tools/src/request_user_input.rs`)

- `HitlHandler` type: `Arc<dyn Fn(Uuid, HitlRequest) -> Result<HitlResponse, String> + Send + Sync>`
- Global per-session registry: `HITL_HANDLERS: RwLock<HashMap<Uuid, HitlHandler>>`
- `set_hitl_handler(session_id, handler)` — register/unregister
- `has_hitl_handler(session_id)` — check if registered
- `execute_hitl(session_id, request)` — validates questions, dispatches to handler
- `clear_all_hitl_handlers()` — test cleanup
- `HitlRequest { questions: Vec<HitlQuestion> }`
- `HitlResponse::Answered { answers: HashMap<String, HitlAnswer> } | Cancelled`
- `HitlAnswer { selected: Vec<String>, other: Option<String> }`

### 1.2 Codex Facade (`codelet/tools/src/facade/codex.rs`, `wrapper.rs`)

- `CodexRequestUserInputFacade` implements `HitlToolFacade` trait
- `HitlToolFacadeWrapper` implements `rig::tool::Tool`, calls `execute_hitl()` directly
- Schema has `additionalProperties: false` (Codex convention)
- Cancellation → `ToolError::Execution` with message `"request_user_input was cancelled before receiving a response"`

### 1.3 Provider Registration

All 5 providers register the tool:

| Provider | File | Registration |
|----------|------|-------------|
| **Claude** | `codelet/providers/src/claude.rs:534` | `.tool(RequestUserInputTool::new(session_id))` |
| **OpenAI** | `codelet/providers/src/openai.rs:342` | `.tool(RequestUserInputTool::new(session_id))` |
| **Gemini** | `codelet/providers/src/gemini.rs:175` | `.tool(RequestUserInputTool::new(session_id))` |
| **Z.AI** | `codelet/providers/src/zai.rs:250` | `.tool(RequestUserInputTool::new(session_id))` |
| **Codex** | `codelet/providers/src/codex/mod.rs:389` | `.tool(HitlToolFacadeWrapper::new(Arc::new(CodexRequestUserInputFacade), session_id))` |

### 1.4 Tests

- 14 unit tests in `request_user_input.rs` — all pass
- 7 facade/wrapper tests in `wrapper.rs` — all pass
- 1 provider integration test in `codex/mod.rs` — passes
- All tests use `set_hitl_handler()` to register mock handlers

---

## 2. What's Missing

### 2.1 Session Manager Handler Registration

`codelet/napi/src/session_manager.rs` registers handlers for every other blocking tool but **NOT** for HITL:

| Handler | Registration Line | Cleanup Line |
|---------|-------------------|-------------|
| `set_pause_handler()` | **5328** | **5669** |
| `set_fspec_handler_for_session()` | **5376** | **5671** |
| `set_session_search_handler()` | **5384** | **5672** |
| `set_deep_search_handler()` | **5408** | **5674** |
| `set_inject_summary_handler()` | **5432** | **5673** |
| `set_hitl_handler()` | **❌ NEVER CALLED** | **❌ NEVER CLEANED UP** |

### 2.2 Blocking Channel Infrastructure

The session manager has blocking channel pairs for other tools:

- `pause_response_tx` / `pause_response_rx` — `std::sync::mpsc::channel::<PauseResponse>` (line 1032)
- `fspec_response_tx` / `fspec_response_rx` — `std::sync::mpsc::channel::<FspecResult>` (line 1035)

**Missing**: No `hitl_response_tx` / `hitl_response_rx` channel pair for HITL.

### 2.3 Wait/Send Methods

Existing patterns:

- `wait_for_pause_response()` (line 1387) — blocks on `pause_response_rx.recv()`
- `send_pause_response()` (line 1401) — sends via `pause_response_tx.send()`
- `wait_for_fspec_response()` (line 1423) — blocks on `fspec_response_rx.recv()`
- `send_fspec_result()` (line 1439) — sends via `fspec_response_tx.send()`

**Missing**: No `wait_for_hitl_response()` or `send_hitl_response()` methods.

### 2.4 StreamChunk Variant

The TypeScript/TUI communication uses `StreamChunk` variants:

- `FspecCommandRequest` / `FspecCommandResult` — for fspec tool calls
- Pause uses session status change to `Paused` + `PauseState` struct

**Missing**: No `HitlRequest` / `HitlResponse` StreamChunk variants for the TUI to display the question modal and send answers back.

### 2.5 NAPI Binding

Existing patterns:

- `session_pause_resume(session_id)` — NAPI function for TUI → Rust (line 6461)
- `session_pause_confirm(session_id, approved)` — NAPI function (line 6472)
- `session_send_fspec_result(session_id, result)` — NAPI function (line 6518)

**Missing**: No `session_send_hitl_response(session_id, response)` NAPI function.

### 2.6 TypeScript TUI Modal

**Missing**: No TUI component to render the HITL question modal and collect user answers.

---

## 3. VTCode Reference Implementation

### 3.1 Architecture Overview

VTCode uses a **front-end intercept** pattern rather than a handler registry:

1. **Tool declaration** (`vtcode-core/src/tools/request_user_input.rs`):
   - `RequestUserInputTool::execute()` always returns `Err("requires interactive UI session")`
   - The tool is never executed through normal tool dispatch
   
2. **Tool pipeline intercept** (`src/agent/runloop/unified/tool_pipeline/hitl.rs`):
   - `execute_hitl_tool()` checks if tool name matches `REQUEST_USER_INPUT`
   - If `request_user_input_enabled` is false, returns error
   - Otherwise calls `execute_request_user_input_tool()` directly

3. **TUI execution** (`src/agent/runloop/unified/request_user_input/modal.rs`):
   - `execute_request_user_input_tool()` normalizes args, resolves options, builds wizard steps
   - Calls `show_wizard_modal_and_wait()` — a TUI wizard modal
   - User selects options / types freeform text
   - Returns `Value` directly (answers or cancelled JSON)

4. **Feature gating** (`tool_catalog.rs`, `turn_loop.rs`):
   - `request_user_input_enabled` is derived from `FeatureSet::from_config()`
   - Only enabled in plan mode by default
   - Gating checked at both tool catalog (schema exposure) and execution time

### 3.2 Key Differences from Our Architecture

| Aspect | VTCode | Codelet (ours) |
|--------|--------|----------------|
| **Execution model** | Front-end intercepts tool call in tool pipeline | Handler registry with per-session `HitlHandler` |
| **Blocking mechanism** | `async` — TUI wizard modal is awaited directly | `sync` — handler blocks on `mpsc::channel::recv()` |
| **UI rendering** | Direct TUI widget (wizard modal with InlineListItem) | StreamChunk → TypeScript → React modal |
| **Answer transport** | Direct `Value` return from modal | Channel: `hitl_response_tx.send()` → `hitl_response_rx.recv()` |
| **Schema** | Same questions/options/id/header schema | Same schema — structurally identical |
| **Option suggestions** | Auto-generates options from `focus_area` + `analysis_hints` | Not implemented (options required or omitted) |
| **Cancellation** | `WizardModalOutcome::Cancelled` → `json!({"cancelled": true})` | `HitlResponse::Cancelled` → `ToolError::Execution` |

### 3.3 Key VTCode Files

- **Tool declaration**: `vtcode-core/src/tools/request_user_input.rs` (118 lines)
- **HITL gate/policy**: `vtcode-core/src/safety/hitl.rs` (326 lines) — tool allow/deny/approval policies
- **Tool pipeline intercept**: `src/agent/runloop/unified/tool_pipeline/hitl.rs` (38 lines)
- **Modal execution**: `src/agent/runloop/unified/request_user_input/modal.rs` (248 lines)
- **Schema/validation**: `src/agent/runloop/unified/request_user_input/schema.rs` (135 lines)
- **Option resolution**: `src/agent/runloop/unified/request_user_input/options.rs`
- **Auto-suggestions**: `src/agent/runloop/unified/request_user_input/suggestions.rs`

---

## 4. Implementation Plan

### 4.1 Rust Side (codelet/napi/src/session_manager.rs)

**Follow the exact fspec handler pattern (CODE-009):**

1. Add `hitl_response_tx` / `hitl_response_rx` channel pair to `BackgroundSession` struct (next to `fspec_response_tx/rx` at line 973)
2. Initialize in `BackgroundSession::new()` (next to fspec channel at line 1035)
3. Add `wait_for_hitl_response()` method (mirror `wait_for_fspec_response()` at line 1423)
4. Add `send_hitl_response()` method (mirror `send_fspec_result()` at line 1439)
5. Register handler in session run method (after fspec handler registration at line 5376):

```rust
// BUG-117: Register HITL handler for request_user_input
let session_for_hitl = session.clone();
let hitl_handler: codelet_tools::request_user_input::HitlHandler = 
    std::sync::Arc::new(move |_session_id, request| {
        // Check global callback is registered
        if GLOBAL_CHUNK_CALLBACK.get().is_none() {
            return Err("Global chunk callback not registered - cannot execute HITL".to_string());
        }
        
        // Emit HitlRequest chunk for TypeScript to process
        session_for_hitl.handle_output(StreamChunk::hitl_request(/* ... */));
        
        // Block until TypeScript sends response
        let response = session_for_hitl.wait_for_hitl_response();
        Ok(response)
    });
codelet_tools::set_hitl_handler(session.id, Some(hitl_handler));
```

6. Clean up in session cleanup (after line 5674):

```rust
codelet_tools::set_hitl_handler(session.id, None);
```

### 4.2 StreamChunk Variants (codelet/napi/src/types.rs)

Add two new variants (mirror `FspecCommandRequest` / `FspecCommandResult`):

```rust
/// BUG-117: HITL request - sent when LLM invokes request_user_input
HitlRequest {
    #[napi(js_name = "hitlRequest")]
    hitl_request: HitlRequestInfo,
},

/// BUG-117: HITL response - sent by TypeScript after user answers questions
HitlResponse {
    #[napi(js_name = "hitlResponse")]
    hitl_response: HitlResponseInfo,
},
```

With supporting types:

```rust
pub struct HitlRequestInfo {
    pub questions: Vec<HitlQuestionInfo>,
}

pub struct HitlQuestionInfo {
    pub id: String,
    pub header: String,
    pub question: String,
    pub options: Option<Vec<HitlOptionInfo>>,
}

pub struct HitlOptionInfo {
    pub label: String,
    pub description: String,
}

pub struct HitlResponseInfo {
    pub cancelled: bool,
    pub answers: Option<HashMap<String, HitlAnswerInfo>>,
}

pub struct HitlAnswerInfo {
    pub selected: Vec<String>,
    pub other: Option<String>,
}
```

### 4.3 NAPI Binding

Add `session_send_hitl_response()` function (mirror `session_send_fspec_result()` at line 6518):

```rust
#[napi]
pub fn session_send_hitl_response(session_id: String, response: HitlResponseInfo) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    // Convert NAPI type to tools HitlResponse
    let hitl_response = if response.cancelled {
        HitlResponse::Cancelled
    } else {
        // Convert answers...
        HitlResponse::Answered { answers }
    };
    session.send_hitl_response(hitl_response);
    Ok(())
}
```

### 4.4 TypeScript TUI (src/)

- Handle `HitlRequest` StreamChunk in `persistentChunkHandler`
- Render a modal with question cards (header, question text, options as selectable list, freeform text input)
- On submit: call `sessionSendHitlResponse(sessionId, { cancelled: false, answers: {...} })`
- On cancel/Esc: call `sessionSendHitlResponse(sessionId, { cancelled: true })`

---

## 5. Provider-Specific Considerations

### 5.1 Codex Provider

The Codex provider already uses `HitlToolFacadeWrapper` with `CodexRequestUserInputFacade` (BUG-116). This wrapper calls `execute_hitl()` which will dispatch to the newly registered handler. **No changes needed** — once the handler is registered in session manager, it will work.

### 5.2 Claude/OpenAI/Gemini/Z.AI Providers

These providers use `RequestUserInputTool::new(session_id)` directly. The `RequestUserInputTool::call()` method also calls `execute_hitl()` (line 345 in `request_user_input.rs`). **No changes needed** — same handler registry, same dispatch path.

### 5.3 Verification

All providers converge on `execute_hitl(session_id, request)`:
- Codex: `HitlToolFacadeWrapper::call()` → `execute_hitl()`
- Others: `RequestUserInputTool::call()` → `execute_hitl()`

Both paths use the same per-session `HITL_HANDLERS` registry. **One handler registration in session manager covers all providers.**

---

## 6. Risks and Edge Cases

1. **Multiple sessions**: The handler is per-session (keyed by `Uuid`), so concurrent sessions are safe. This follows the same pattern as `set_fspec_handler_for_session()`.

2. **Headless/CLI mode**: If no GLOBAL_CHUNK_CALLBACK is registered, the handler should return `Err(...)` immediately rather than blocking forever (mirror the fspec handler guard at line 5337).

3. **Session cleanup**: Must call `set_hitl_handler(session.id, None)` in the cleanup block to prevent handler leaks.

4. **Channel exhaustion**: Using `std::sync::mpsc::channel` (unbounded) — same as pause and fspec channels. Single send/recv per tool call, so no backpressure concern.

5. **Cancellation during compaction**: The `request_user_input` tool should not be invoked during compaction (LLM generates summaries, not tool calls), but the handler should handle unexpected calls gracefully.
