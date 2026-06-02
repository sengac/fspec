# RPC-100 AST research — compaction percentage gap (Rust vs TS)

Compiled from two parallel DeepSearches plus targeted Grep / Read on
`src/tui/` (TS reference) and `codelet/fspec-tui/src/` (Rust port).

## TS reference: how `[X%]` and `[X%: COMPACTED Y%]` are produced

| Concern | File | Lines |
|---|---|---|
| Bracket text construction | `src/tui/components/SessionHeader.tsx` | 129–132 |
| Decimal formatting | `src/tui/components/SessionHeader.tsx` | 100–102 |
| Color band (50/70/85) | `src/tui/utils/sessionHeaderUtils.ts` | 37–42 |
| `contextFillPercentage` state (per active session) | `src/tui/components/AgentView.tsx` | 1101–1108 |
| Live push from `ContextFillUpdate` chunk | `src/tui/components/AgentView.tsx` | 1110–1125 (handler), 2421–2426 (send loop), 3431–3436 (attach) |
| Background-session replay extraction | `src/tui/components/AgentView.tsx` | 3605–3614 + `tokenStateUtils.ts:51–81` |
| Persisted-session fallback compute | `src/tui/components/AgentView.tsx` | 4234–4260 + `tokenStateUtils.ts:83–117` |
| Reset on `SessionStateChange→cleared` | `src/tui/components/AgentView.tsx` | 992–1006 |
| `compactionReduction` push from `CompactionComplete` | `src/tui/components/AgentView.tsx` | 946–949, 959–979 |

### Renderer bracket logic (verbatim)

```ts
const percentText =
  compactionReduction !== null
    ? `[${formatPercentage(contextFillPercentage)}%: COMPACTED ${formatPercentage(Math.abs(compactionReduction))}%]`
    : `[${formatPercentage(contextFillPercentage)}%]`;
```

### Fallback formula (verbatim — `tokenStateUtils.ts:83-117`)

```ts
const MAX_OUTPUT_RESERVATION = 32000;

export function calculateContextFillPercentage(
  inputTokens: number,
  contextWindow: number,
  maxOutput: number
): number {
  const maxOutputReservation = Math.min(maxOutput, MAX_OUTPUT_RESERVATION);
  const threshold = contextWindow - maxOutputReservation;
  if (threshold <= 0) return 0;
  return Math.round((inputTokens / threshold) * 100);
}
```

The TS comment on `SessionHeader.tsx:101` explicitly notes:
*"Fill percentage (0-100+, can exceed 100 near compaction threshold)"*.

## Rust port — gaps verified by AST `Read`

### 1. Clamp bug (loses >100% signal)

`codelet/fspec-tui/src/store/agent_view/token_state.rs`

```rust
// line 17-33: TokenState struct
pub struct TokenState {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_fill_pct: u8,    // ← u8, clamped to 100
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub reasoning_tokens: u64,
    pub tokens_per_second: Option<f64>,
}

// line 62-64: apply_context_fill
fn apply_context_fill(&mut self, info: &ContextFillInfo) {
    self.context_fill_pct = info.fill_percentage.min(100) as u8;
    //                                          ^^^^^^^^^^^^ ← clamp loses >100%
}
```

Wire type: `ContextFillInfo.fill_percentage: u32`
(`codelet/rpc-types/src/lib.rs:679-684`).

Backend formula source: `codelet/cli/src/interactive/stream_loop.rs:108-126`
emits the raw u32 with no clamp:

```rust
let fill_percentage = if threshold > 0 {
    ((total_tokens as f64 / threshold as f64) * 100.0) as u32
} else { 0 };
```

Conclusion: the backend can and does send 100+ but the TUI store discards the
overshoot.

### 2. Hardcoded `compaction_reduction: None` (dead-code suffix path)

`codelet/fspec-tui/src/views/agent/chrome_paint.rs:55-69`

```rust
SessionHeader {
    session_index: store.session_index(),
    model,
    thinking,
    tokens,
    work_unit_id,
    work_unit_status,
    is_isolated: false,
    is_debug_enabled,
    is_select_mode: false,
    tokens_per_second: tokens.tokens_per_second.map(|v| v as f32),
    reasoning_tokens: tokens.reasoning_tokens,
    compaction_reduction: None,     // ← HARDCODED, never read from store
    is_loading,
    subordinate_label: subordinate_label.as_deref(),
}
.render(areas.header, buf);
```

The render path already supports it:

```rust
// codelet/fspec-tui/src/views/agent/header_build.rs:158-164
let pct_text = match compaction_reduction {
    Some(r) => format!("[{}%: COMPACTED {}%]", tokens.context_fill_pct, r.abs()),
    None    => format!("[{}%]", tokens.context_fill_pct),
};
```

So the rendering branch is reachable — it just never receives a `Some(_)`.

### 3. `CompactionComplete` handler doesn't persist reduction

`codelet/fspec-tui/src/app/dispatch_rpc045.rs:120-133`

```rust
StreamChunk::CompactionComplete { compaction_result } => {
    self.agent_view_store.clear_compaction_progress(session_id);
    let text = format_compaction_notice(compaction_result);
    let _ = self
        .action_tx
        .send(Action::EmitSessionNotice(session_id.clone(), text));
}
```

The reduction value IS computed in `dispatch_rpc020.rs:289-298` (for the
notice text):

```rust
pub(crate) fn format_compaction_notice(result: &CompactionResult) -> String {
    let reduction_pct = (1.0 - result.compression_ratio) * 100.0;
    format!(
        "[compaction] {reduction:.1}% reduction ({orig} → {compacted} tokens, {turns} turns summarised)",
        ...
    )
}
```

…but never stored per-session in the store.

### 4. No reset on `SessionStateChange → Cleared`

`codelet/fspec-tui/src/app/dispatch_rpc045.rs:57-75`

```rust
StreamChunk::SessionStateChange { state } => {
    self.agent_view_store
        .set_session_status(session_id.clone(), session_status_from_state(*state));
    match state {
        SessionState::Paused => { /* dispatch PauseChunkReceived */ }
        SessionState::Running | SessionState::Idle => { /* dispatch PauseCleared */ }
        _ => {}
        // ← no SessionState::Cleared arm; TokenState + compaction_reduction
        //   stay frozen at their last value
    }
}
```

TS reference resets via `setContextFillPercentageRef.current(0)` at
`AgentView.tsx:992-1006` on the equivalent cleared signal.

## Pre-existing reusable infrastructure

| Function | File | Use |
|---|---|---|
| `AgentViewStore::reset_token_state(&SessionId)` | `store/agent_view/work_unit_state.rs:48` | Reuse on `Cleared` |
| `format_compaction_notice(&CompactionResult)` | `app/dispatch_rpc020.rs:289-298` | Reduction formula source |
| `context_fill_color(pct)` | `views/agent/header_build.rs:185-195` | Will work with u16 thresholds 50/70/85 unchanged |
| `tests/common/mod.rs` helpers | `codelet/fspec-tui/tests/common/mod.rs` | `sid`, `build_app`, `render_into` — same pattern as RPC-099 |

## Fix scope summary

| File | Change |
|---|---|
| `store/agent_view/token_state.rs` | `context_fill_pct: u8 → u16`; remove `.min(100)` clamp (clamp to `u16::MAX` instead) |
| `store/agent_view.rs` | Add `compaction_reduction_by_session: HashMap<SessionId, i32>` field |
| `store/agent_view/chrome_state.rs` (or new sibling) | Add `compaction_reduction_for/set_compaction_reduction/clear_compaction_reduction` accessors |
| `app/dispatch_rpc045.rs` | In `CompactionComplete` arm: compute reduction, call `set_compaction_reduction`. In `SessionStateChange` arm: on `Cleared`, call `reset_token_state` + `clear_compaction_reduction` |
| `views/agent/chrome_paint.rs:65` | Replace `compaction_reduction: None` with `sid.and_then(\|s\| store.compaction_reduction_for(s))` |
| `views/agent/header.rs` + `header_build.rs` | Propagate `u8 → u16` for `TokenState.context_fill_pct` and `context_fill_color` |

No routing changes needed — `apply_chunk_to_token_state` already routes by
chunk source-id (RPC-099 confirmed). Backend already emits raw `fill_percentage`
without clamping.
