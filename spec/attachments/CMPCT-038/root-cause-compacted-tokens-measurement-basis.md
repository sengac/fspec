# CMPCT-038 — CompactionComplete overstates reduction: measurement-basis dossier

**Date:** 2026-07-08 (restored after add-attachment self-copy truncation)
**Author:** Supervisor session (first-principles DeepSearch trace of the embedded compaction flow)
**Symptom:** Even with the RPC-420 display fix in place, the COMPACTED badge reports ~98-99%
reduction when the user's real context reduction is ~60%. The user's original report ("shows ~9800%,
should be ~60%") decomposes into RPC-420 (×100 sign-flipped display of 99.0) **and this bug**
(99% reported where ~60% is true).

---

## 1. The event that feeds the badge

For one compaction in the embedded (DAG / hierarchical-compaction) flow, the TUI receives exactly
**one** `StreamChunk::CompactionComplete` (confirmed by trace):

- `compact_session` (`codelet/sessions/src/handle_impl.rs:261-341`) dispatches **no** chunk — it only
  returns a `CompactionResult` from the RPC (see RPC-421 for that value's own problem).
- The chunk is emitted by `emit_post_injection_events`
  (`codelet/agent-loop/src/inject_summary_handler.rs:85-103`), wired via the `on_injected` callback
  registered in `codelet/agent-loop/src/agent_loop.rs:686-710`, when the agent calls the
  `inject_summary` tool with the DAG summary.
- `codelet/agent-loop/src/background_output.rs:295-309` and `codelet/napi/src/agent_loop.rs:1712-1715`
  are dead fallback arms in this flow (comment at background_output.rs:296-297; zero production
  callers of `emit_compaction_complete`).

## 2. The defect — `compacted_tokens` counts the DAG summary text only

`codelet/agent-loop/src/inject_summary_handler.rs`:

- Line ~53: `injected_tokens = count_tokens(&wrapped)` — token count of the **wrapped DAG summary
  string alone**.
- Lines 92-99:
  ```rust
  let ratio = compression_ratio(original_tokens as u64, injected_tokens as u64) * 100.0;
  ...
  emit(StreamChunk::compaction_complete(CompactionResult {
      original_tokens,
      compacted_tokens: injected_tokens,   // ← summary-only
      compression_ratio: ratio,
  ```

But the **real** post-compaction context contains more than the summary: the system reminders
(large in fspec — workflow reminder, CLAUDE.md, environment, etc.) are re-injected, plus whatever
else `inject_summary` leaves in `session.messages`. The honest "compacted" size is the recalculated
token tracker AFTER the message list is rebuilt (`recalculate_token_tracker`,
`codelet/cli/src/interactive_helpers.rs:194-205`, sets `token_tracker.input_tokens` to the sum over
all remaining messages).

### Worked example (matches the user's report)

- `original_tokens` (pre-compaction) = 100 000
- DAG summary (`wrapped`) = 1 000 tokens → reported reduction = (1 − 1000/100000) × 100 = **99%**
- Real post-injection context = summary 1 000 + system reminders ~35 000 + residual ≈ 40 000 tokens
  → true reduction = (1 − 40000/100000) × 100 = **60%**

Under RPC-420's (now fixed) broken display this rendered as `.abs((1−99)×100)` = **9800%** — the
exact number observed. Post-RPC-420 it would display 99% — correctly-displayed garbage input.

## 3. Both twins are affected (keep them in lock-step)

| Twin | File | Consumer |
|---|---|---|
| Rust engine | `codelet/agent-loop/src/inject_summary_handler.rs:85-103` (+ `on_injected` wiring in `agent_loop.rs:686-710`) | Rust fspec-tui (embedded + WebSocket) |
| NAPI | `codelet/napi/src/inject_summary_handler.rs:~95-115` | TS Ink TUI (still operational per RPC-002 Q10) |

The NAPI twin uses the same `injected_tokens`-only basis and MUST receive the identical fix so the
two frontends report the same number for the same compaction.

## 4. Required fix

1. **Change the measurement basis** for the `CompactionComplete` chunk in both twins:
   `compacted_tokens` = the session's recalculated post-injection total
   (`session.token_tracker.input_tokens` after `inject_summary` has rebuilt messages + reminders and
   run `recalculate_token_tracker`), NOT `count_tokens(&wrapped)`.
   - The `on_injected` callback signature likely needs to carry the post-injection total (or the
     handler reads the tracker directly at emit time). Choose the design that keeps the emit site
     race-free (the tracker must be recalculated BEFORE the chunk is emitted).
2. **`original_tokens` basis is correct and unchanged** (`pre_compaction_tokens` stored at
   `handle_impl.rs:296-298` / NAPI equivalent).
3. `compression_ratio` stays `compression_ratio(original, compacted) * 100.0` — percent removed on
   the corrected basis (contract per RPC-420; documented on rpc-types).
4. **Degenerate cases**: if post-injection total ≥ original (tiny sessions where reminders dominate),
   the helper yields ≤ 0 — clamp the shipped ratio to ≥ 0.0 (never negative on the wire);
   `original == 0` already yields 0.0 via the helper guard.
5. **Tests** (Rust): unit/integration tests in `agent-loop` (and NAPI twin) proving:
   - chunk `compacted_tokens` equals the recalculated tracker total, not the summary size;
   - a scenario with reminders (e.g. orig 100 000, summary 1 000, reminders 39 000) reports 60%
     removed, not 99%;
   - clamp-at-zero for post-injection ≥ original;
   - both twins produce identical `CompactionResult` values for identical inputs.

## 5. What this bug is NOT

- NOT the display inversion (RPC-420; fixed in fspec-tui).
- NOT the premature `/compact` RPC-result value (RPC-421; `handle_impl.rs:310` measures before
  injection — a different producer with a different problem).

## 6. Verification commands

```bash
cd codelet && cargo test -p codelet-agent-loop
cargo test -p codelet-napi        # if NAPI tests build in this environment
cargo check --workspace
```
