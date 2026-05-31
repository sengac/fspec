# RPC-093 — Streaming Text + Thinking Port Findings

**Status:** Research complete. Ready for ACDD (specifying → testing → implementing).
**Scope:** Lift the TypeScript Ink streaming pipeline (`Text` + `Thinking` chunks)
into the Rust ratatui port (`codelet/fspec-tui`). Fix the visual bug
where every `StreamChunk::Thinking` delta renders as its own
`[Thinking]\n<delta>` scrollback row.

**Aligned with:** RPC-002 (NAPI-free dual-transport boundary), RPC-091
(chunk-processor accumulation), RPC-078 (scrollback parity).

---

## 1. Symptom (from screenshot 2026-05-29 18:19)

```
[Thinking]
The user is asking about
[Thinking]
how well a "card" was done. This is likely referring to a work unit in
[Thinking]
fspec. Let me check the board to see what work units are in
[Thinking]
progress or recently completed.
```

Each line above is a **separate** `RenderedChunk` in the scrollback —
every streamed thinking delta becomes its own row, each beginning with
`[Thinking]`. The TS Ink frontend renders this as ONE yellow block:

```
[Thinking]
The user is asking about how well a "card" was done. This is
likely referring to a work unit in fspec. Let me check the board
to see what work units are in progress or recently completed.
```

---

## 2. Root cause

`codelet/fspec-tui/src/store/agent_view/session_context.rs:80-86`:

```rust
StreamChunk::Thinking { thinking, .. } => {
    self.push_chunk(ChunkSource {
        text: format!("[Thinking]\n{thinking}"),
        color: Color::Yellow,
        kind: ChunkKind::Thinking,
        is_streaming: false,
    });
}
```

Three independent defects on this branch:

1. **No accumulation.** Every delta calls `push_chunk` → a fresh
   `RenderedChunk` is allocated. Consecutive `Thinking` deltas for the
   same logical block do NOT merge.
2. **`[Thinking]\n` baked into `source.text` per delta.** Even after
   accumulation parity is fixed, the prefix must move to render-time
   (parity with `'● '` for AssistantText per RPC-091 rule [1]).
3. **`is_streaming: false` from the first chunk.** Streaming indicator
   (`...`) and "in-flight" semantics are lost; finalisation triggers
   (`Done`/`ToolCall`/`Error`/`UserInput`/`Interrupted`) have nothing to
   finalise.

The neighbouring `StreamChunk::Text` branch (RPC-091) correctly
accumulates — `append_assistant_text` (`chunk_processor.rs:19-38`)
mutates the in-flight chunk via `chunks_mut().get_mut(idx)` and
re-wraps via `scrollback.rewrap_at(idx)`. Thinking has no equivalent.

There is also no `in_flight_thinking: Option<usize>` slot on
`SessionContext`, no `flush_in_flight_thinking_drop_empty` helper, and
no port of `findActiveThinkingBlock`'s turn-boundary check.

---

## 3. End-to-end TypeScript ground truth

### 3.1 Chunk dispatch (single point of entry)

One NAPI callback registered once at startup
(`src/tui/services/globalSessionStreamManager.ts:151-173`):

```ts
napi.sessionSetGlobalChunkCallback(
  (err, args) => {
    if (err || !args || !args.sessionId || !args.chunk) return;
    this.handleChunk(args.sessionId, args.chunk);
  }
);
```

`handleChunk` fans out to per-session handler sets
(`globalSessionStreamManager.ts:316-404`). React components register
via `useSessionStreamManager` (`src/tui/hooks/useSessionStreamManager.ts:39-66`).

In `AgentView.tsx:981-1045`, the persistent handler is the **sole**
dispatch into `processStreamingChunk`:

```ts
const persistentChunkHandler = useCallback(
  (routedSessionId, chunk) => {
    if (!chunk || sessionCleanupRef.current) return;
    if (chunk.type === 'SessionStateChange') { /* … */ return; }
    if (chunk.type === 'CompactionComplete') { /* … */ return; }
    const ctx: ChunkProcessorContext = { formatToolHeader, formatCollapsedOutput, pendingToolCalls };
    setConversation(prev => {
      const updated = [...prev];
      processStreamingChunk(chunk, updated, ctx);
      return updated;
    });
  },
  [setConversation, setTokenUsage]
);
useSessionStreamManager(currentSessionId, persistentChunkHandler);
```

**One chunk → one `setConversation` update.** Zero application-level
batching, throttling, debouncing, or buffering. The only smoothing is
Ink's intrinsic ~60fps render coalescing
(`src/tui/config/inkConfig.ts:18`).

`AgentView.tsx:1322-1326` documents the deliberate absence of
`useDeferredValue`:

```ts
// PERF-003: Previously used useDeferredValue here, but Ink uses LegacyRoot
// (synchronous rendering) which means deferred values always lag one render
// behind. … The line cache (lineCacheRef) already handles perf.
const deferredConversation = conversation;
```

### 3.2 Text accumulation (assistant-text)

`src/tui/utils/chunkProcessor.ts:444-461`:

```ts
if (chunk.type === 'Text' && chunk.text) {
  const lastIdx = conversation.findLastIndex(m => m.type === 'assistant-text');
  if (lastIdx >= 0 && conversation[lastIdx].isStreaming) {
    conversation[lastIdx] = {
      ...conversation[lastIdx],
      content: conversation[lastIdx].content + chunk.text,
    };
  } else {
    conversation.push({
      type: 'assistant-text',
      content: chunk.text || '',
      isStreaming: true,
    });
  }
  return true;
}
```

**Rule:** append to the trailing `assistant-text` only if
`isStreaming === true`. Otherwise start a fresh streaming bubble.

`isStreaming` lifecycle:

- **Created** `true` on first `Text` after a turn/tool boundary.
- **Cleared** to `false` on:
  - `ToolCall` (`chunkProcessor.ts:487-497`) — empty bubbles spliced out.
  - `Done` (`chunkProcessor.ts:538-557`) — also runs `formatMarkdownTables`.
  - `Interrupted` / `Error` (`chunkProcessor.ts:359-401, 560-575`).
- After `ToolResult` a fresh empty streaming placeholder is pushed
  (`chunkProcessor.ts:529-535`) so the next `Text` continues.

Tokens are NOT split char-by-char. `chunk.text` is whatever Rust
emitted — the TS layer treats it as opaque.

### 3.3 Thinking accumulation (the missing port target)

`src/tui/utils/chunkProcessor.ts:463-466`:

```ts
if (chunk.type === 'Thinking' && chunk.thinking) {
  appendThinking(conversation, chunk.thinking);
  return true;
}
```

All real logic lives in `src/tui/utils/thinkingBlockManager.ts`. Key
contracts that MUST be ported:

**Constant** (`thinkingBlockManager.ts:36`):

```ts
const THINKING_PREFIX = '[Thinking]\n';
```

The prefix lives ONCE in `content` (idempotent — `appendThinking`
strips it before re-concatenating). Render layer prepends nothing.

**`findActiveThinkingBlock`** (`thinkingBlockManager.ts:58-79`):

- Find the last message where `type === 'thinking' && isStreaming === true`.
- If any `user-input` / `supervisor-input` appears AFTER that index,
  return `-1` (turn boundary — the streaming block is stale).
- Otherwise return the index.

**`appendThinking`** (`thinkingBlockManager.ts:139-181`):

```
activeIdx = findActiveThinkingBlock(messages)
if activeIdx >= 0:
    existing = messages[activeIdx]
    body = strip-prefix(existing.content)
    messages[activeIdx] = { ...existing, content: PREFIX + body + delta }
else:
    new = { type: 'thinking', content: PREFIX + delta, isStreaming: true }
    streamingAssistantIdx = findLastIndex(type==='assistant-text' && isStreaming)
    if streamingAssistantIdx >= 0:
        splice(streamingAssistantIdx, 0, new)   # insert BEFORE
    else:
        push(new)
```

The splice-before-assistant rule keeps thinking blocks visually
"above" the still-streaming text bubble. Rust port note: today
`record_chunk` only ever pushes to the tail; the insertion semantics
need explicit modelling against `ScrollbackList`.

**`finalizeThinkingBlock`** (`thinkingBlockManager.ts:194-207`):

```
activeIdx = findActiveThinkingBlock(messages)
if activeIdx >= 0:
    messages[activeIdx] = { ...messages[activeIdx], isStreaming: false }
```

**Finalization triggers** in `chunkProcessor.ts`:

- `ToolCall` (line 469): `finalizeThinkingBlock(conversation)` BEFORE
  pushing the tool card.
- `Done` (line 538-558): no explicit thinking finalize — the assistant
  flush implicitly leaves the last thinking block as the final one;
  `isStreaming` becomes irrelevant after `Done`.
- `Error` (line 560-580): no explicit thinking finalize either, but
  the next stream cycle starts with a fresh `findActiveThinkingBlock`
  result of `-1` because the surrounding context resets.
- `Interrupted` (line 359-401): assistant-text only is rewound.
- `UserInput`: implicit via the turn-boundary rule in
  `findActiveThinkingBlock` — no per-chunk handler needed.

> Therefore the **only explicit `finalizeThinkingBlock` call site is
> `ToolCall`**. Every other finalisation falls out of the "no streaming
> across turn boundary" invariant. The Rust port must mirror this:
> calling `finalize_in_flight_thinking` from every flush handler is
> incorrect; only `handle_tool_call` should do it.


### 3.4 Render side (TS Ink) — no batching

`src/tui/conversation/utils.ts` builds `ConversationLine[]` from
`ConversationMessage[]`. Every state update triggers a full rebuild
under a `lineCacheRef` keyed on message identity + width. The
`isStreaming: true` thinking block is rendered identically to a
finalised one — the marker only controls the accumulation contract,
not the visual output. The "streaming dots" indicator is a separate
spinner driven by `streaming` boolean in `AgentView` props.

There is no buffering / throttling layer. The TS Ink rule is:

> **One chunk → one `setConversation` → one React render →
> Ink coalesces draws at ~60fps.**

The Rust ratatui port must achieve the same end-effect with:

> **One chunk → one `record_chunk` → in-place mutation +
> `scrollback.rewrap_at(idx)` → ratatui's draw loop redraws
> on the next tick.**

Both pipelines have the same data-flow guarantee: the screen sees
every accumulated state, never a per-delta repetition.

---

## 4. Rust port targets

### 4.1 `SessionContext` state additions

```rust
pub struct SessionContext {
    // ...existing fields...
    pub in_flight_assistant: Option<usize>,   // RPC-091
    pub in_flight_thinking:  Option<usize>,   // RPC-093 (NEW)
}
```

`in_flight_thinking` is the analogue of "the last thinking message
where `isStreaming === true`, with no `UserInput` after it".

### 4.2 `chunk_processor::append_thinking` (NEW)

Mirrors `appendThinking` + `findActiveThinkingBlock`:

```rust
pub fn append_thinking(ctx: &mut SessionContext, delta: &str) {
    if delta.is_empty() { return; }

    if let Some(idx) = ctx.in_flight_thinking {
        if let Some(chunk) = ctx.scrollback.chunks_mut().get_mut(idx) {
            if let Some(source) = chunk.source.as_mut() {
                // Body lives without the prefix; render layer adds it.
                source.text.push_str(delta);
            }
        }
        ctx.scrollback.rewrap_at(idx);
        return;
    }

    // No active thinking block — create one.
    let source = ChunkSource {
        text: delta.to_string(),
        color: Color::Yellow,
        kind: ChunkKind::Thinking,
        is_streaming: true,
    };
    let new_idx = ctx.scrollback.chunk_count();
    ctx.push_source(source);
    ctx.in_flight_thinking = Some(new_idx);
}
```

> **Prefix decision:** unlike the TS impl (which stores
> `'[Thinking]\n' + body` in `content`), the Rust render path already
> has a `ChunkKind::Thinking` discriminant. Move the `[Thinking]\n`
> prefix to render-time. This mirrors the `'● '` decision for
> AssistantText per RPC-091 rule [1] and keeps `source.text` as the
> raw model output (matches the chunk processor's "opaque text" rule).

### 4.3 Finalisation triggers

Only `handle_tool_call` should call:

```rust
pub fn finalize_in_flight_thinking(ctx: &mut SessionContext) {
    if let Some(idx) = ctx.in_flight_thinking.take() {
        if let Some(chunk) = ctx.scrollback.chunks_mut().get_mut(idx) {
            if let Some(source) = chunk.source.as_mut() {
                source.is_streaming = false;
            }
            ctx.scrollback.rewrap_at(idx);
        }
    }
}
```

Wire-up:

| Trigger        | `in_flight_assistant`      | `in_flight_thinking`              |
|----------------|----------------------------|-----------------------------------|
| `Text`         | append/create              | —                                 |
| `Thinking`     | —                          | append/create (**NEW**)           |
| `ToolCall`     | flush_drop_empty           | finalize (**NEW**)                |
| `ToolResult`   | restart placeholder        | —                                 |
| `ToolProgress` | —                          | —                                 |
| `Done`         | finalize + table-format    | clear slot only (no isStreaming)  |
| `Error`        | flush_drop_empty           | clear slot only                   |
| `UserInput`    | flush_drop_empty           | clear slot only (turn boundary)   |
| `Interrupted`  | flush_drop_empty           | clear slot only                   |

> "Clear slot only" = `ctx.in_flight_thinking = None;` without
> mutating the chunk — matches TS where the existing thinking content
> stays visible, but the next `Thinking` delta cannot append to it
> because `findActiveThinkingBlock` returns `-1` once `isStreaming`
> is false OR a turn-boundary appears.

### 4.4 Insertion-before-assistant rule

TS `appendThinking` splices NEW thinking blocks *before* the
in-flight assistant message. In the Rust port the equivalent is:
when `append_thinking` creates a new block AND
`in_flight_assistant.is_some()`, insert the new chunk at
`in_flight_assistant_idx` and increment `in_flight_assistant` by 1
to keep the slot pointing at the same chunk.

```rust
// Inside append_thinking, "no in-flight thinking" branch:
let source = ChunkSource { /* … */ };
match ctx.in_flight_assistant {
    Some(assist_idx) => {
        ctx.scrollback.insert_chunk_at(assist_idx, source); // NEW helper
        ctx.in_flight_thinking = Some(assist_idx);
        ctx.in_flight_assistant = Some(assist_idx + 1);
    }
    None => {
        let new_idx = ctx.scrollback.chunk_count();
        ctx.push_source(source);
        ctx.in_flight_thinking = Some(new_idx);
    }
}
```

`ScrollbackList::insert_chunk_at` does not exist yet — see Open
question Q1.


### 4.5 Render-time prefix

In `views/agent/scrollback.rs` (or wherever `ChunkKind::Thinking`
becomes lines), prepend `[Thinking]\n` when rendering. Pseudo-diff:

```rust
ChunkKind::Thinking => {
    let display_text = format!("[Thinking]\n{}", source.text);
    wrap_source(&display_text, color, width)
}
```

Today the prefix is in `source.text` and stripped/re-added per delta
in TS — the Rust port collapses both steps by storing only the body
and prepending once at draw.

---

## 5. ACDD scope (this card)

**IN scope:**

1. Add `in_flight_thinking: Option<usize>` to `SessionContext`.
2. Implement `chunk_processor::append_thinking` with prepend-or-append
   semantics matching `appendThinking` + `findActiveThinkingBlock`.
3. Implement `chunk_processor::finalize_in_flight_thinking` and wire
   it from `handle_tool_call` ONLY.
4. Clear `in_flight_thinking` (no mutation) in
   `flush_in_flight_drop_empty`, `handle_done`, `handle_error`, and
   the `UserInput` / `Interrupted` arms of `record_chunk`.
5. Replace the broken `StreamChunk::Thinking` branch in
   `record_chunk` with `append_thinking(self, thinking)`.
6. Move `[Thinking]\n` prefix from `source.text` to render-time in
   `views/agent/scrollback.rs` (or equivalent).
7. Add `ScrollbackList::insert_chunk_at(idx, ChunkSource)` and the
   bookkeeping that bumps `in_flight_assistant` when a thinking
   block is spliced before it.

**OUT of scope (separate cards if needed):**

- Streaming indicator dots (`...`) parity — see RPC-091 deferral note
  in `session_context.rs`. Track as a new card if regressed.
- `appendThinkingBulk` parity (only used by transcript hydration in
  TS; Rust port doesn't currently rehydrate).
- Markdown formatting inside thinking blocks (TS doesn't format
  thinking either — only `format_markdown_tables` on assistant text
  via `handle_done`).
- Any throttling / batching — both TS and Rust agree: one chunk →
  one update → renderer coalesces.

---

## 6. Open questions

- **Q1**: Should `ScrollbackList::insert_chunk_at` live on
  `ScrollbackList` (mirrors `Vec::insert`) or on `SessionContext`
  as a convenience? Lean: on `ScrollbackList`, with a
  `SessionContext::insert_source_at` helper that also handles
  `scrollback_next_seq` and `rewrap_at`.
- **Q2**: Does any existing test depend on the current
  `[Thinking]\n<delta>` text being in `source.text`? (Search
  `agentview-chunk-rendering-parity.feature` + tests for the literal
  `"[Thinking]"`.)
- **Q3**: When the in-flight assistant chunk is the *last* chunk and
  a Thinking arrives, should the new thinking block go BEFORE it
  (TS-faithful) or AFTER it (cosmetically nicer for ratatui scroll
  semantics)? TS-faithful is safer — defer cosmetic change.

---

## 7. Verification plan (testing phase)

1. Unit test: 3 consecutive `StreamChunk::Thinking` deltas produce
   exactly ONE `RenderedChunk` whose body concatenates the deltas
   and whose render output starts with `[Thinking]\n`.
2. Unit test: `Thinking` → `ToolCall` → `Thinking` produces TWO
   thinking chunks, each finalised independently.
3. Unit test: `Thinking` → `UserInput` → `Thinking` produces TWO
   thinking chunks (turn boundary).
4. Unit test: streaming `Text` interleaved with `Thinking` keeps the
   thinking block *above* the in-flight assistant chunk.
5. Unit test: `Done` after `Thinking` does NOT clear the chunk; it
   only clears the `in_flight_thinking` slot.
6. Integration: replay the screenshot transcript (4 deltas of one
   logical thought) → one scrollback row.
