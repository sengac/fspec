# RPC-072 Parity Gap: Original NAPI Agent Loop vs. RPC-072 "Minimum-Viable" Stub

**Status (2026-05-28):** RPC-072 was prematurely marked `done`. The deliverable at
`codelet/agent-loop/src/agent_loop.rs` (203 lines) is a stub that explicitly
omits **everything that makes the agent agentic**. This document is the
line-by-line gap against the canonical NAPI implementation at
`codelet/napi/src/agent_loop.rs` (**1,769 lines**) and its supporting
streaming engine `codelet/cli/src/interactive/stream_loop.rs`.

The user-visible symptom that surfaced this gap was a 429 from Anthropic
rendered as a raw JSON dump in an `ErrorDialog` — but the 429 is
incidental. The actual defect class is "the Rust agent loop is not the
NAPI agent loop." Every rule, example, and architecture note in this
attachment is derived from comparing the two implementations.

---

## TL;DR — What RPC-072 Shipped vs. What It Should Have Shipped

| Concern                             | NAPI canonical                                              | RPC-072 deliverable                          |
| ----------------------------------- | ----------------------------------------------------------- | -------------------------------------------- |
| LOC                                 | 1,769                                                       | 203                                          |
| Provider dispatch                   | `run_with_provider!` macro + 7 arms + custom fallthrough    | Flat `match`, 5 arms, no streaming           |
| Conversation history                | Persistent `session.inner.messages: Vec<rig::Message>`      | `vec![only the new user text]` per turn      |
| System-prompt / role injection      | `session.get_role()` → `create_rig_agent(preamble)`         | Not invoked                                  |
| Tools                               | ~21 tools via `create_rig_agent` + MCP wrappers             | `&[]` (empty slice)                          |
| Streaming                           | rig multi-turn stream via `run_agent_stream_with_images`    | Single `complete_with_tools` round-trip     |
| Tool execution loop                 | `RigAgent::with_default_depth(agent)` + inner agentic loop  | Not present                                  |
| Thinking config                     | Per-provider via `thinking_config_value` + facade           | None                                         |
| Token tracking                      | `BackgroundOutput` → `session.update_tokens`                | `update_tokens` never called                 |
| Error classification / retry        | NET-001 + recovery_compaction/network/thinking/truncation   | Single `?` propagation                       |
| Interrupt cascade                   | `is_interrupted: AtomicBool` + `interrupt_notify: Notify`   | Not implemented                              |
| MCP injection drain                 | `tokio::select!` arm + `mcp_channel_open` flag              | Channel held open, never drained             |
| Lifecycle hooks                     | session_start / user_prompt_submit / post_tool_use / session_end | Not invoked                            |
| Persistence (REFAC-007)             | `persist_user_message` / `_assistant_message` / `_tool_result` / `_token_state` | Not invoked       |
| `BackgroundOutput` chunk variants   | 19+ (Text, Thinking, ToolCall, ToolResult, ToolProgress, TokenUpdate, ContextFillUpdate, UserNotification, Interrupted, SessionStateChange, CompactionComplete, FspecCommandRequest, IncomingMessage, UserInput, Done, Error, …) | 2 (`Text` or `Error`, then `Done`)           |

---

## 1. Provider Dispatch — `run_with_provider!` Macro

### Canonical: `codelet/napi/src/agent_loop.rs:68–140`, call site `:1097–1251`

```rust
macro_rules! run_with_provider {
    ($inner:expr, $getter:ident, $input:expr, $images:expr, $session:expr, $output:expr, $thinking:expr) => {
        match $inner.provider_manager_mut().$getter() {
            Ok(provider) => {
                let mcp_wrappers = codelet_tools::gather_mcp_tool_wrappers($session.id);
                let role_preamble = $session.get_role();
                let agent = provider.create_rig_agent($session.id, role_preamble.as_deref(), $thinking.clone());
                if !mcp_wrappers.is_empty() {
                    for wrapper in mcp_wrappers {
                        agent.tool_server_handle.add_tool(wrapper).await?;
                    }
                }
                codelet_tools::set_mcp_tool_server_handle($session.id, agent.tool_server_handle.clone());
                let agent = codelet_core::RigAgent::with_default_depth(agent);
                codelet_cli::interactive::run_agent_stream_with_images(
                    agent, $input, $images, $inner,
                    $session.is_interrupted.clone(),
                    $session.compaction_in_progress.clone(),
                    $session.interrupt_notify.clone(),
                    $output,
                ).await
            }
            Err(e) => Err(anyhow::anyhow!("Failed to get provider: {}", e)),
        }
    };
}
```

Provider arms at `:1097-1251`: `claude`, `openai` (inlined because `get_openai` takes `session.id`), `gemini`, `zai`, `codex`, `github-copilot | copilot`, and `_ =>` (custom-provider via `CustomProvider::create_rig_agent`).

Structural predicate at `:155-160`:
```rust
pub(crate) fn agent_loop_dispatch_supports_provider(provider_name: &str) -> bool {
    matches!(provider_name, "claude" | "openai" | "gemini" | "zai" | "codex" | "github-copilot" | "copilot")
}
```

### Stub: `codelet/agent-loop/src/agent_loop.rs:74-172`

```rust
async fn run_one_turn(session: &BackgroundSession, input: &str) -> Result<String, AgentLoopError> {
    let messages = vec![Message { role: MessageRole::User, content: MessageContent::Text(input.to_string()) }];
    let mut inner = session.inner.lock().await;
    let provider_type = inner.provider_manager().current_provider_type().clone();
    let response = match provider_type {
        ProviderType::Custom(slug) => { /* stub-registry only */ }
        ProviderType::Claude => inner.provider_manager_mut().get_claude()?.complete_with_tools(&messages, &[]).await?,
        ProviderType::OpenAI => inner.provider_manager_mut().get_openai(session.id)?.complete_with_tools(&messages, &[]).await?,
        ProviderType::Gemini => inner.provider_manager_mut().get_gemini()?.complete_with_tools(&messages, &[]).await?,
        ProviderType::ZAI    => inner.provider_manager_mut().get_zai()?.complete_with_tools(&messages, &[]).await?,
        other => return Err(AgentLoopError::ProviderUnavailable(format!(
            "RPC-072 minimum-viable agent_loop does not yet dispatch provider {other:?}; \
             the NAPI-side `run_with_provider!` macro covers this case"
        ))),
    };
```

### Gaps

- ❌ No Codex, no `github-copilot`/`copilot`, no Custom (non-stub) provider arms.
- ❌ No `create_rig_agent` call → **no preamble, no tools, no thinking config**.
- ❌ No `run_agent_stream_with_images` → **no streaming, no multi-turn, no images**.
- ❌ No MCP wrapper gather, no `set_mcp_tool_server_handle`.
- ❌ Calls `complete_with_tools(&messages, &[])` directly — **zero tools, zero history, non-streaming**.

---

## 2. Conversation History (REFAC-007 / CLI-008 / BRIDGE-007)

### Canonical storage

`codelet/cli/src/session/mod.rs:30-62`:
```rust
pub struct Session {
    provider_manager: ProviderManager,
    /// Message history — single source of truth for conversation context
    /// Persists across REPL iterations, cleared on provider switch (CLI-008)
    pub messages: Vec<rig::message::Message>,
    pub turns: Vec<ConversationTurn>,
    pub token_tracker: TokenTracker,
    pub annotations: HashMap<usize, Vec<StructuralAnnotation>>,
    pub thinking_exhaustion_cross_turn_count: u32,
    pub session_thinking_level: codelet_tools::facade::ThinkingLevel,
}
```

Accessed through `BackgroundSession.inner: Arc<Mutex<codelet_cli::session::Session>>` (`codelet/sessions/src/background_session.rs:294-295`).

### Canonical data flow (per turn)

1. **Input drained** (`agent_loop.rs:361-383`) via `tokio::select!` from `input_rx`.
2. **Persist user message BEFORE LLM call** (`agent_loop.rs:529-534`):
   ```rust
   if let Err(e) = persist_user_message(&session.id, input) { tracing::error!(...); }
   ```
   `persist_user_message` (`codelet/napi/src/persist.rs:19-57`) writes a `MessageEnvelope { message_type:"user", payload: MessagePayload::User(UserMessage{ role:"user", content: vec![UserContent::Text] }) }` via `append_message_with_metadata`.
3. **Build chat history for LLM** — done inside `run_agent_stream_internal` (`codelet/cli/src/interactive/stream_loop.rs:461-471`):
   ```rust
   let mut stream = agent
       .prompt_streaming_with_history_and_hook(effective_prompt, &mut session.messages, hook)
       .await;
   // Add user message to history AFTER rig clones it
   session.messages.push(Message::User { content: build_user_content_with_images(effective_prompt, images) });
   ```
4. **Tool result interleaving** (`agent_loop.rs:1558-1572`): when a `ToolResult` arrives, the assistant message accumulated so far is flushed first → on-disk order is `user → assistant(text+tool_use) → tool_result → assistant(final)`.
5. **Final assistant message** flushed on `StreamEvent::Done(stop_reason)` (`agent_loop.rs:1666-1690`) plus `persist_token_state`.
6. **History append by rig itself** in `handle_final_response` (`stream_loop.rs:1131,1796`).
7. **Restore on attach** — `session_bindings.rs:2401 pub async fn session_restore_messages(...)` reads on-disk envelopes back into both `inner.messages` and the output buffer (synthetic `StreamChunk::UserInput` / `Text` / `ToolCall` / `ToolResult` replay).

### Stub gap

`agent_loop.rs:78-81` rebuilds the prompt from scratch every turn:
```rust
let messages = vec![Message { role: MessageRole::User, content: MessageContent::Text(input.to_string()) }];
```

The persistent `session.inner.lock().await.messages` is **never read or written**. None of `persist_user_message`, `persist_assistant_message_internal`, `persist_tool_result_internal`, `persist_token_state` are invoked. **The LLM sees no chat history. Resumed sessions have no history.**

---


## 3. System Prompt / Role Injection (BUG-120)

### Canonical

Storage on `BackgroundSession` (`codelet/sessions/src/background_session.rs:343-344`):
```rust
/// Session role - simple string overlay for system prompt
role: RwLock<Option<String>>,
```

Getter at `:836-841`, setter at `:829-834`.

Injection at `agent_loop.rs:91-96`:
```rust
// BUG-120: Read session role and pass as preamble so it becomes part of the system prompt.
let role_preamble = $session.get_role();
let agent = provider.create_rig_agent($session.id, role_preamble.as_deref(), $thinking.clone());
```

Per-provider absorption (Claude example, `codelet/providers/src/claude.rs:507-595`):
```rust
let facade = select_claude_facade(is_oauth);
let effective_preamble = facade.transform_preamble(preamble_text);
agent_builder = agent_builder.preamble(&effective_preamble);
let cached_system = facade.format_for_api(preamble_text);
let additional = json!({ "system": cached_system });
agent_builder = agent_builder.additional_params(additional);
```

### Stub gap

Neither `session.get_role()` nor any preamble is read or sent. The `/role <text>` slash command is parsed by the TUI dispatch path but never reaches the LLM. Roles are dead config in the fspec binary.

---

## 4. Tools

### Canonical tool registry

Each provider's `create_rig_agent` builds the agent's tool list. Example
`codelet/providers/src/claude.rs:507-595` registers Read/Write/Edit/Bash/
Grep/Glob/MultiEdit/NotebookRead/NotebookEdit/WebFetch/WebSearch/Task/
TodoWrite/Plan/AskFollowupQuestion/AttemptCompletion etc. plus MCP-injected
tools per `gather_mcp_tool_wrappers(session.id)` at `agent_loop.rs:79-89`.

`codelet_core::RigAgent::with_default_depth(agent)` (used at every call
site) wraps the rig agent in the tool-use loop. Default depth caps the
inner agentic loop iterations.

### Stub gap

`complete_with_tools(&messages, &[])` is called with an empty tool slice.
There is no tool registry, no MCP gather, no tool-server-handle
registration. The agent cannot Read/Write/Edit/Bash. It is not an agent;
it is a chat completion endpoint.

---

## 5. Streaming + Tool Execution Loop

### Canonical streaming engine

`codelet/cli/src/interactive/stream_loop.rs` — `run_agent_stream_with_images`
(multi-turn, image-aware) calls into `run_agent_stream_internal`.

Key responsibilities:
- Opens `agent.prompt_streaming_with_history_and_hook(...)` returning a rig event stream.
- `tokio::select!`'s on (a) next stream event, (b) `interrupt_notify.notified()`, (c) compaction-in-progress flag.
- Translates each rig `StreamEvent` into a `StreamChunk` via the `BackgroundOutput` sink (`agent_loop.rs:1463-1690`):
  - `StreamEvent::Text(delta)` -> accumulates in `assistant_content` + emits `StreamChunk::Text`.
  - `StreamEvent::Thinking(delta)` -> emits `StreamChunk::Thinking`.
  - `StreamEvent::ToolCall { id, name, input }` -> emits `StreamChunk::ToolCall`.
  - `StreamEvent::ToolResult { ... }` -> flushes pending assistant message, persists tool result, emits `StreamChunk::ToolResult`.
  - `StreamEvent::ToolProgress { ... }` -> emits `StreamChunk::ToolProgress` (TOOL-011).
  - `StreamEvent::Tokens { input, output, cache_read, cache_creation }` -> `session.update_tokens(...)` + `StreamChunk::TokenUpdate` + `StreamChunk::ContextFillUpdate`.
  - `StreamEvent::Done(stop_reason)` -> flush, persist, `StreamChunk::Done`.
  - `StreamEvent::Notification` -> `StreamChunk::UserNotification` (NET-001 reconnection lines).
  - `StreamEvent::Interrupted` -> `StreamChunk::Interrupted`.
- Inner tool-execution loop (via `RigAgent::with_default_depth`): when the model returns a `ToolUse`, rig executes the tool, feeds back the `ToolResult`, and continues until `StopReason::EndTurn` or depth limit.

### Stub gap

Stub emits exactly 2 chunks per turn:
```rust
session.handle_output(StreamChunk::text(reply_text));    // or Error
session.handle_output(StreamChunk::done());
session.set_status(SessionStatus::Idle);
```

No streaming, no tool execution, no inner agent loop, no thinking, no
tool progress, no token usage, no notification, no interrupt. AgentView's
RPC-045 dispatch path for those variants is dead code in the fspec binary.

---

## 6. Thinking Config (BRIDGE-006 / PROV-005 / PROV-041)

### Canonical

Per-turn computation in `agent_loop.rs` (around `:495-520`):
```rust
let thinking_config_value = compute_effective_thinking_config(
    &session.inner.lock().await.session_thinking_level,
    detected_level_for_message,
    prompt_input.thinking_config.as_ref(),
);
```

Passed into `run_with_provider!(... thinking_config_value)` -> `provider.create_rig_agent(session.id, role, thinking)` where each provider merges it into `additional_params` (Claude/OpenAI/Gemini) or into the streaming request body.

The `/thinking` slash command (RPC-048) updates `session.inner.session_thinking_level`. Per-prompt overrides flow through `PromptInput.thinking_config`.

### Stub gap

`PromptInput.thinking_config` is ignored in `run_one_turn`. `session_thinking_level` is not read. `/thinking high` has no effect on the wire request.

---

## 7. Token Tracking

### Canonical

`BackgroundOutput::emit` (`agent_loop.rs:1620-1660` approx) on `StreamEvent::Tokens`:
```rust
session.update_tokens(input_tokens, output_tokens);
session.update_reasoning_tokens(reasoning_tokens);
self.emit_chunk(StreamChunk::TokenUpdate { tokens: self.session.token_tracker_snapshot() });
self.emit_chunk(StreamChunk::ContextFillUpdate { context_fill: ContextFill { fill_percentage } });
```

Drives the TUI header widget showing tokens up/down and context fill percentage.

### Stub gap

`update_tokens` is never called. `TokenUpdate` and `ContextFillUpdate`
chunks are never emitted. The tokens widget is permanently stuck at zero,
exactly what the screenshot shows.

---

## 8. Error Classification + Retry (NET-001, CMPCT-027, PROV-038/039)

### Canonical recovery helpers

Each wrapped around the rig stream:
- `recovery_network.rs` -- SSE disconnect / 429 backoff / circuit-breaker. Bounded by `MAX_NETWORK_RETRIES`.
- `recovery_compaction.rs` -- context-overflow -> auto-compact -> retry. Bounded by `MAX_COMPACTION_RETRIES`.
- `recovery_thinking.rs` -- thinking exhaustion fallback. Bounded by `MAX_THINKING_EXHAUSTION_RETRIES`.
- `recovery_truncation.rs` -- output truncation continuation. Bounded by `MAX_TRUNCATION_RETRIES`.
- `recovery_stall.rs` -- stream stall watchdog.
- `recovery_image.rs` -- image-too-large retry without image.

Each helper emits structured `StreamChunk::UserNotification` lines (e.g.
"Reconnecting..." / "Reconnected") that the TS frontend replaces in-place
rather than appending (NET-001 contract).

Errors are classified via `error_classifiers.rs` into `(retryable, kind)`
before deciding whether to retry or surface `StreamChunk::Error`.

### Stub gap

`agent_loop.rs:44-57`:
```rust
match run_one_turn(session.as_ref(), &prompt.input).await {
    Ok(reply_text)  => session.handle_output(StreamChunk::text(reply_text)),
    Err(err)        => session.handle_output(StreamChunk::error(err.to_string())),
}
```

A 429 becomes a permanent fatal error in the dialog. No retry, no backoff,
no classification. The raw provider error string (including the JSON body)
is emitted into both scrollback and the ErrorDialog. This is the exact
regression in the screenshot.

---

## 9. Interrupt Cascade (Esc Key)

### Canonical

Fields on `BackgroundSession` (`background_session.rs`):
- `is_interrupted: Arc<AtomicBool>`
- `interrupt_notify: Arc<Notify>`
- `compaction_in_progress: Arc<AtomicBool>`

`run_agent_stream_with_images` selects against `interrupt_notify.notified()`
and short-circuits the stream on Esc, emits `StreamChunk::Interrupted`,
flushes partial assistant content, returns control.

`AgentView` Esc handler (RPC-051) calls `backend.interrupt_session(id)`
which fires `interrupt_notify.notify_waiters()` and flips `is_interrupted`.

### Stub gap

`agent_loop.rs:38-70` does not consult `is_interrupted` or `interrupt_notify`.
Once `complete_with_tools` is awaiting, Esc does nothing. The provider call
must run to completion or error.

---

## 10. MCP Injection Drain (MCP-001)

### Canonical (`agent_loop.rs:323-460` approx)

```rust
let mut mcp_channel_open = true;
loop {
    let input_with_images = tokio::select! {
        result = input_rx.recv() => match result {
            Some(prompt_input) => Some(InputWithImages { ... }),
            None => { break; }                              // HOOK-013 session_end
        },
        injection_result = mcp_injection_rx.recv(), if mcp_channel_open => {
            match injection_result {
                Some(injection) => { /* synthesize prompt from MCP server message */ }
                None => { mcp_channel_open = false; continue; }
            }
        }
    };
    // process input
}
```

### Stub gap

`agent_loop.rs:41` parameter `_mcp_injection_rx` is held open but never
drained. MCP server-initiated messages (notifications + sampling) are
silently dropped.

---

## 11. Lifecycle Hooks

### Canonical

`agent_loop.rs` invokes through `codelet_core::lifecycle_hooks::*`:
- `session_start` -- once on first input.
- `user_prompt_submit` -- pre-LLM, per turn. Can mutate the prompt or block.
- `post_tool_use` -- after each tool invocation.
- `session_end` -- when `input_rx` closes (HOOK-013).

### Stub gap

None of these hooks are invoked. The stub's `input_rx.recv() == None`
branch (`agent_loop.rs:66-69`) only logs a trace; `session_end` does not fire.

---

## 12. Persistence (REFAC-007)

### Canonical persistence calls (per turn)

| When                                | What                                         | Where                                                          |
| ----------------------------------- | -------------------------------------------- | -------------------------------------------------------------- |
| Before LLM call                     | User message envelope                        | `persist_user_message` (`codelet/napi/src/persist.rs:19-57`)   |
| On `StreamEvent::ToolResult`        | Flush partial assistant, then tool result    | `agent_loop.rs:1558-1572`                                      |
| On `StreamEvent::Done(stop_reason)` | Final assistant + stop_reason                | `agent_loop.rs:1666-1690`                                      |
| On `StreamEvent::Done`              | Token state snapshot                         | `persist_token_state(&session.id, input, output)`              |
| On compaction complete              | Compaction marker                            | `persist_compaction_marker`                                    |

### Stub gap

Zero persistence calls. Restarting the fspec binary loses every
conversation. `/resume` (RPC-049) finds nothing to restore.

---

## 13. `BackgroundOutput` Chunk Variants Emitted

### Canonical (`agent_loop.rs:1463-1700`, plus broadcast points elsewhere)

`StreamChunk::Text`, `Thinking`, `ToolCall`, `ToolResult`, `ToolProgress`,
`SessionStateChange`, `UserNotification`, `Interrupted`, `TokenUpdate`,
`ContextFillUpdate`, `Done`, `Error`, `UserInput`, `IncomingMessage`,
`SupervisorPendingInjection`, `CompactionComplete`, `FspecCommandRequest`,
`FspecCommandResult`, `WorkUnitsUpdate`, `IsolationStateChange`,
`FooterStateUpdate`, `DebugStateChange`.

### Stub emits

`Text` (or `Error`), then `Done`. That's it. 17+ variants of dead code
in the AgentView dispatch path.

---

## 14. AgentView (TS Ink) Chunk Handling -- for parity reference

Canonical handler `src/tui/components/AgentView.tsx:3134-3527` includes
specialized rendering for every variant above (streaming concat, thinking
block manager, pending-diff store for Edit/Write tool calls, NET-001
reconnection replace, supervisor envelope parse, dual error surface
into scrollback + ErrorDialog).

Rust port `codelet/fspec-tui/src/store/agent_view/session_context.rs:155-219`
returns `None` for ToolCall / ToolResult / ToolProgress / Done. Tool
output is invisible in the Rust TUI even when chunks arrive. RPC-078
addressed prefix/duplication but didn't restore tool rendering.


## 15. Implementation Roadmap

Aim: port `codelet/napi/src/agent_loop.rs` to a NAPI-free crate, line by line, with no functional cut-downs.

### Phase A -- Foundation (extend, do not replace)

1. Lift `BackgroundOutput` + `BackgroundProgressEmitter` from `codelet/napi/src/agent_loop.rs:1310-1700` into a new NAPI-free module under `codelet/agent-loop/src/background_output.rs`. Preserve `StreamOutput` trait impl so `codelet_cli::interactive::run_agent_stream_with_images` continues to drive it.
2. Lift `InputWithImages`, `run_with_provider!` macro, `agent_loop_dispatch_supports_provider` into `codelet/agent-loop/src/dispatch.rs`. Same provider arms verbatim.
3. Lift `persist_user_message` / `persist_assistant_message_internal` / `persist_tool_result_internal` / `persist_token_state` / `persist_compaction_marker` from `codelet/napi/src/persist.rs` into `codelet/core/src/persistence/agent_loop_persist.rs` (already NAPI-free Rust calling `append_message_with_metadata`).

### Phase B -- Rewrite the loop body

Replace `agent_loop.rs:38-70` with the canonical body (paraphrased):

```rust
pub async fn agent_loop(
    session: Arc<BackgroundSession>,
    mut input_rx: mpsc::Receiver<PromptInput>,
    mut mcp_injection_rx: mpsc::Receiver<McpInjection>,
) {
    let mut mcp_channel_open = true;
    let output = BackgroundOutput::new(session.clone());

    invoke_lifecycle_hook(&session, LifecycleHook::SessionStart).await;

    loop {
        let input_with_images = tokio::select! {
            res = input_rx.recv() => match res {
                Some(prompt) => InputWithImages::from_prompt(prompt),
                None => break,
            },
            res = mcp_injection_rx.recv(), if mcp_channel_open => match res {
                Some(inj) => InputWithImages::from_mcp(inj),
                None => { mcp_channel_open = false; continue; }
            }
        };

        invoke_lifecycle_hook(&session, LifecycleHook::UserPromptSubmit { input: &input_with_images }).await;

        persist_user_message(&session.id, &input_with_images.text)?;

        let thinking = compute_effective_thinking_config(&session, &input_with_images).await;

        let mut inner = session.inner.lock().await;
        let provider_name = inner.provider_manager().current_provider_name();
        let result = match provider_name.as_str() {
            "claude"  => run_with_provider!(&mut inner, get_claude,  ...),
            "openai"  => /* inlined arm because get_openai(session.id) */,
            "gemini"  => run_with_provider!(&mut inner, get_gemini,  ...),
            "zai"     => run_with_provider!(&mut inner, get_zai,     ...),
            "codex"   => run_with_provider!(&mut inner, get_codex,   ...),
            "github-copilot" | "copilot" => run_with_provider!(&mut inner, get_github_copilot, ...),
            _ => /* custom-provider via CustomProvider::create_rig_agent */,
        };

        match result {
            Ok(()) => {},
            Err(e) => output.emit_error(classify_provider_error(e)).await,
        }

        session.set_status(SessionStatus::Idle);
    }

    invoke_lifecycle_hook(&session, LifecycleHook::SessionEnd).await;
}
```

### Phase C -- Wire everything else

- `FspecAgentHooks::spawn_agent_loop` already correct (`codelet/agent-loop/src/hooks.rs:37-62`).
- `codelet-fspec::common::build_service` already installs `FspecAgentHooks`.
- Crate dependencies: add `codelet-cli` (for `interactive::run_agent_stream_with_images`), `codelet-core` (for `RigAgent`, `lifecycle_hooks`, persistence), `codelet-tools` (for MCP gather), to `codelet/agent-loop/Cargo.toml`.
- Verify `no_napi_dependency` boundary test still passes after the lifts.

### Phase D -- Tests (one per gap)

- History test: send "remember 42"; next turn ask "what number"; expect "42".
- Role test: set role "ROBOT"; send "hi"; expect transcript shows preamble injected (verified via stub provider echoing preamble).
- Tools test: send "read file /tmp/x"; expect Read tool emits ToolCall + ToolResult + final assistant.
- Streaming test: count `StreamChunk::Text` deltas > 1 per assistant reply for a streaming-capable stub.
- Thinking test: `/thinking high` then prompt; expect `thinking_config` reaches the provider's `additional_params`.
- Token test: after one turn, `tokens_input > 0`, `context_fill > 0`.
- Retry test: inject 429 from stub; expect at least one `UserNotification: Reconnecting...` then either success or classified error.
- Interrupt test: start a long stream; fire interrupt; expect `StreamChunk::Interrupted` and stream halt.
- MCP injection test: send an `McpInjection` through `mcp_injection_rx`; expect it processed as a turn.
- Persistence test: turn -> restart binary -> `/resume` -> expect prior turn replayed via `session_restore_messages`.

### Phase E -- Acceptance

A `cargo test -p codelet-agent-loop --test parity` suite must pass each
of the gap tests above. Then a `tui-test` end-to-end against the real
Anthropic provider (or a real-looking stub) must show streaming tokens,
multi-turn history, and tool invocations, matching the TS Ink frontend's
behaviour against the NAPI loop.

---

## 16. Card Breakdown (Estimate-Aware)

The unified RPC-072 deliverable is 13+ points. Proposed split:

| Card     | Title                                                                              | Points |
| -------- | ---------------------------------------------------------------------------------- | ------ |
| RPC-072 (refit) | Lift BackgroundOutput + run_with_provider! macro + dispatch into NAPI-free codelet-agent-loop; replace stub loop body; smoke-test parity | 8 |
| RPC-080  | Persistence: REFAC-007 user/assistant/tool_result/token_state writes in NAPI-free loop | 5  |
| RPC-081  | Conversation history: session.inner.messages round-trip; session_restore_messages parity | 5 |
| RPC-082  | Role injection (BUG-120) + /role end-to-end through fspec binary                    | 3   |
| RPC-083  | Tool registry + RigAgent::with_default_depth + MCP wrapper gather                   | 8   |
| RPC-084  | Streaming: run_agent_stream_with_images + all 17 StreamChunk variants               | 13  |
| RPC-085  | Thinking config: BRIDGE-006/PROV-005/PROV-041 wiring through PromptInput.thinking_config | 5 |
| RPC-086  | Token tracking: update_tokens + TokenUpdate + ContextFillUpdate chunks              | 3   |
| RPC-087  | Error classification + retry recovery (NET-001 etc.) ports                          | 8   |
| RPC-088  | Interrupt cascade: is_interrupted + interrupt_notify + Esc handler                  | 3   |
| RPC-089  | MCP injection drain                                                                 | 5   |
| RPC-090  | Lifecycle hooks: session_start / user_prompt_submit / post_tool_use / session_end   | 5   |
| RPC-091  | AgentView Rust port: tool-call / tool-result / tool-progress rendering parity       | 8   |

Total: 79 points across 13 cards. None of this work was done. RPC-072 was prematurely marked done.

