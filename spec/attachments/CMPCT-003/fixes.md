# CMPCT-003: Post-Review Fixes

**Date:** 2026-03-10
**Trigger:** Self-review of completed implementation against feature file and example map
**Status:** All fixes applied and verified

---

## ✅ Fix 1: `compaction_in_progress` flag never cleared on failure path

**File:** `codelet/napi/src/session_manager.rs` (agent_loop, ~line 5674)

**Problem:** The `compaction_in_progress` flag is set to `true` in `execute_compaction()` and only cleared inside the `inject_summary` handler closure. If the agent errors out, gets interrupted, or simply doesn't call `inject_summary`, the flag stays `true` permanently. All subsequent `SessionSearch` calls apply Layer 0 trimming indefinitely, corrupting search results.

**Fix applied:** Added unconditional `session.compaction_in_progress.store(false, Ordering::SeqCst)` after the `apply_pending_dag` block, before handler cleanup. Runs on every stream completion regardless of outcome.

---

## ✅ Fix 2: Stale "CompactionComplete already signals state change" comments

**File:** `codelet/cli/src/interactive/stream_loop.rs` (lines ~1838, ~1850)

**Problem:** Comments referenced old behavior where `CompactionComplete` was emitted from stream_loop. Now `CompactionComplete` is emitted from agent_loop after retry stream completes. The `emit_done()` calls are standard Done events.

**Fix applied:** Updated both comments to: `CMPCT-003: Emit Done normally. CompactionComplete is emitted by agent_loop after apply_pending_dag succeeds (not from stream_loop).`

---

## ✅ Fix 3: Meaningless metrics in agent_loop CompactionComplete emission

**File:** `codelet/napi/src/session_manager.rs` (agent_loop ~line 5658, BackgroundOutput ~line 6063, session_compact ~line 7383)

**Problem:** `original_tokens: 0` and `compression_ratio: 100.0` were hardcoded in the agent_loop's CompactionComplete emission. The original token count was known before compaction but wasn't tracked through to the agent_loop.

**Fix applied:** Added `pre_compaction_tokens: AtomicU32` field to `BackgroundSession`. Snapshot is written at two entry points:
- `CompactionStarted` handler in `BackgroundOutput::emit` — reads `cached_input_tokens`
- `session_compact` — reads `inner.token_tracker.input_tokens`

Agent_loop now reads `pre_compaction_tokens` and computes actual compression ratio.

---

## ✅ Fix 4: Dead code in BackgroundOutput::emit for CompactionComplete

**File:** `codelet/napi/src/session_manager.rs` (~line 6072)

**Problem:** The `StreamEvent::CompactionComplete` arm in `BackgroundOutput::emit` sets status to Idle, clears progress, and emits SessionStateChange — but stream_loop no longer emits this event. Dead code that could mislead future developers.

**Fix applied:** Added clarifying comment: `CMPCT-003 NOTE: In the in-view DAG flow, CompactionComplete is emitted directly by the agent_loop via handle_output (not through BackgroundOutput). This handler is retained as a fallback for any future code path that emits StreamEvent::CompactionComplete through the standard StreamOutput trait.`
