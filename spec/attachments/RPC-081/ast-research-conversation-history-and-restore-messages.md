# RPC-081 AST Research: Conversation History Round-Trip + session_restore_messages Parity

## Scope

Two sub-areas needed for full RPC-081 acceptance:

1. **Multi-turn agent-loop round-trip** — verify and pin that `session.inner.messages: Vec<rig::message::Message>` is read+appended every turn through `run_agent_stream_with_images`, so the LLM sees prior turns.
2. **`restore_session_messages` port** — replace the 5-line stub in `codelet/sessions/src/handle_impl.rs` with the canonical envelope-parsing replay logic from `codelet/napi/src/session_bindings.rs:2401-2567`, keeping the no_napi_dependency boundary intact.

---

## Area 1 — Agent loop history round-trip

### Current call sites in `codelet-agent-loop`

```
codelet/agent-loop/src/agent_loop.rs:288  let mut inner_session = session.inner.lock().await;       # hook path
codelet/agent-loop/src/agent_loop.rs:473  let mut inner_session = session.inner.lock().await;       # main turn lock
codelet/agent-loop/src/agent_loop.rs:868  "claude"  => run_with_provider!(&mut inner_session, ...);
codelet/agent-loop/src/agent_loop.rs:897  codelet_cli::interactive::run_agent_stream_with_images(   # openai inlined
                                              agent, input, bridge_images, &mut inner_session, ...);
codelet/agent-loop/src/agent_loop.rs:915..917  gemini/zai/codex via run_with_provider!
codelet/agent-loop/src/agent_loop.rs:944  github-copilot via run_with_provider!
codelet/agent-loop/src/agent_loop.rs:996  custom-provider arm: run_agent_stream_with_images(... &mut inner_session ...)
```

So the dispatch path **already** threads `&mut session.inner.messages` (via `&mut inner_session`) into the rig streaming engine. The actual append-after-clone happens in `codelet/cli/src/interactive/stream_loop.rs:461-471`:

```rust
let mut stream = agent
    .prompt_streaming_with_history_and_hook(effective_prompt, &mut session.messages, hook)
    .await;
session.messages.push(Message::User {
    content: build_user_content_with_images(effective_prompt, images),
});
```

`prompt_streaming_with_history_and_hook` is invoked 5× in `stream_loop.rs` (lines 462, 670, 1243, 1603, 1702) — all branches accept `&mut session.messages` as the chat-history slot. After each branch, the *new* user message is pushed onto `session.messages` so the next turn includes it.

Final assistant append happens inside `handle_final_response` (`stream_loop.rs:1131` and `:1796` per the gap-analysis attachment).

### Implication for RPC-081 scope

The plumbing is already in place. The work in RPC-081 is:

A. **Add behavioural test** that exercises a two-turn session through `SessionManager.send_input` and asserts the *recorded* history seen by the stub provider on turn 2 contains turn 1's `User` and `Assistant`. This requires extending `StubProvider` to capture incoming `rig::completion::Message` slices.

B. **Add source-shape regression** that the literal `"vec![Message { role: MessageRole::User"` (the broken stub from §2 of the gap analysis) is absent from `codelet/agent-loop/src/agent_loop.rs`.

### Stub provider extension

`codelet/providers/src/stub_provider.rs` exposes `StubProvider` implementing `LlmProvider`. Two relevant methods:

```
codelet/providers/src/stub_provider.rs:71  async fn complete(&self, _messages: &[codelet_common::Message]) -> Result<String, ProviderError>
codelet/providers/src/stub_provider.rs:75  async fn complete_with_tools(&self, messages: &[codelet_common::Message], _tools: &[ToolDefinition]) -> Result<CompletionResponse, ProviderError>
```

These take `codelet_common::Message` (NOT `rig::completion::Message`). For the agentic Rig path, history is threaded via `create_rig_agent` → `prompt_streaming_with_history_and_hook`. So the recording surface needs to be on the **Rig model** (`RhaiCustomProviderModel` for custom providers, or the provider-specific rig adapter for built-ins).

For RPC-081, the simplest approach: register the stub as a **custom provider** that goes through `CustomProvider::create_rig_agent` (already wired in the agent-loop body's `_ => { ... }` arm) and add a recording hook there. Alternatively, build an in-test rig provider trait impl that captures the slice.

**Decision**: extend `StubProvider` with a process-global `Mutex<Vec<Vec<rig::completion::Message>>>` recording slot, populated by a **rig model** adapter constructed at session-spawn time (parallel to `RhaiCustomProviderModel`). New helper: `codelet_providers::stub_provider::recorded_histories(slug: &str) -> Vec<Vec<rig::completion::Message>>` and `clear_recorded_histories(slug: &str)`.

---

## Area 2 — `restore_session_messages` port

### Canonical source

```
codelet/napi/src/session_bindings.rs:2401  pub async fn session_restore_messages(session_id: String, envelopes: Vec<String>) -> Result<()>
codelet/napi/src/session_bindings.rs:2574  pub async fn session_restore_token_state(session_id: String, state: TokenRestoreState) -> Result<()>
```

The body parses each envelope JSON's `message.role` and `message.content` blocks:
- **`role == "assistant"`**: walks content blocks for types `thinking` / `text` / `tool_use`, producing `StreamChunk::Thinking` / `StreamChunk::Text` / `StreamChunk::ToolCall`, plus a terminating `StreamChunk::Done`. Text blocks are accumulated and pushed onto `session.inner.messages` as `rig::message::Message::Assistant`.
- **`role != "assistant"` (user)**: walks content blocks for types `text` / `tool_result`. Text blocks become `StreamChunk::UserInput` (and accumulated as `rig::message::Message::User`); `tool_result` blocks become `StreamChunk::ToolResult { tool_call_id, content, is_error }` (NOT appended to inner.messages).
- **Skip-rule**: if the joined text contains BOTH `<system-reminder>` AND `<!-- type:`, the entire content block is silently skipped (no inner.messages push, no stream chunk).
- Final write: `session.inner.lock().await.messages.extend(...)`; then `session.handle_output(chunk)` for each `StreamChunk`.

### Current target stub

```
codelet/sessions/src/handle_impl.rs:274
    fn restore_session_messages(&self, session_id: &SessionId, _envelopes: Vec<String>) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(_session) => Ok(()),
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }
```

Stub returns `Ok(())` immediately and ignores `_envelopes`. The signature is **synchronous** (`-> Result<(), String>`), unlike the NAPI source which is `async fn ... -> Result<()>`. The trait method `SessionManagerHandle::restore_session_messages` is sync — so the port needs to either:
- Acquire `session.inner` via `tokio::runtime::Handle::current().block_on(...)`, or
- Refactor inner access to use `std::sync::Mutex` / `parking_lot::Mutex` instead of `tokio::sync::Mutex`, or
- Use `tokio::task::block_in_place` if a multi-thread runtime is available.

**Inspection needed**: confirm the lock type on `session.inner`. From `agent_loop.rs:288`, `session.inner.lock().await` shows `tokio::sync::Mutex`. So the port needs to either use `try_lock()` (no `.await`) or `block_on`. Since restoration happens at session attach time (not in a hot loop), `block_in_place` is acceptable but requires the multi-thread runtime.

**Decision**: keep restoration sync at the trait boundary. Use `tokio::task::block_in_place(|| Handle::current().block_on(...))` so the lock can be acquired cleanly. If a single-thread runtime is in use, fall back to `try_lock()` with a clear error.

### Boundary regression

The existing `assert_no_transitive_dependency!("codelet-sessions", "codelet-napi")` and `assert_no_import_in_sources!("sessions", "codelet_napi")` rules (analogous to those in `codelet/agent-loop/tests/rpc072_work_agent_roundtrip.rs`) must continue to hold. The NAPI source body uses `napi::Error::from_reason` and `Error::from_reason` — these need to be removed in the port (use plain `format!` strings since the trait returns `Result<(), String>`).

---

## Test file layout

| File | Purpose | Test count |
|------|---------|------------|
| `codelet/agent-loop/tests/rpc081_conversation_history.rs` | Multi-turn round-trip + source-shape regression | 3 |
| `codelet/sessions/tests/rpc081_restore_session_messages.rs` | Restoration handler unit tests | 7 |

Total: 10 tests mapping to the 10 scenarios in `agent-loop-conversation-history-session-inner-messages-round-trip-session-restore-messages-parity.feature`.

---

## Risks / Open questions

1. **Stub provider as custom provider**: registering the stub under `ProviderType::Custom("stub")` works (already done in RPC-066), but for the rig-agent path to record histories, the `CustomProvider::create_rig_agent` must dispatch to a recording model. Either:
   a. Extend the existing `RhaiCustomProviderModel` with a "recorder mode" flag, or
   b. Add a separate `StubRigModel` and wire it into the custom-provider dispatch when slug == "stub" (gated by `cfg(feature = "test-support")`).

   **Recommendation**: option (b). Keeps the production Rhai path untouched.

2. **Sync ↔ async mismatch in restoration**: confirmed that `session.inner` is `tokio::sync::Mutex`. The chosen `block_in_place` strategy assumes the test runtime is `tokio::test(flavor = "multi_thread")`. Single-thread test runtimes will panic — gate tests with `flavor = "multi_thread"`.

3. **System-reminder content-block heuristic**: NAPI source checks `s.contains("<system-reminder>") && s.contains("<!-- type:")` against either the joined text array OR a single string content. Both branches must be preserved in the port.
