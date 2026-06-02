# RPC-080 — AST Research: Persistence Call Sites in codelet-agent-loop

## Persistence helper exports (codelet/agent-loop/src/persist.rs)

| Line | Helper                              | Signature (return)                            |
| ---- | ----------------------------------- | --------------------------------------------- |
| 26   | `pub fn persist_user_message`       | `Result<(), String>`                          |
| 66   | `pub fn persist_assistant_message_internal` | `Result<(), String>`                  |
| 132  | `pub fn persist_tool_result_internal` | `Result<(), String>`                        |
| 190  | `pub fn persist_token_state`        | `Result<(), String>`                          |

All four helpers are `pub` (RPC-072 Phase A widened them from `pub(crate)`),
so the agent loop body and BackgroundOutput in the same crate can call
them as `crate::persist::*`. No NAPI dependency.

## Call sites in codelet/agent-loop/src/agent_loop.rs

| Line | Call                                              | Context                              |
| ---- | ------------------------------------------------- | ------------------------------------ |
| 301  | `persist_user_message(&session.id, input)`        | BEFORE the LLM stream dispatch       |

That single call mirrors NAPI agent_loop.rs:529 — same predicate
position, same arguments.

## Call sites in codelet/agent-loop/src/background_output.rs

| Line | Call                                                                                           | Stream event   |
| ---- | ---------------------------------------------------------------------------------------------- | -------------- |
| 143  | `self.persist_assistant_message()`                                                             | ToolResult     |
| 146  | `persist_tool_result_internal(&self.session.id, &tr.id, &tr.content, tr.is_error)`             | ToolResult     |
| 252  | `self.persist_assistant_message()`                                                             | Error          |
| 257  | `self.persist_assistant_message()`                                                             | Interrupted    |
| 262  | `self.persist_assistant_message_with_stop_reason(stop_reason)` (via `persist_assistant_message_with_stop_reason`) | Done           |
| 266  | `persist_token_state(&self.session.id, input_tokens, output_tokens)`                            | Done           |

The Done arm runs (1) `persist_assistant_message_with_stop_reason(stop_reason)`
then (2) `persist_token_state(...)`. The ToolResult arm runs (1)
`self.persist_assistant_message()` then (2) `persist_tool_result_internal(...)`.

## Order invariants (vs canonical NAPI source map)

| Canonical NAPI line | Rust port site                                              | Status |
| ------------------- | ----------------------------------------------------------- | ------ |
| 529 (user)          | agent-loop/src/agent_loop.rs:301                            | ✓      |
| 1436 (assistant flush before ToolResult) | agent-loop/src/background_output.rs:143       | ✓      |
| 1439-1446 (tool result) | agent-loop/src/background_output.rs:146                  | ✓      |
| 1532 (Error flush)  | agent-loop/src/background_output.rs:252                     | ✓      |
| 1537 (Interrupted flush) | agent-loop/src/background_output.rs:257                | ✓      |
| 1542 (Done stop_reason) | agent-loop/src/background_output.rs:262                  | ✓      |
| 1545-1548 (token state) | agent-loop/src/background_output.rs:266                  | ✓      |

All seven canonical sites are wired up in the Rust port. The
implementation lifted in RPC-072 Phase A is structurally correct; RPC-080
proves the wiring with explicit scenarios + tests and pins the call-site
order with source-shape regression guards.

## Helpers' on-disk envelope shape (from persist.rs source)

### persist_user_message (line 36-50)
```rust
MessageEnvelope {
    message_type: "user",
    provider: "user", // sentinel for non-provider input
    message: MessagePayload::User(UserMessage {
        role: "user",
        content: vec![UserContent::Text { text }],
    }),
    ...
}
```

### persist_assistant_message_internal (line 94-112)
```rust
MessageEnvelope {
    message_type: "assistant",
    provider: <param>,
    message: MessagePayload::Assistant(AssistantMessage {
        role: "assistant",
        content,
        stop_reason: stop_reason.or_else(|| Some("unknown".into())), // PROV-039
        ...
    }),
    ...
}
```

### persist_tool_result_internal (line 145-160)
```rust
MessageEnvelope {
    message_type: "user", // tool results ARE user messages by convention
    provider: "tool",     // distinguishes from real user input
    message: MessagePayload::User(UserMessage {
        role: "user",
        content: vec![UserContent::ToolResult {
            tool_use_id, content, is_error, tool_use_result: None,
        }],
    }),
    ...
}
```

### persist_token_state (line 190-213)
Calls `update_session_tokens(manifest, input as u64, output as u64, 0, 0)`
using a cumulative update — does NOT replace the token totals.

## Conclusion

The persistence layer in codelet-agent-loop is already structurally
correct vs the NAPI canonical source map. RPC-080's job is to:

1. Write hermetic unit tests that drive each `persist_*` helper against a
   temp data directory and read back the persisted manifest, proving the
   envelope shape matches the documented contract.
2. Write source-shape regression scenarios that pin the seven canonical
   call sites so any future refactor that drops a call or reorders the
   pair fails CI with a clear message.
3. Link feature coverage so the call-site discipline is documentation.
