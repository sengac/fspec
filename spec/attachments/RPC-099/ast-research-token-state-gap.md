# AST Research — RPC-099 SessionHeader per-session token tracking gap

## Goal
Identify exactly which AST nodes in the Rust port need modification so that the SessionHeader displays **per-session** values for every TokenTracker field (input, output, reasoning, tokens_per_second, cache_read, cache_creation) when the user cycles sessions with Shift+Left/Right.

## Method
Used AstGrep + Grep + Read tools to enumerate the relevant Rust AST nodes and TS Ink reference sites.

---

## 1. Authoritative wire-format shape (codelet/rpc-types/src/lib.rs:766-788)

```rust
pub struct TokenTracker {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_read_input_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
    pub tokens_per_second: Option<f64>,
    pub reasoning_tokens: Option<u32>,
}
```

All six fields are emitted on every `StreamChunk::TokenUpdate { tokens: TokenTracker }`.

## 2. Rust store state (codelet/fspec-tui/src/store/agent_view.rs:40-67) — INSUFFICIENT

```rust
pub struct TokenState {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_fill_pct: u8,
}

fn apply_token_tracker(&mut self, t: &TokenTracker) {
    self.input_tokens = t.input_tokens as u64;
    self.output_tokens = t.output_tokens as u64;
    // BUG: drops cache_read, cache_creation, reasoning_tokens, tokens_per_second
}
```

Per-session HashMap (lines 86): `token_state_by_session: HashMap<SessionId, TokenState>` — correctly per-session in shape, just under-populated in content.

## 3. Header build site (codelet/fspec-tui/src/views/agent/chrome_paint.rs:25-71) — HARDCODES NULLS

```rust
SessionHeader {
    session_index: store.session_index(),
    model,
    thinking,
    tokens,                            // <- per-session (input/output/context_fill_pct only)
    work_unit_id,
    work_unit_status,
    is_isolated: false,
    is_debug_enabled,
    is_select_mode: false,
    tokens_per_second: None,           // <- HARDCODED
    reasoning_tokens: 0,                // <- HARDCODED
    compaction_reduction: None,         // <- HARDCODED
    is_loading,
    subordinate_label: subordinate_label.as_deref(),
}
```

These should be sourced from `store.token_state_for(sid)` per current focused SessionId.

## 4. Header widget shape (codelet/fspec-tui/src/views/agent/header.rs:45-79) — already plumbed

```rust
pub struct SessionHeader<'a> {
    ...
    pub tokens: TokenState,
    pub tokens_per_second: Option<f32>,
    pub reasoning_tokens: u64,
    pub compaction_reduction: Option<i32>,
    ...
}
```

The widget already accepts these fields and passes them into `build_right_line(&self.tokens, self.tokens_per_second, self.reasoning_tokens, self.compaction_reduction)` at lines 104-110. So fix is upstream-only.

## 5. Dispatch routing (codelet/fspec-tui/src/app/dispatch.rs:28-35) — CORRECT

```rust
Action::ChunkReceived(id, chunk) => {
    if let Some(ctx) = self.agent_view_store.session_context_mut_for(id) {
        ctx.record_chunk(chunk);
    }
    self.agent_view_store.apply_chunk_to_token_state(id, chunk);
    self.handle_stream_chunk_state_updates(id, chunk); // RPC-045
    self.maybe_push_error_dialog_for_chunk(chunk);     // RPC-079
}
```

`id` is the chunk's source SessionId (NOT current_session). Bootstrap.rs:92 (RPC-045) forwards chunks for ALL sessions. No filter changes needed.

## 6. Shift+Left/Right dispatch path

- `views/agent/dispatch.rs:29-30`: `KeyCode::Left → Action::SessionPrev`, `KeyCode::Right → Action::SessionNext` (when Shift modifier present — full path in views/agent/dispatch.rs)
- `app/dispatch.rs:216-225`: routes to `handle_session_cycle(±1)`
- `app/dispatch_rpc024.rs:87-149`: `handle_session_cycle` → `switch_to_session_index(idx)` → `focus_session_index(idx)` (store/agent_view.rs:165-169) — ONLY updates `current_session_index`. Does not touch token_state_by_session (correct — every per-session field should be read from the map at render time).

## 7. TS reference (DeepSearch into src/tui/)

### Renderer: src/tui/components/SessionHeader.tsx:104-206
```tsx
const { inputTokens, outputTokens, reasoningTokens } =
  getMaxTokens(tokenUsage, rustTokens);  // L127
...
<Text dimColor>
  tokens: {inputTokens}↓ {outputTokens}↑
  {reasoningTokens > 0 ? ` ${reasoningTokens}🧠` : ''}
</Text>  // L196-197
```

### Per-session Rust subscription: src/tui/hooks/useRustSessionState.ts:245-292
```ts
const snapshot = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
// subscribe/getSnapshot deps on `sessionId` — re-subscribes on switch
```

### Ingestion: src/tui/components/AgentView.tsx:1110-1125
```ts
const updateTokenStateFromChunk = useCallback((chunk: StreamChunk) => {
  if (chunk.type === 'TokenUpdate' && chunk.tokens) {
    setTokenUsage(chunk.tokens);   // FULL TokenTracker
    if (chunk.tokens.tokensPerSecond !== undefined) { ... }
  }
}, []);
```

### Token shape passed to header (src/tui/utils/sessionHeaderUtils.ts:47-53)
```ts
export interface TokenTracker {
  inputTokens: number;
  outputTokens: number;
  cacheReadInputTokens?: number;
  cacheCreationInputTokens?: number;
  reasoningTokens?: number;
}
```

## 8. Required fixes (concrete diff plan)

### (a) Extend `TokenState` in codelet/fspec-tui/src/store/agent_view.rs:40-67
- Add fields: `cache_read_input_tokens: u64`, `cache_creation_input_tokens: u64`, `reasoning_tokens: u64`, `tokens_per_second: Option<f64>`
- Extend `apply_token_tracker` to copy all six fields from the TokenTracker

### (b) Wire chrome_paint.rs:25-71
- Replace hardcoded `tokens_per_second: None` → `tokens.tokens_per_second.map(|v| v as f32)`
- Replace hardcoded `reasoning_tokens: 0` → `tokens.reasoning_tokens`
- (compaction_reduction stays None until a future card; out of scope here — explicit non-goal)

### (c) Header widget (header.rs:45-79)
- No structural change required — already accepts all relevant fields. If desired, may also expose `cache_read_input_tokens`/`cache_creation_input_tokens` on the header struct to surface in `build_right_line` (deferred — TS Ink doesn't render cache numbers in the header strip either; they're only persisted).

### (d) Integration tests (codelet/fspec-tui/tests/agentview_session_header_per_session_tokens_rpc099.rs — NEW)
- Build App with MockBackend + real AgentViewStore.
- Use `append_session` to add s-1 and s-2.
- Dispatch `Action::ChunkReceived(sid, StreamChunk::TokenUpdate { tokens })` for each session with distinct field values.
- Render into `ratatui::backend::TestBackend::new(100, 24)` and scrape the header Buffer text.
- Toggle focus with `Action::SessionNext` / `Action::SessionPrev`, re-render, re-assert.
- Cover all 5 example-mapping scenarios.

## 9. Out of scope (deferred)
- `compaction_reduction` per-session tracking (depends on a separate ContextFillUpdate semantics review — file a follow-up if needed).
- Local React `tokenUsage` mirror parity — Rust uses the single per-session HashMap directly, so the TS local-state mirror has no Rust analogue and isn't needed.
- Persistence layer round-trip (`restore_session_token_state`) — handled in RPC-049.
