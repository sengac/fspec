# Research: Goal State Back-Sync to Chrome — Stale Bar + Satisfied-Goal Resurrection (VERIFIED 2026-07-10)

## Problem

Goal lifecycle transitions performed by the ENGINE (satisfied auto-clear, escalation) are
applied only to the inner CLI `Session` and the codelet_tools registry. Nothing writes back
to the chrome-visible `BackgroundSession.goal_state`, and nothing notifies the TUI store.
The sync is strictly one-way (chrome → inner) at message dispatch.

## Two consequences (both verified)

### 1. Stale bar indicator
- On acceptance the engine clears only the inner goal:
  `apply_goal_acceptance` → `session.clear_goal()` (`cli/src/interactive/goal.rs:81-84`,
  `cli/src/session/mod.rs:235-242`) plus the tools registry via
  `codelet_tools::set_session_goal(session_id, None)` (`stream_loop.rs:1519`).
- The TUI footer reads only its local cache (`chrome_state.rs:73-75 goal_state_for`),
  written solely by `/goal` dispatch (`dispatch_slash_goal.rs:50-51`).
- Result: the footer keeps showing `🎯 goal (0/N)` for a goal that was already satisfied.

### 2. Satisfied-goal RESURRECTION (server-side, not cosmetic)
On the NEXT dispatched user message, the chrome→inner sync re-applies the stale chrome goal:

```rust
// codelet/agent-loop/src/agent_loop.rs:508-518 (verified; logic now lives in the
// CONT-009 shared helper BackgroundSession::sync_completion_contract_for_user_turn)
// chrome_goal != inner_goal_pair → inner_session.set_goal(...)
```

Because `BackgroundSession.goal_state` still holds the satisfied goal, the comparison sees
a mismatch (inner is None) and **re-applies the already-satisfied goal to the inner
session** — reinstating Tier 1/2 enforcement, the CompletionContract reminder, and Goal
mode for a goal the model already completed. `/goal clear` from the user is the only
recovery.

## Current sync topology (verified)

| Direction | Where | What |
|---|---|---|
| chrome → inner | CONT-009 shared helper (sessions/src/background_session.rs, called from both twins at dispatch) | continue_enabled, budget, goal set/clear, `set_continue_armed`/`set_session_goal` registry sync, `reset_for_new_user_turn` |
| inner → chrome | **NONE** | engine-side transitions never propagate out |
| chrome → TUI | RPC getters get_continue_state/get_goal_state (sessions/handle_impl.rs:1352,1377 → background_session.rs:1191,1206) | plumbed end-to-end but **never called by the TUI** |
| TUI local | chrome_state.rs (:64-67, :79-92) | written only by slash dispatch (dispatch_slash_continue.rs:50-54, dispatch_slash_goal.rs:50-51) |

`BackgroundSession` goal storage: background_session.rs:397-406 — `(text, verify)` pair
(Mutex), plus `(enabled, budget)` atomics for continue. **No nudge counter, no rejection
counter exported.**

## What must change

1. **Inner → chrome write-back** on engine-side transitions:
   - Goal satisfied (stream_loop.rs:1517-1519): also clear `BackgroundSession` goal state.
   - Escalation (stream_loop.rs:1533-1545): optionally mark state (goal stays active by
     design — goal.rs:65 — but the bar could reflect "blocked").
   - Mechanism choice: the stream loop has no direct handle to `BackgroundSession`; the
     natural channel is a pushed `StreamEvent`/`StreamChunk` state update (CONT-007's
     `ContinueStateUpdate` carrying `goal_active`/goal text) that BOTH `BackgroundOutput`
     (which CAN reach the owning session object — agent-loop/src/background_output.rs;
     napi mirror napi/src/agent_loop.rs:1627-1630) and the TUI consume. Alternative: a
     callback/registry similar to tool_pause handlers. Decide during specifying.
2. **Fix the resurrection comparison** (now in the CONT-009 shared helper): dispatch
   sync must not blindly re-apply chrome goal over an inner None caused by engine
   acceptance. Once write-back exists the mismatch disappears; a defensive
   generation/version stamp on goal state would make the sync direction unambiguous.
3. **TUI cache invalidation**: consume the pushed state chunk in
   session_context.rs (state-only arm :144-155) → `chrome_state::set_goal_state(None)` so
   the bar clears the `🎯` indicator the moment the goal is satisfied.
4. **/goal show correctness**: TUI-local `goal_parser.rs:85-141` hard-codes
   "nudges used: 0, rejections: 0" (:88, :98). Bare `/goal` state display should be driven
   by real state (pushed chunk or RPC getter), not the local mirror parser. CLI mirror
   (`cli/src/interactive/goal.rs:132-201`) reads real session state and is fine.

## Boundary with other cards

- **CONT-007** owns the new push chunk + live `nudges_used` in the bar. CONT-008 owns the
  chrome/server-side goal truth: BackgroundSession write-back, resurrection fix, TUI goal
  cache clear on engine transitions. They share the transport variant — implement the
  chunk once (dependency or shared first card).
- **CONT-006** (goal immediate termination) changes WHEN acceptance teardown runs, not
  WHAT syncs — the write-back must hang off the shared teardown helper so early-exit and
  settle-point paths both propagate.
- **CONT-009** — fixed; the arming/sync helper is the place to integrate the resurrection
  guard.

## Repro sketch (agent-loop twin surface)

1. `/goal make tests pass` → chrome + inner + registry all hold the goal; bar shows `🎯`.
2. Model completes, calls done() with evidence; verify passes → engine announces
   `🎯 goal satisfied`, inner goal cleared, registry cleared. Bar STILL shows `🎯 (0/N)`.
3. Send any new message → dispatch sync re-applies the chrome goal to inner →
   Goal mode re-armed for a completed goal; done() enforcement + reminder return.

## Test Coverage Sketch

- Engine acceptance clears BackgroundSession goal state (RPC getter returns None).
- Next-message dispatch does NOT re-apply a satisfied goal.
- TUI bar drops the `🎯` indicator on the acceptance state chunk.
- `/goal` (bare) after satisfaction reports "no goal set" on both surfaces.
- User-initiated `/goal clear` still works and syncs all three stores.
- Escalation leaves the goal active everywhere (by design) and the bar reflects it.
