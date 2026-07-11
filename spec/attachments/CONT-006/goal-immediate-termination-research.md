# Research: /goal Immediate Termination & Atomic Teardown (VERIFIED 2026-07-10)

## Verdict

**Yes — /goal suffers the same delayed-termination problem as auto-continue done()
(CONT-005), and worse.** Goal mode shares the single FinalResponse settle point but adds
goal-specific state (Tier-2 verify execution, rejection counters, CompletionContract
reminder, the session goal itself) that all remain live during the gap between an accepted
`done()` ToolResult and `FinalResponse`.

## 1. Goal-mode acceptance flow — `codelet/tools/src/done.rs`

`DoneTool::call()` (done.rs:315-363):

| Step | Lines | Behavior |
|---|---|---|
| Tier 0 (empty summary) | 317-323 | `Err(Validation)`, no rejection recorded |
| Disarmed stale call | 327-329 | inert Ok, no acceptance |
| Goal gate | 332 | `if let Some(goal) = get_session_goal(...)` |
| Tier 1 | 334-344 | `tier1_ok` (209-219: ≥1 non-empty evidence AND assessment ≥20 chars, MIN at :49); fail → `record_rejection` (:335) + `Err(Validation)` |
| Tier 2 | 346-354 | `run_verify` (224-263; `sh -c` at CWD, 300s timeout :43); fail/timeout → `record_rejection` (:348) + `Err(Validation)` with exit code + 4KB tail |
| **Accepted** | 357-361 | `DONE_ACCEPTANCE.insert(session_id, summary)` |
| Return | 362 | `"Completion recorded. The turn will finish with your summary."` |

Registry facts:
- `record_rejection` (done.rs:145-150) increments `CONTRACT_STATE.rejections`.
- **Rejections are NOT reset at acceptance time** — only via `set_session_goal`
  (done.rs:110-116, `state.rejections = 0`), called at settle/dispatch.
- `take_done_acceptance` (162-167) is read-and-clear; `DONE_ACCEPTANCE.insert` (:360) is a
  plain HashMap insert → **last-writer-wins overwrite** on a second done() before settle.

## 2. All goal teardown is at the FinalResponse settle point

`codelet/cli/src/interactive/stream_loop.rs`:
- :1458 `take_done_acceptance` (single consumption site codebase-wide)
- :1483-1497 `goal_active = session.goal.is_some()`; sync
  `session.done_rejections = done_rejection_count(...)` (:1487-1488); `decide_goal_continuation` (:1489-1497)
- :1510-1526 `FinishWithSummary`:

```rust
if goal_active {
    let announcement = super::goal::apply_goal_acceptance(session, &summary);  // :1517
    output.emit_status(&announcement);                                          // :1518
    codelet_tools::set_session_goal(session_id, None);                          // :1519
} else {
    output.emit_status(&format!("✓ done: {summary}"));                          // :1523
}
session.continue_nudges_used = 0;                                               // :1525
```

- `apply_goal_acceptance` — `goal.rs:81-84`: `session.clear_goal(); "🎯 goal satisfied: {summary}"`
- `Session::clear_goal` — `cli/src/session/mod.rs:235-242`: `goal = None`,
  `done_rejections = 0`, `remove_system_reminders_of_type(CompletionContract)`
  (mod.rs:238-241; fn at session/system_reminders.rs:366)
- Break at :1613-1614.

**No early-exit check exists in the ToolCall (:903-941) or ToolResult (:965+) arms.**
NAPI/agent-loop twins delegate to the same shared loop (napi agent_loop.rs:125/:1064/:1188 →
stream_loop.rs:226) — one fix covers all surfaces (napi arming gap fixed by CONT-009).

## 3. Gap hazards specific to Goal mode

1. **Model keeps working after "goal satisfied".** Announcement prints only after the model
   finally stops; post-acceptance edits happen while the goal is nominally met.
2. **Verify-state staleness.** Tier 2 passed at tool-call time; anything the model does in
   the gap (more Bash/Edit) can invalidate the verified state. The stale summary is still
   surfaced at settle with no re-verification.
3. **Second done() in the gap.** Goal still in `CONTRACT_STATE` (cleared only at :1519),
   DoneTool still registered → full pipeline re-runs, **verify command executes again**
   (side effects, up to 300s). Second acceptance silently overwrites the first summary;
   second *rejection* leaves the first acceptance intact.
4. **Acceptance masks due escalation.** `decide_goal_continuation` checks `done_summary`
   first (auto_continue.rs:113-117) before `done_rejections >= 4` (auto_continue.rs:124-126)
   and the stall fast-path (:129-134). An accepted-then-rejected sequence finishes with the
   stale first summary and swallows a pending human-review escalation.
5. **CompletionContract reminder stays injected during the gap** (removed only by
   `clear_goal` at settle; text at session/mod.rs:252-257) — actively encouraging redundant
   work / a second done().
6. **Unconsumed acceptance lingering:** an acceptance recorded but never consumed (error/
   interrupt exit before clean FinalResponse) is only cleaned by
   `set_continue_armed(false)` → `clear_done_acceptance` (agent_runner.rs:110-111, CLI
   per-message UUID); on the long-lived agent-loop session id it lingers until next
   dispatch's disarm/re-arm.

## 4. Escalation is also settle-point-only

- `ContinueDecision::Escalate` handled at stream_loop.rs:1533-1545 → message first, then
  `raise_goal_escalation` (goal.rs:66-76) → `pause_for_user` (tool_pause.rs:81-86; no
  handler in plain repl → immediate `Resumed`, hence message-first ordering).
- The ≥4 threshold is recorded at tool time (done.rs:335/:348) but *evaluated* only at
  settle — a model can accumulate arbitrarily many rejections + verify runs mid-turn with
  no intervention until it stops.

## 5. Required work (distinct from CONT-005)

An early-exit path in Goal mode must atomically perform, at the ToolResult arm
(stream_loop.rs:965-1030, after `handle_tool_result` :968):

| Teardown step | Current location |
|---|---|
| Announce `🎯 goal satisfied: <summary>` (NOT `✓ done:`) | stream_loop.rs:1516-1518 via `apply_goal_acceptance` (goal.rs:81-84) |
| Clear `session.goal` + reset `session.done_rejections` | `Session::clear_goal` (session/mod.rs:235-242) |
| Remove CompletionContract reminder | mod.rs:238-241 → system_reminders.rs:366 |
| Clear registry goal + registry rejections | `set_session_goal(session_id, None)` (stream_loop.rs:1519 → done.rs:110-116) |
| Reset nudge budget | `session.continue_nudges_used = 0` (:1525) |
| Prevent post-acceptance work / repeat verify runs | no current guard — only immediate termination fixes this |
| `emit_done_with_stop_reason` + break | :1613-1614 |

**Design requirement:** factor the goal-acceptance teardown (:1511-1525) into a helper
shared by the early-exit site and the FinalResponse fallback so announce/clear/reset/
reminder-removal can never diverge. `apply_goal_acceptance` (goal.rs:81-84) is the natural
seed for that helper; it must additionally take on `set_session_goal(None)`.

Also worth fixing under this card (or explicitly deferring):
- DONE_ACCEPTANCE overwrite semantics (guard against second acceptance pre-settle — moot
  if immediate termination lands, since DoneTool can't be called again).
- Acceptance-vs-escalation ordering in `decide_goal_continuation` (auto_continue.rs:113 vs
  :124) — moot for the early-exit path, still relevant for the fallback path.

## Dependencies

- **Depends on CONT-005** (shared early-exit mechanism at the ToolResult arm).
- **Related: CONT-008** (goal back-sync to chrome — the satisfied goal must also clear the
  TUI bar and BackgroundSession.goal_state or it gets resurrected on the next message).
- **Related: CONT-009** (napi arming gap — fixed; goal mode now armed on the production
  NAPI surface).

## Test Coverage Required

- Accepted goal done() → immediate break; goal cleared, reminder removed, rejections reset
  (session + registry), announcement is `🎯 goal satisfied:`.
- Rejected done() (Tier 1 / Tier 2 / timeout) → NO early exit; loop continues; rejection
  counted.
- Verify command runs at most once per accepted completion (no double execution).
- Escalation at settle unaffected when no acceptance pending.
- FinalResponse fallback still performs identical teardown (shared helper).
- Post-early-exit `session.messages` contains the paired done tool_use/tool_result and no
  CompletionContract reminder.
