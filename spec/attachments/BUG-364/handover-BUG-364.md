# BUG-364 handover (2026-11-03)

Status: backlog, not started. Research done while closing BUG-377:

- BUG-377 (af474b8ab) deliberately left the exec-stdin probe to this card.
- The discarded `action_tx.send` results in `app/dispatch_exec_stdin.rs` are already fixed: TUI-117 routes every send
  through `dispatch_exec_stdin_recovery::send_to_app`, which logs a closed app loop. Only the probe failure remains.
- Two probe sites drop a failed `get_exec_stdin_request` at `debug` as "silently dropped":
  `app/dispatch_exec_stdin.rs::probe_exec_stdin_for` (focus switch) and the Paused parallel probe in
  `app/dispatch_pause_hitl.rs` (its pause/HITL reads and `let _ = action_tx.send` sends are BUG-394).
- `Handle::try_current().is_err() => return` in `probe_exec_stdin_for`, `handle_exec_stdin_submit` and the control
  path also drops work silently when no runtime exists; decide here or file it.

Plan:
1. Feature: add scenarios to `spec/features/exec-stdin-probe-freshness.feature` (first tag @BUG-209 owns
   `tests/exec_stdin_probe_freshness_bug209.rs`; add @BUG-364 after it). E.g. "A failed probe marks an open notice as
   possibly stale" and "A failed probe with no open notice is logged".
2. New `Action::ExecStdinProbeFailed { agent_session, generation, error }` appended after `ExecStdinMuted` in
   `components/action_board_card.rs`; add the name to `tests/refac019/appended.rs` APPROVED_APPENDED_TAILS; run
   `refac019_contracts` and `board024_additive_api_contract` before committing.
3. Reducer (in `dispatch_exec_stdin_recovery.rs`): when the generation is current, `tracing::error!` with the session
   and, if a notice is open, record a per-session probe error in `store/agent_view/exec_stdin_state.rs` that the
   notice row paints (cleared by the next fetch/set/clear). `views/agent/input_area.rs` is 284 lines; keep the paint
   change in `stdin_answer_box.rs`.
4. Remap coverage for every file edited (scratchpad remap.py approach: difflib old->new, link-coverage per impl
   mapping; edit test ranges in the JSON because link-coverage adds a second test mapping).

Outcome: done as planned. The silent no-runtime returns (`probe_exec_stdin_for`, `handle_exec_stdin_submit` and the
pause/HITL writers) were filed as BUG-395 together with the pause/HITL write failures logged at debug.
