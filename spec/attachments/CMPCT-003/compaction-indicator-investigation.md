# CMPCT-003: Compaction Indicator Investigation

**Date:** 2026-03-10  
**Trigger:** User observed "Thinking..." indicator disappearing when auto-compaction started, instead of transitioning to "Compacting..." indicator.

---

## Observed Behavior

When the context window fills up and auto-compaction triggers:
1. "Thinking..." indicator was showing (agent was processing)
2. Compaction triggered
3. **The indicator disappeared entirely** — no "Compacting..." shown
4. The input area returned to the normal idle placeholder (user could type)
5. The agent was still actively building the DAG (calling SessionSearch, inject_summary) but the UI showed idle state

## Root Cause Analysis

### Two Separate Bugs Compound

**Bug 1: Status gap between Done and CompactionStarted**

For post-loop compaction, the sequence is:
1. Agent's response stream finishes → `StreamEvent::Done` emitted
2. In `BackgroundOutput::emit()`: `Done` → `set_status(Idle)` → emits `SessionStateChange(Idle)`
3. Post-loop code detects `compaction_needed`
4. Emits `CompactionStarted` → `set_status(Compacting)` → emits `SessionStateChange(Compacting)`

Between steps 2 and 4, the JS side receives `SessionStateChange(Idle)` and `isLoading` becomes `false`. The `InputTransition` useEffect fires:
```
wasThinking=true, isThinking=false → starts hide animation → animationPhase='hiding'
```

When `SessionStateChange(Compacting)` arrives at step 4, the useEffect should catch it:
```
wasThinking=false, isThinking=true → setAnimationPhase('loading')
```

But this window is very brief.

**Bug 2: CompactionComplete fires before DAG construction**

This is the **critical bug**. The `execute_compaction()` function in `interactive_helpers.rs` is an **in-memory setup** phase only:
1. Sets `compaction_in_progress` flag
2. Clears messages, restores system reminders
3. Injects `COMPACTION_SYSTEM_INSTRUCTION` as a user message
4. Recalculates token tracker
5. Returns `Ok(())`

It does NOT make any LLM calls. It completes in milliseconds.

After `execute_compaction()` returns, the stream_loop emits:
- `CompactionComplete` → status becomes `Idle` (in `session_manager.rs` line 6038)
- `CompactionContinuing` → no-op for NAPI

Then the stream_loop either:
- **Pre-prompt path**: Continues into the main streaming loop with a new API call
- **Post-loop path**: Returns from `run_agent_stream_internal`

In **both** cases, the session status is `Idle` when the agent starts processing the compaction instruction. Nobody sets it back to `Running`.

### The Complete Broken Sequence

```
1. Agent running               → status=Running,    isLoading=true,  isCompacting=false  → "Thinking..."
2. Stream finishes (Done)      → status=Idle,        isLoading=false, isCompacting=false  → Hide animation starts
3. CompactionStarted           → status=Compacting,  isLoading=false, isCompacting=true   → "Compacting..." (maybe, very brief)
4. execute_compaction() <5ms   → (in-memory work)
5. CompactionComplete          → status=Idle,        isLoading=false, isCompacting=false  → Input placeholder (idle)
6. New LLM API call for DAG    → status=STILL IDLE!  isLoading=false, isCompacting=false  → Still idle!
7. Agent builds DAG via tools  → status=STILL IDLE!  isLoading=false, isCompacting=false  → Still idle!
8. inject_summary called       → status=STILL IDLE!
9. Stream finishes (Done)      → status=Idle (no change)
```

Steps 6-8 are the entire DAG construction phase where the agent is actively working but the UI shows idle.

## Key Code Locations

### Rust Side
- `codelet/napi/src/session_manager.rs:6024-6064` — `BackgroundOutput::emit()` handles `CompactionStarted/Complete/Continuing`
- `codelet/napi/src/session_manager.rs:6038` — `CompactionComplete` sets status to `Idle` ← **BUG**
- `codelet/napi/src/session_manager.rs:6061-6063` — `CompactionContinuing` is a no-op ← **Should set Running**
- `codelet/cli/src/interactive_helpers.rs:281-327` — `execute_compaction()` (in-memory only, no LLM)
- `codelet/cli/src/interactive/stream_loop.rs:500-531` — Pre-prompt compaction path
- `codelet/cli/src/interactive/stream_loop.rs:1605-1680` — Post-loop compaction path

### TypeScript Side
- `src/tui/components/AgentView.tsx:2517-2528` — `SessionStateChange(Compacting)` handler calls `compaction.startCompaction()`
- `src/tui/components/AgentView.tsx:2531-2537` — `CompactionComplete` handler calls `compaction.endCompaction()`
- `src/tui/components/InputTransition.tsx:185-211` — `isLoading || isCompacting` tracking
- `src/tui/hooks/useCompaction.ts:130-330` — Unified compaction state hook
- `src/tui/hooks/useRustSessionState.ts:147-150` — `isLoading = status === 'running'`, `isCompacting = status === 'compacting'`

## Proposed Fix

### Option A: Fix the event semantics (Recommended)

The in-view DAG flow has a different lifecycle than the old batch LLM compaction. The `CompactionComplete` event should only fire when the entire compaction cycle is truly done (after `inject_summary` applies the DAG), not after the setup phase.

1. **`CompactionComplete` should NOT be emitted from `stream_loop.rs`** after `execute_compaction()` returns. The compaction isn't complete yet — it's just beginning.
2. **`CompactionContinuing` should set status to `Running`** (or stay `Compacting` until DAG is applied).
3. **`CompactionComplete` should be emitted from `agent_loop`** after `apply_pending_dag()` succeeds.

### Option B: Keep Compacting status throughout DAG construction

Instead of `CompactionComplete` → `Idle` after setup, keep the status as `Compacting` throughout:
1. `CompactionStarted` → status = `Compacting`
2. `execute_compaction()` runs (setup)
3. **Don't emit CompactionComplete yet**
4. Agent processes compaction instruction (status stays `Compacting`)
5. Agent calls `inject_summary` → handler stores DAG, clears `compaction_in_progress`
6. Stream finishes → `agent_loop` applies pending DAG
7. **NOW emit CompactionComplete** → status = `Idle`

This requires the stream_loop to NOT emit `CompactionComplete/Continuing` for the in-view DAG flow, and instead let the agent_loop handle it.

### Option C: Minimal JS-side fix

Don't call `endCompaction()` on `CompactionComplete` if the agent is still running. Track whether the stream has truly ended. This is a workaround, not a proper fix.

## Relationship to CMPCT-003

This investigation confirms and expands the description in CMPCT-003. The original description identified the status going to `Idle` when it should be `Running`, but the full picture is more nuanced:
- It's not just about `CompactionComplete` setting `Idle` — it's that the entire in-view DAG construction phase has no status management
- The `CompactionComplete` event fires prematurely (after setup, not after actual completion)
- The `CompactionContinuing` event does nothing when it should signal "still working"
