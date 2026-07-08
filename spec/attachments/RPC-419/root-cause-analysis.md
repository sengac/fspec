# RPC-419 — Root Cause Analysis: Context-Fill Badge Oscillation

**Date:** 2026-07-08
**Author:** Claude Code (supervisor session, deep-search investigation)
**Affects:** Rust ratatui TUI (`codelet/fspec-tui`) AND TypeScript Ink TUI (`src/tui`)
**Introduced by:** RPC-101 (local realtime recompute) — the recompute *cadence* fix was correct; the *formula* it lifted was wrong.

---

## 1. Symptom

The SessionHeader `[X%]` context-fill badge fluctuates wildly **within a single turn**, dropping sharply the moment output/thinking tokens start streaming, then snapping back up. Example with real numbers (`raw input=20k, cache_read=150k, cache_creation=5k, output=3k, reasoning=8k, threshold=168k`):

- Backend authoritative value: `186,000 / 168,000` → **110%**
- Frontend local recompute: wire input `175,000` − `0.9×150,000` = `40,000 / 168,000` → **24%**

The badge alternates ~110% ↔ ~24% as chunks interleave. Both frontends have the identical defect; the Rust TUI is where it was noticed.

## 2. The Two Formulas

| Source | Formula | What it measures |
|---|---|---|
| **Backend** `ContextFillUpdate.fill_percentage` — `emit_context_fill_from_usage` (`codelet/cli/src/interactive/stream_loop.rs:119-137`) | `trunc(ApiTokenUsage::total_context() / threshold × 100)` where `total_context = input + cache_read + cache_creation + output + reasoning` (`codelet/core/src/token_usage.rs:64-72`) | **Physical context occupancy** — correct for a fill gauge |
| **Frontend local recompute** on every `TokenUpdate` (Rust `codelet/fspec-tui/src/store/agent_view/token_state.rs:88-100`; TS `src/tui/components/AgentView.tsx:1142-1161`) | `round((wire_input − floor(0.9 × cache_read)) / threshold × 100)` | **Cache-discounted billing/cost proxy** — the *compaction trigger* metric (`TokenTracker::effective_tokens`, `codelet/core/src/compaction/model.rs:85-96`), NOT context fill |

## 3. Why It Fires Exactly When Output Tokens Arrive

Backend emission cadence (verified in `stream_loop.rs`):

| Site | Line(s) | Trigger | Emits |
|---|---|---|---|
| A | 604-608 | Start of turn | TokenUpdate **+** ContextFillUpdate |
| B | 882 | Text deltas (throttled) | TokenUpdate **ALONE** |
| C | 940 | Reasoning deltas (throttled) | TokenUpdate **ALONE** |
| D | 1014-1023 | Usage/MessageDelta (`output>0`) | TokenUpdate **+** ContextFillUpdate |
| E | 1072-1081 | FinalResponse | TokenUpdate **+** ContextFillUpdate |

A `ContextFillUpdate` lands the correct occupancy percentage → then bare `TokenUpdate`s from Sites B/C trigger the frontend recompute, which **overwrites the correct value** with the cache-discounted, output-and-reasoning-excluding number → the next MessageDelta (Site D) snaps it back up. Sawtooth.

## 4. The Three Compounding Defects

### Defect 1 — Wrong formula lifted into RPC-101
The `input − 0.9·cache_read` discount belongs to the compaction subsystem's `TokenTracker::effective_tokens()` — a cost proxy deciding *when to compact*. The badge's authoritative source uses `total_context()` with **no discount** and **includes output + reasoning**. RPC-101's rule [2] explicitly mandated the wrong formula; the Rust port then faithfully mirrored the TS defect. The code comments in both frontends falsely claim the recompute "mirrors `emit_context_fill_from_usage`" — it does not.

### Defect 2 — Wire semantics of `input_tokens` double-dip the cache discount
On the wire, `TokenTracker.input_tokens` is **`total_input()`** = raw + cache_read + cache_creation (`codelet/cli/src/interactive/output.rs:44` and `:61` — "Display total, not raw (PROV-001)"), while `cache_read_input_tokens` is *also* carried alongside. The frontend subtracts 90% of cache_read from a value that already contains 100% of it. Even as a cost proxy the arithmetic is against the wrong base.

### Defect 3 — Rounding mismatch
Backend computes `(total / threshold * 100.0) as u32` → **truncation**. Both frontends use `round()` → even with the correct formula, local and authoritative values can disagree by 1% and flicker.

## 5. The Fix (this card)

**Formula change only — all RPC-101 cadence/caching machinery stays.**

In both `token_state.rs::apply_token_tracker` (Rust) and `AgentView.tsx::updateTokenStateFromChunk` (TS):

```
effective = input_tokens + output_tokens + reasoning_tokens
            // input_tokens on the wire ALREADY includes cache_read + cache_creation (PROV-001)
pct       = trunc(effective / threshold * 100)   // truncation, matching backend `as u32`
```

- **Remove** the `0.9 × cache_read` discount entirely from the badge recompute.
- **Replace** `round` with truncation.
- Missing optional fields (`reasoning_tokens`, cache counters) are treated as 0.
- Fix the false comments claiming the old formula mirrored `emit_context_fill_from_usage`.

### Invariants that MUST be preserved (from RPC-101/RPC-100/RPC-099)
1. Threshold cached from every `ContextFillUpdate`; non-positive/non-finite threshold never wipes a cached good value.
2. Recompute skipped when threshold == 0 (no divide-by-zero; badge holds last value).
3. Authoritative `ContextFillUpdate` always overwrites the local value.
4. Overshoot > 100% preserved (never clamp to 100) — RPC-100 pre-compaction signal.
5. Per-session isolation (RPC-099 per-SessionId `TokenState`).
6. Restore paths (resume / Shift+Left/Right) still seed the threshold cache.

### Convergence property (the actual acceptance test of the fix)
A bare `TokenUpdate` carrying the same usage numbers as the preceding `ContextFillUpdate` must recompute to **exactly the same percentage** — no sawtooth, no ±1 flicker.

## 6. Files to Change

| File | Change |
|---|---|
| `codelet/fspec-tui/src/store/agent_view/token_state.rs` | Fix formula in `apply_token_tracker` (lines ~73-100) + comments |
| `src/tui/components/AgentView.tsx` | Fix formula in `updateTokenStateFromChunk` (lines ~1142-1161) + comments |
| `spec/features/context-fill-percentage-realtime-recompute.feature` | Replace cache-discount scenario with occupancy-formula + convergence scenarios; fix architecture doc string |
| `spec/features/context-fill-percentage-realtime-recompute-ui.feature` | Same (UI-level) |
| `spec/features/context-fill-percentage-realtime-recompute-restore.feature` | Doc string only if it references the discount |
| `codelet/fspec-tui/tests/token_state_realtime_recompute_rpc101.rs` | Update expected values; add convergence + truncation + output/reasoning-inclusion tests |
| `src/tui/utils/__tests__/tokenStateUtils.test.ts` | No change needed — verified during testing phase: its RPC-101 section covers extractTokenStateFromChunks threshold surfacing only; no assertion encodes the 0.9 discount or rounding. |
| `src/tui/__tests__/context-window-fill-percentage.test.tsx` | Update the 4 RPC-101 integration tests (lines ~536-671) |

## 7. Explicitly OUT of scope (noted for future cards)

- **OpenAI-family provider `Usage` semantics** (possible double-counting of cached tokens in `total_input()` for providers whose `input_tokens` already includes cache reads). Unverified; backend/provider domain (PROV). Not touched here.
- `calculateContextFillPercentage` (`src/tui/utils/tokenStateUtils.ts:127-141`) — used only on the **persisted-session restore path** with `currentContextTokens`; it is a plain `input/threshold` with no discount and is not part of the live oscillation. Left as-is.
- Backend emission cadence (Sites B/C emitting bare TokenUpdates) — correct behavior once the frontend formula matches; no backend change.
- Compaction's `TokenTracker::effective_tokens` — correct for its own purpose (compaction trigger); untouched.
