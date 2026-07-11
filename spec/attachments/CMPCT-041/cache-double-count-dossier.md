# CMPCT-041 — Turn-Start Seed Cache Double-Count + `pre_compaction_tokens` Basis Split

**Date:** 2026-07-09
**Source:** Supervisor DeepSearch audit (post-CMPCT-038), confirmed with exact arithmetic.
**Type:** Bug — token accounting integrity (display, compaction reporting, billing analytics)

---

## 1. The Double-Count Mechanism (Mode A), Step by Step

1. End of turn N-1: TokenTracker::update_from_usage
   (codelet/core/src/compaction/model.rs:152-169) sets
   input_tokens = usage.total_input() (line 158 — CACHE-INCLUSIVE, per PROV-001
   "Store TOTAL context") AND simultaneously cache_read_input_tokens = Some(...),
   cache_creation_input_tokens = Some(...) (lines 165-166).
2. Start of turn N: codelet/cli/src/interactive/stream_loop.rs:577-583 reads all
   three back: prev_input_tokens (cache-inclusive total), prev_cache_read,
   prev_cache_creation (non-zero). Lines 590-595 seed
   StreamingTokenDisplay::new(prev_input, prev_output, prev_cache_read, prev_cache_creation).
3. The emit: stream_loop.rs:604 — output.emit_tokens(&streaming_display.current().into()).
   current() yields TokenDisplayUpdate { input_tokens: prev_input, cache_read,
   cache_creation } (streaming_token_display.rs:248-257); the .into() —
   impl From<TokenDisplayUpdate> for TokenInfo at
   codelet/cli/src/interactive/output.rs:62 — computes
   input_tokens: update.total_input() where total_input() = input + cache_read +
   cache_creation (streaming_token_display.rs:46-48). CACHE IS ADDED A SECOND TIME.
4. Landing: BackgroundOutput implements StreamOutput (background_output.rs:4-11);
   the StreamEvent::Tokens(info) arm calls session.update_tokens(info.input_tokens ...)
   (background_output.rs:221-224; NAPI twin napi/src/agent_loop.rs:1631-1634) →
   cached_input_tokens stores the inflated value (background_session.rs:772-777).

WORKED EXAMPLE: true context 180,000 = 30,000 raw + 150,000 cache_read.
Turn N-1 → tracker.input_tokens = 180,000, cache_read = Some(150,000).
Turn N seed emit → TokenInfo.input_tokens = 180,000 + 150,000 = 330,000 (1.83x).

The code already knows this hazard elsewhere: stream_loop.rs:396-401 deliberately
zeroes cache fields when building TokenState — comments "// Don't double count" —
but NO equivalent guard exists at the seed sites.

Seed/re-seed sites (ALL must be fixed): stream_loop.rs:590-595 plus identical
re-seed sites at ~729, ~1339, ~1705, ~1809 (verify by searching
StreamingTokenDisplay::new in the file).

Self-heal window: mid-stream Usage events correct the display
(streaming_token_display.rs:178-196 sets raw input + cache separately), so the
inflation window is [turn start → first Usage event of turn N] — exactly the window
where a "prompt is too long" rejection occurs.

## 2. Consequence A — Inflated pre_compaction_tokens on Overflow Recovery

- Pre-prompt auto-compaction (stream_loop.rs:350-357): emit_compaction_started()
  fires BEFORE the seed emit → correct snapshot. NOT affected.
- Overflow recovery paths B/C/D (recovery_compaction.rs:328): fires AFTER the seed.
  If the request was rejected before any authoritative Usage event,
  cached_input_tokens = 330,000 at CompactionStarted time → AUTO writers snapshot
  330k into pre_compaction_tokens:
  - codelet/agent-loop/src/background_output.rs:278-283
  - codelet/napi/src/agent_loop.rs:1692-1697
- Post-CMPCT-038, the emit site (agent-loop/src/agent_loop.rs:1244-1250,
  napi/src/agent_loop.rs:1222-1228) passes pre_compaction_tokens VERBATIM as
  CompactionComplete.original_tokens → the user is shown a context that never
  existed (~83% overstated), inflating the reduction percent.

## 3. Consequence B — Tracker Corruption + Billing Inflation

flush_partial_state_before_compaction (recovery_compaction.rs:184-200) — called by
begin_compaction_recovery on every overflow recovery — builds
ApiTokenUsage::new(current.input_tokens, current.cache_read_tokens,
current.cache_creation_tokens, delta) FROM THE DISPLAY and calls
tracker.update_from_usage(&usage, ...):
- If no usage event arrived this turn (exactly the prompt-too-long case), the
  TRACKER ITSELF is corrupted to the double-counted 330k value.
- cumulative_billed_input += 180,000 — billing analytics inflated with cache tokens.

## 4. Consequence C — Auto vs Manual Basis Split (four writers)

| Path | Writer | Basis |
|---|---|---|
| AUTO (agent-loop) | background_output.rs:278-283 | session.cached_input_tokens (display basis; double-count-susceptible) |
| AUTO (NAPI twin) | napi/src/agent_loop.rs:1692-1697 | same |
| MANUAL (RPC) | sessions/src/handle_impl.rs:295-298 | inner.token_tracker.input_tokens (PROV-001 API basis) |
| MANUAL (NAPI) | napi/src/session_bindings.rs:3051-3055 | same |

In steady state the bases agree; they diverge in (a) the seed window (double-count)
and (b) post-compaction windows where recalculate_token_tracker switches to a
count_tokens() estimate basis. Result: auto vs manual compaction of identical
contexts can report different original_tokens.

## 5. The Fix

CANONICAL BASIS: token_tracker.input_tokens (PROV-001: provider-reported
total_input() of the last completed API request — token_usage.rs:5-12).

1. Kill the double-add at the root. At EVERY StreamingTokenDisplay seed/re-seed
   site in stream_loop.rs (~590-595, ~729, ~1339, ~1705, ~1809): seed cache fields
   as 0 (mirroring the deliberate "// Don't double count" pattern at
   stream_loop.rs:396-401). Preferred over changing TokenInfo::from / total_input()
   semantics, which are correct for genuine mid-stream updates where input_tokens
   is raw. This simultaneously fixes: the seed emit (:604), the
   flush_partial_state_before_compaction tracker corruption, and the billing
   inflation (all three consume the same display state).
2. Unify the four pre_compaction_tokens writers. The two AUTO writers must snapshot
   the same canonical source as the manual path. Route all four through a single
   accessor (e.g. a BackgroundSession snapshot method) or have the AUTO
   CompactionStarted arms read the tracker-backed value, so drift is structurally
   impossible across the NAPI/agent-loop twins. (If reading the tracker under the
   session lock mid-stream is unsafe at that point, an acceptable alternative is
   keeping cached_input_tokens as the source AFTER step 1 guarantees it is never
   double-counted — but document that equivalence and add the parity test in §6.3.
   Prefer the single-accessor approach.)
3. Do NOT regress: the "// Don't double count" TokenState path
   (stream_loop.rs:396-401), mid-stream Usage self-heal
   (streaming_token_display.rs:178-196), CMPCT-038 emit-at-apply behavior, and
   pre-prompt auto-compaction (currently correct).

## 6. Required Test Coverage (currently effectively NONE)

Existing tests do NOT cover this: compaction_post_inject_loading_test.rs:82-118 uses
an arbitrary AtomicU32::new(50_000) (ordering only); rpc086_token_tracking.rs
:105-116,277-295 is a source-text assertion that LOCKS IN the current
double-count-susceptible wiring (inspect and update if wiring changes);
rpc418_compact_session.rs:262-274 only asserts original > 0 and compacted < original;
token_inflation_bug_test.rs covers a DIFFERENT historical double-count.

1. Seed regression test: seed StreamingTokenDisplay with (total=180_000,
   cache_read=150_000, cache_creation=0) per the post-fix seed pattern; assert the
   first emitted TokenInfo.input_tokens == 180_000, not 330_000.
2. Flush integrity test: in the no-usage-event-this-turn state, after
   flush_partial_state_before_compaction, assert tracker.input_tokens equals the
   true total (180k) and cumulative_billed_input did not absorb cache tokens.
3. Auto/manual parity test: for the same session state, AUTO-path and MANUAL-path
   snapshots of pre_compaction_tokens must be equal.
4. Overflow-window snapshot test: simulate seed-then-CompactionStarted (recovery
   path); assert pre_compaction_tokens is 180k, not 330k.

## 7. Files Expected to Change

- codelet/cli/src/interactive/stream_loop.rs (all seed/re-seed sites)
- codelet/cli/src/interactive/recovery_compaction.rs (verify flush integrity after root fix)
- codelet/agent-loop/src/background_output.rs + codelet/napi/src/agent_loop.rs (AUTO snapshot writers)
- possibly codelet/core/src/session/background_session.rs (single snapshot accessor)
- codelet/agent-loop/tests/rpc086_token_tracking.rs (update locked-in wiring assertion if wiring changes)
- New tests in codelet/cli/tests/ and/or codelet/agent-loop/tests/, codelet/napi/tests/

## 8. Out of Scope
- Negative ratio clamping → CMPCT-039 (done).
- Header abs() masking → CMPCT-040.
- /compact premature RPC measurement → RPC-421.
- Normalizing the compacted side's estimate basis vs API basis (mixed-basis ratio):
  document rather than change here; do NOT attempt an estimator rewrite.
