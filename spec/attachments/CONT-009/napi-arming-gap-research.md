# Research: NAPI agent_loop Missing CONT-002/003 Arming Sync (VERIFIED 2026-07-10)

## Problem

`codelet/napi/src/agent_loop.rs` — the PRODUCTION dispatch path for the NAPI/TUI surface —
contains **zero** CONT-002/CONT-003 wiring. Grep for
`goal|continue_|set_continue_armed|set_session_goal|reset_for_new_user_turn` in that file
returns no matches.

Consequence chain:
1. `codelet_tools::set_continue_armed(session_id, …)` is never called on this surface →
   `is_continue_armed(session_id)` stays false.
2. `DoneTool` is registered conditionally on `is_continue_armed` in the create_rig_agent
   builder chains (CONT-002 rule 8) → **done() is never registered**.
3. `/continue` and `/goal` set chrome state (BackgroundSession atomics / goal Mutex) and the
   TUI bar indicator, but the engine never nudges, never enforces the goal, never
   accepts done() — **auto-continue and Goal mode are silently inert** on the production
   NAPI surface. The UI lies (indicator shows armed).

## Verified reference implementation (the extracted twin)

`codelet/agent-loop/src/agent_loop.rs:495-530` performs, per dispatched user message:
- read chrome continue state; `session.reset_for_new_user_turn()` (:503)
- chrome-goal → inner-session goal sync (`inner_session.set_goal/clear_goal`, :504-518)
- registry sync (:529-530):
  ```rust
  codelet_tools::set_continue_armed(session.id, continue_enabled || goal_spec.is_some());
  codelet_tools::set_session_goal(session.id, goal_spec);
  ```
- Pause-handler registration nearby (agent-loop agent_loop.rs:538-555; napi twin HAS the
  pause handler at napi agent_loop.rs:607-625 — that part is present).

CLI repl equivalent: `agent_runner.rs:49-50` (arm + goal sync before create_rig_agent),
teardown/disarm at `agent_runner.rs:110-111`.

## Why this diverged

`codelet-agent-loop` is the extracted twin of the napi loop, but it is a **dev-only**
dependency of codelet-napi (`codelet/napi/Cargo.toml:135-136`: "production codelet-napi
must NOT depend on codelet-agent-loop"). It is a production dependency only of the
standalone `codelet/fspec` binary. CONT-002/003 wiring was applied to the twin
(agent-loop) and to the CLI repl (agent_runner.rs) but never ported to the napi original.

## Fix

Port the sync block from `codelet/agent-loop/src/agent_loop.rs:495-530` into
`codelet/napi/src/agent_loop.rs` at the dispatch site (~:599-605, the "Re-acquire lock"
site between thinking-config and `BackgroundOutput` construction):
1. `reset_for_new_user_turn` on the inner session per real user message.
2. Chrome goal → inner session goal sync (respecting CONT-008's planned back-sync
   direction fix — do not blindly re-apply; coordinate with CONT-008).
3. `set_continue_armed(session.id, continue_enabled || goal_spec.is_some())`.
4. `set_session_goal(session.id, goal_spec)`.
5. Keep parity with any future changes via a shared helper if feasible (the twins are
   intentionally near-identical; consider extracting the sync block into codelet-cli or
   codelet-common so both call one function — the twin-divergence is exactly how this bug
   happened).

## Interaction with the other cards

- **Blocks CONT-005/CONT-006 usefulness on the NAPI surface** — immediate termination
  can't matter where done() is never registered. (The stream-loop fix itself is shared,
  so those cards are implementable/testable on CLI without this one.)
- **CONT-007/CONT-008** — counter/goal state pushes originate in the shared stream loop,
  but on the NAPI surface counters never move until arming works.
- Suggested dependency edges: CONT-009 relates-to CONT-007/008; CONT-006 depends on
  CONT-005; no hard blocks besides scheduling preference.

## Test Coverage Sketch

- NAPI dispatch with continue_enabled=true → `is_continue_armed(session.id)` true before
  create_rig_agent; DoneTool present in the agent ToolSet.
- NAPI dispatch with a chrome goal set → `get_session_goal(session.id)` returns it; done()
  description contains "The current goal is: …".
- `reset_for_new_user_turn` clears nudges_used per real user message on the NAPI surface.
- Twin-parity shape test: assert both dispatch sites call the shared sync helper
  (precedent: rpc082/083 source-shape tests).

## Implementation outcome (post-completion note)

Implemented as `BackgroundSession::sync_completion_contract_for_user_turn` in
`codelet/sessions/src/background_session.rs` (shared helper; codelet-sessions chosen over
codelet-cli/common because both twins already depend on it in production and it owns the
chrome state + inner Session). Both twins call it; twin-parity shape test in
`codelet/sessions/tests/cont009_completion_contract_sync.rs`.
