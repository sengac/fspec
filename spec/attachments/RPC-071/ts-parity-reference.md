# RPC-071 — TypeScript Ink Parity Reference

> The canonical reference for how each `StreamChunk` variant must be rendered.
> The Rust AgentView's job is to match this behaviour line-for-line.

---

## 1. Where the TS Renderer Lives

| File | Role |
|---|---|
| `src/tui/utils/chunkProcessor.ts` | Pure function — chunks → conversation messages |
| `src/tui/components/AgentView.tsx` | React component — realtime chunk dispatcher |
| `src/tui/hooks/persistent-chunk-handler.ts` | Persistent chunk subscription |

The pure function in `chunkProcessor.ts` is the single source of truth for the
mapping rule. `AgentView.tsx` mirrors the same `if/else if` ladder in its
realtime stream handler for instant UI feedback. Both must agree.

---

## 2. The Ladder (Source-Annotated)

### 2.1 `chunkProcessor.ts:220-380`

```typescript
export function chunksToMessages(
  chunks: StreamChunk[],
  ctx: ChunkProcessorContext
): ConversationMessage[] {
  const messages: ConversationMessage[] = [];
  const pendingToolCalls =
    ctx.pendingToolCalls ?? new Map<string, PendingToolCallInfo>();

  for (const chunk of chunks) {
    if (chunk.type === 'UserInput' && chunk.text) {
      messages.push({
        type: 'user-input',
        content: chunk.text,
      });
    } else if (chunk.type === 'IncomingMessage' && chunk.text) {
      const msg = processSupervisorInputChunk(chunk.text);
      messages.push(msg);
    } else if (chunk.type === 'Text' && chunk.text) {
      // Merge with previous streaming assistant-text message, else create new one.
      const lastIdx = messages.findLastIndex(m => m.type === 'assistant-text');
      if (lastIdx >= 0 && messages[lastIdx].isStreaming) {
        messages[lastIdx].content += chunk.text;
      } else {
        messages.push({
          type: 'assistant-text',
          content: chunk.text,
          isStreaming: true,
        });
      }
    } else if (chunk.type === 'Thinking' && chunk.thinking) {
      // Similar streaming merge for thinking blocks.
      // ...
    } else if (chunk.type === 'ToolCall' && chunk.toolCall) {
      // Push tool-call message; correlate with later tool-result.
      // ...
    } else if (chunk.type === 'ToolResult' && chunk.toolResult) {
      // Resolve pending tool-call; attach the result.
      // ...
    } else if (chunk.type === 'UserNotification' && chunk.message) {
      messages.push({
        type: 'notice',
        content: chunk.message,
        severity: chunk.severity,
      });
    } else if (chunk.type === 'Error' && chunk.error) {
      messages.push({
        type: 'error',
        content: chunk.error,
      });
    } else if (chunk.type === 'Done') {
      // Seal the trailing assistant-text message (isStreaming = false).
      // Does NOT push a visible '[done]' marker — that's the Rust side's choice.
    }
    // ALL OTHER chunk.type values are intentionally ignored here —
    // they are state-only chunks consumed elsewhere in the AgentView.
  }
  return messages;
}
```

### 2.2 `AgentView.tsx` realtime dispatch (lines 280-310, 3490-3520)

The realtime path mirrors the same `if (chunk.type === 'UserInput' && chunk.text)`
gate, which means a missing or empty `text` field also silently skips the chunk.
Our Rust impl must do the same — never crash, never Debug-dump.

### 2.3 What the TS frontend does with the silent chunks

The state-only chunks are routed through `usePersistentChunkHandler` (and the
session-store reducer) into:

| Chunk type | Drives |
|---|---|
| `SessionStateChange` | `<SessionStatusPill>`, pause/HITL dialogs |
| `IsolationStateChange` | `<IsolationBanner>`, status pill colour |
| `DebugStateChange` | `<DebugCapturePanel>` |
| `FooterStateUpdate` | `<SessionFooter>` cwd/branch display |
| `FspecCommandRequest` | `useFspecToolHandler` → response back to LLM |
| `SupervisorPendingInjection` | "supervisor waiting" indicator |
| `CompactionComplete` | `<CompactionToast>`, scrollback summary line |
| `TokenUpdate` | `<TokenMeter>` |
| `ContextFillUpdate` | context-fill % gauge |
| `WorkUnitsUpdate` | `<BoardView>` (NOT the AgentView) |
| `Interrupted` | "[interrupted]" message + queued-input restore |

The Rust side has the exact same wiring already in `dispatch_rpc045.rs` and
`dispatch_rpc053.rs` — the only missing piece is **not** also dumping them
into scrollback.

---

## 3. The Rust Mapping Table (Authoritative)

Use this as the literal match arm order in `chunk_to_lines`. The output
column is what the rendered scrollback line MUST equal byte-for-byte (after
`Line::from(Span::raw(...))` wrapping).

```rust
fn chunk_to_lines(chunk: &StreamChunk) -> Option<Vec<Line<'static>>> {
    let body: String = match chunk {
        // Conversation lines (8 visible variants):
        StreamChunk::UserInput { text }                 => format!("user> {text}"),
        StreamChunk::Text { text, .. }                  => format!("assistant> {text}"),
        StreamChunk::Thinking { thinking, .. }          => format!("(thinking) {thinking}"),
        StreamChunk::IncomingMessage { text, .. }       => format!("supervisor> {text}"),
        StreamChunk::UserNotification { message, .. }   => format!("[notice] {message}"),
        StreamChunk::Error { error }                    => format!("[error] {error}"),
        StreamChunk::Interrupted { queued_inputs }      =>
            format!("[interrupted] {} queued", queued_inputs.len()),
        StreamChunk::Done                               => "[done]".to_string(),

        // State-only variants (12 silent — return None, never appear in scrollback):
        StreamChunk::SessionStateChange { .. }
        | StreamChunk::IsolationStateChange { .. }
        | StreamChunk::DebugStateChange { .. }
        | StreamChunk::FooterStateUpdate { .. }
        | StreamChunk::FspecCommandRequest { .. }
        | StreamChunk::FspecCommandResult { .. }
        | StreamChunk::WorkUnitsUpdate { .. }
        | StreamChunk::SupervisorPendingInjection { .. }
        | StreamChunk::CompactionComplete { .. }
        | StreamChunk::TokenUpdate { .. }
        | StreamChunk::ContextFillUpdate { .. } => return None,

        // Tool variants (3 — deferred to a richer renderer; suppress until then):
        StreamChunk::ToolCall { .. }
        | StreamChunk::ToolResult { .. }
        | StreamChunk::ToolProgress { .. } => return None,
    };
    Some(vec![Line::from(Span::raw(body))])
}
```

**No catch-all arm.** Adding a new variant to `StreamChunk` must force a
compile error in `chunk_to_lines` — this is the only way to prevent another
RPC-045-style regression.

---

## 4. Test Vector — Screenshot Reproduction

This is the literal sequence of chunks the production code emitted for the
screenshot:

```rust
let chunks = vec![
    StreamChunk::UserInput { text: "please review this card".to_string() },
    StreamChunk::SessionStateChange { state: SessionState::Running },
    StreamChunk::SessionStateChange { state: SessionState::Idle },
];
```

After feeding these into `record_chunk` in order, the rendered scrollback
MUST contain exactly one line:

```
user> please review this card
```

(No `UserInput { ... }`, no `SessionStateChange { ... }`. The two state
chunks are absorbed by `handle_stream_chunk_state_updates` only.)
