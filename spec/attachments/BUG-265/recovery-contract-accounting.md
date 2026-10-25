# BUG-265: Completion-contract accounting across recovery

## Finding

Source-confirmed, pre-existing at `f0eb0759`. Synchronization for every dispatch
resets counters whose contract boundary is a real user turn or a goal change.
The new automatic recovery paths make this distinction especially important.

## Evidence

- `rust/agent-loop/src/agent_loop.rs:125` invokes synchronization unconditionally.
- `rust/agent-loop/src/agent_loop/input.rs:8` also produces synthetic retry inputs.
- `rust/sessions/src/background_session.rs:1711` resets nudges on every invocation.
- Line 1738 calls set_session_goal even when the goal is unchanged.
- `rust/tools/src/done.rs:115` resets registry rejections for every such call.
- `spec/attachments/CONT-002/design-auto-continue.md:47` defines real-user-turn
  budget boundaries; CONT-003 resets rejections only on goal set/replace/clear.

## Acceptance/test direction

Use real BackgroundSession/agent-loop fixtures to consume nudges and rejected
Done calls, then trigger automatic compaction or loop-abort retry. Counts remain
within the same task/goal contract. A genuine new user task resets its nudge
budget; an unchanged goal retains rejections until it changes or is cleared.
Verify four-rejection escalation can still be reached across recovery dispatch.

Separate arming/synchronization from reset operations. Preserve CONT-008 goal
generation and replacement-race protections, CONT-009 arming order and cancellation.
Ordinary in-stream nudge prompts already avoid the helper and must stay correct.
See [full audit](../CONT-011/fspec-auto-continue-investigation.md).
