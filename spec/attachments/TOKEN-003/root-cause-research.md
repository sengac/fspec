# TOKEN-003 Root-Cause Research: Reasoning (🧠) Token Accumulation

**Date:** 2026-08-23
**Work Unit:** TOKEN-003
**Symptom:** The AgentView SessionHeader reasoning-token counter (`N🧠`) displays only the
latest API-reported reasoning value. It does not keep/accumulate the previous value across
segments or turns, unlike the output (`↑`) counter which is session-cumulative.

---

## 1. The full data-flow pipeline (verified by AST read)

```
Provider Usage (rig-core patch)
  → StreamingTokenDisplay.reasoning_tokens   (rust/core/src/streaming_display/streaming_token_display.rs)
  → TokenInfo / StreamEvent::Tokens          (rust/cli/src/interactive/output.rs)
  → StreamChunk::TokenUpdate (TokenTracker)  (rust/napi/src/agent_loop.rs:1639,
                                             rust/agent-loop/src/background_output.rs:377)
  → TUI TokenState.apply_token_tracker       (rust/fspec-tui/src/store/agent_view/token_state.rs:70)
  → SessionHeader "N🧠" suffix              (rust/fspec-tui/src/views/agent/header_build.rs:148)
```

The TUI store is a **pure last-value mirror** — `self.reasoning_tokens =
t.reasoning_tokens.unwrap_or(0)` overwrites on every `TokenUpdate`. That part is by design
(RPC-099): the backend is supposed to be authoritative and send the right value.
**The bug is that the backend never sends an accumulated value** — every layer in between
is "last value wins".

---

## 2. Root causes (four layers, all overwrite-instead-of-accumulate)

### 2.1 `StreamingTokenDisplay` (core) — the main culprit

File: `rust/core/src/streaming_display/streaming_token_display.rs`

- `reasoning_tokens` (line 104) is a single "last known value" field.
- `update_from_usage()` (line 232): `self.reasoning_tokens = usage.reasoning_tokens.unwrap_or(0)`
  — plain overwrite.
- `update_from_final_response()` (line 282): same overwrite.
- `start_new_segment()` (line 244): accumulates the previous segment's **output** into
  `cumulative_base` (via `OutputTokenTracker::start_new_segment`,
  `output_token_tracker.rs:84-93`), but does **nothing** for reasoning. So in a
  multi-segment tool loop, each new API segment's value replaces the previous segment's.
- The constructors (`new` line 131, `from_cache_inclusive_total` line 161) hardcode
  `reasoning_tokens: 0`. The previous turn's value is **never seeded in** — compare with
  `prev_output`/`prev_input` which are threaded through from `session.token_tracker` at
  `stream_loop.rs:563-586`.

### 2.2 Session `TokenTracker` (core) — the value is destroyed at end of turn

File: `rust/core/src/compaction/model.rs`

- `update_from_usage()` (line 168): `self.reasoning_tokens = usage.reasoning_tokens`
  — overwrite, not accumulate.
- The end-of-turn update at `stream_loop.rs:2252-2265` builds
  `ApiTokenUsage::new(final_display.input_tokens, cache_read, cache_creation, per_turn_output_delta)`
  **without** `.with_reasoning_tokens(final_display.reasoning_tokens)` — so it defaults to 0,
  and the session tracker's `reasoning_tokens` is **reset to 0 at the end of every turn**.
  Same pattern in `gemini_continuation.rs:116-124` (`update_display_only`) and
  `update_token_tracker` (`gemini_continuation.rs:468-484`).
- `reset_after_compaction()` (line 204) also zeroes `reasoning_tokens`.

Net effect: the per-session value is 0 between turns, and mid-turn it's only the
*current* segment's value. This is exactly the observed symptom.

### 2.3 Persistence — reasoning tokens are never saved

- `persist_token_state()` (`rust/agent-loop/src/persist.rs:198`,
  `rust/napi/src/persist.rs:191`) only takes `input_tokens, output_tokens`.
- The manifest `TokenUsage` struct (`rust/core/src/persistence/manifest.rs:44`) has
  **no reasoning field at all**. So on `/resume` the 🧠 count is gone entirely.
- Note: `spec/features/reasoning-token-tui-display.feature` has a
  "Token persistence saves and restores reasoning tokens" scenario marked `@done` —
  that was true for the old TS layer; the Rust port never implemented it.

### 2.4 Provider coverage gap (context, not strictly a bug)

Only OpenAI-compatible providers (o-series, Codex, Z.AI/GLM via
`completion_tokens_details.reasoning_tokens`) report reasoning tokens, and only in the
**final** usage chunk:

- `rust/patches/rig-core/src/providers/openai/completion/streaming.rs:94-96` — sets
  `usage.reasoning_tokens` from `completion_tokens_details` in the final response.
- Anthropic: `PartialUsage` (`rust/patches/rig-core/src/providers/anthropic/streaming.rs:80-88`)
  has **no reasoning field at all** — the API folds thinking tokens into `output_tokens`.
  The `MessageDelta` Usage event (lines 348-354) carries no reasoning value.
- Gemini: `GetTokenUsage` impl (`rust/patches/rig-core/src/providers/gemini/streaming.rs:59-70`)
  folds `thoughts_token_count` into `output_tokens` (lines 33-35); `reasoning_tokens`
  is never set.

So the 🧠 counter is structurally invisible for Anthropic/Gemini unless we estimate it
from the `ReasoningDelta` text (which the stream loop already receives at
`stream_loop.rs:918-936` and could count with the same tiktoken estimator
`OutputTokenTracker` uses).

---

## 3. What the TUI side already does correctly (do NOT change)

- `rust/fspec-tui/src/store/agent_view/token_state.rs:70` — `apply_token_tracker` copies
  `reasoning_tokens` from the wire chunk verbatim. Last-value mirror is correct IF the
  backend sends the right (cumulative) value.
- `rust/fspec-tui/src/views/agent/header_build.rs:148-152` — renders ` {N}🧠` when
  `reasoning_tokens > 0`.
- `rust/fspec-tui/src/views/agent/chrome_paint.rs:72` — sources `reasoning_tokens` from
  the per-session `TokenState` (RPC-099).
- The context-fill recompute at `token_state.rs:98` already includes reasoning tokens in
  the effective-context sum.
- Pinned by `rust/fspec-tui/tests/agentview_session_header_per_session_tokens_rpc099.rs`
  and `rust/fspec-tui/tests/token_state_realtime_recompute_rpc101.rs`.

---

## 4. Existing related work units (context)

- **RIG-012** (done) — wired `reasoning_tokens` through the whole pipeline
  (`ApiTokenUsage` → `TokenDisplayUpdate` → `TokenInfo` → NAPI `TokenTracker` → TUI).
  Feature files: `reasoning-token-core-propagation.feature`,
  `reasoning-token-napi-bridge.feature`, `reasoning-token-tui-display.feature`,
  `reasoning-token-cli-mapping.feature`. Research:
  `spec/attachments/RIG-012/ast-research-reasoning-token-pipeline.md`.
  RIG-012 only made the value *flow* — it never made it *accumulate*.
- **TOKEN-001** (done) — fixed `cumulative_billed_output` by introducing
  `TokenTracker::compute_output_delta` (centralized per-turn delta helper,
  `model.rs:236`). The same DRY pattern applies to the reasoning fix.
- **PROV-041** (done) — thinking-token exhaustion recovery; reads
  `usage.reasoning_tokens` per-segment (not cumulative) for budget detection —
  unaffected by this change.

---

## 5. Reference implementation (Codex)

From `spec/attachments/RIG-012/ast-research-reasoning-token-pipeline.md`:

- `codex-rs/protocol/src/protocol.rs:1527` — `TokenUsage` has
  `reasoning_output_tokens: i64`.
- `protocol.rs:1695` — `add_assign` **includes reasoning** (accumulates).
- `protocol.rs:1729` — display shows "(reasoning N)" when > 0.

Design decision (settled with user): the header should show **session-cumulative**
reasoning tokens, matching the `↑` output counter semantics and the Codex reference.

---

## 6. Fix plan (ACDD scope)

1. **`StreamingTokenDisplay`** (core): make reasoning cumulative like output —
   - add `reasoning_cumulative_base: u64` + `reasoning_current: u64` (or equivalent),
   - accumulate the previous segment's reasoning in `start_new_segment()`,
   - `update_from_usage` / `update_from_final_response` set the *current segment* value,
   - `current_display()` reports `cumulative_base + current`,
   - thread a `prev_reasoning` seed through the constructors (from
     `session.token_tracker.reasoning_tokens` at the 6 seed sites in `stream_loop.rs`
     (lines 581, 722, 1384, 1641, 1940, 2044) + 2 in `gemini_continuation.rs`
     (lines 164, 349)).
2. **End-of-turn updates**: carry `final_display.reasoning_tokens` into the final
   `ApiTokenUsage` via the existing `with_reasoning_tokens()` builder
   (`rust/core/src/token_usage.rs:53`) — it currently exists but is only used for the
   context-fill calc at `stream_loop.rs:1066/1124`. Sites: `stream_loop.rs:2252-2265`,
   `gemini_continuation.rs:116-124` and `:475-483`.
3. **`TokenTracker`** (`model.rs`): `update_from_usage` / `update_display_only` store the
   (now cumulative) reasoning value. Decide `reset_after_compaction()` policy —
   recommendation: keep the cumulative (like `cumulative_billed_*`), since it is a
   session-spend metric, not a context metric.
4. **Persistence**: add `reasoning_tokens` to the manifest `TokenUsage` struct
   (`manifest.rs:44`), extend `persist_token_state` / `update_session_tokens` /
   `set_session_tokens` (both `rust/agent-loop/src/persist.rs` and
   `rust/napi/src/persist.rs` twins) and restore it on resume.
5. **Optional / separate work unit (OUT OF SCOPE for this card):** estimate reasoning
   tokens from `ReasoningDelta` text for Anthropic/Gemini so the 🧠 badge works for
   those providers too.

### Invariants that MUST hold after the fix

- The 🧠 counter is monotonically non-decreasing within a session (never ticks back),
  matching the `↑` output counter behavior.
- `TokenUpdate` chunks on the wire carry the session-cumulative reasoning value, so the
  TUI last-value mirror keeps working unchanged.
- Context-fill percentage (RPC-419 formula) continues to use the *current* physical
  context — note: after this change, the TUI recompute at `token_state.rs:98`
  (`input + output + reasoning`) will use cumulative reasoning; verify the backend
  `emit_context_fill_from_usage` values remain authoritative and consistent (the
  backend already adds current-segment reasoning via `with_reasoning_tokens`).
- Compaction threshold checks (`CompactionHook` / `TokenState`) are based on context
  occupancy, not the cumulative reasoning display — do not let the cumulative display
  value leak into threshold math.

### Testing notes (ACDD phase 2)

- Core unit tests: `rust/core/tests/rig_012_reasoning_token_propagation_test.rs`
  exists — extend with accumulation scenarios (multi-segment, multi-turn, tick-back).
- Existing display tests in `streaming_token_display.rs` use `make_usage` with
  `reasoning_tokens: None` — new tests must exercise `Some(n)`.
- CLI integration tests pattern: `rust/cli/tests/token_001_cumulative_billed_output_test.rs`
  (TOKEN-001) is the closest template for an end-of-turn accumulation test.
- Persistence: `rust/agent-loop/tests/rpc080_agent_loop_persistence.rs` covers
  `persist_token_state` — extend for the reasoning field.
- TUI side needs NO changes; if a regression appears, `agentview_session_header_per_session_tokens_rpc099.rs`
  is the pin.
