# TUI-116 handover (2026-11-02)

TUI-116 has not been started. It is still in **backlog** and no feature or test exists yet. Its backend prerequisite is done:
- **TOOL-027 (91724ac83)**: `FspecBackend::control_exec_stdin(id, exec_session_id, ExecStdinControl::{Interrupt, Eof}) -> anyhow::Result<()>` exists end to end (RPC index 130). A success clears the session's exec-stdin request. In tests, `MockBackend::exec_control()` returns an `ExecControlMock`: `calls()` returns `(SessionId, exec id, control)` and `fail_with(msg)` scripts a failure (`tests/common/exec_control_mock.rs`). TUI-116 depends on TOOL-027.
- REFAC-043 already moved the BUG-171 push test helpers into `tests/exec_stdin_push_chunk_bug171_support/`. That root file (274 lines) can be edited.

## Plan worked out (not yet specified)
- **New feature** `spec/features/stdin-notice-and-answer-box.feature` (@TUI-116 @tui @agent-view @tool-execution), with its own test file `rust/fspec-tui/tests/tui116_stdin_notice_answer_box.rs` and helpers in `tests/tui116_stdin_notice_answer_box_support/`. Write it without clippy allows: use `TestResult` and `?`, no expect/unwrap/panic.
- **Scenarios**:
  - Notice row above the composer: `<cmd> may be waiting for input · Ctrl+O answer · Ctrl+X dismiss`. The chat draft is still shown and typing still edits it.
  - Ctrl+O opens a separate answer box with its own buffer. Its hint row reads e.g. `Enter send · Esc chat · Ctrl+C interrupt · Ctrl+D end input`.
  - Enter writes the box line (`write_exec_stdin`) and sends no chat message.
  - Esc returns to the chat draft, keeps the notice, and keeps the box text.
  - Ctrl+C sends `control_exec_stdin(Interrupt)` and no session interrupt. Ctrl+D sends `Eof`.
  - With chat focused: Esc interrupts the turn, and Ctrl+X dismisses the notice without touching the command.
  - Scenarios moved from `exec-stdin-prompt.feature`: HITL wins over the notice, ghost panes show no notice, and a re-probe with no request closes the notice and box.
- **Superseded spec and tests**: delete the seven TUI-layer scenarios from `spec/features/exec-stdin-prompt.feature` (render in composer slot, HITL precedence, Enter submits, Esc dismisses, no prompt while none waiting, slot cleared when exec session gone, ghost panes). Then delete `rust/fspec-tui/tests/exec_stdin_prompt_p2.rs` (705 lines, TUI layer only). Its unmapped tests (HITL swallows a fresh fetch, failed submit keeps slot) belong to the new feature and TUI-117. This avoids editing a 705-line file.
- `exec-stdin-push-tui-composer.feature` (BUG-171): its tests assert `⌨ git commit` and press Enter on the shared input. Update them to the notice text, open the box with Ctrl+O first, and reword the steps that say "composer overlay".
- **Code**:
  - `views/agent/input_area.rs` (260 lines): paint the notice row on top of the normal and busy paths and add 1 to `last_hitl_input_offset`. When the box is focused, paint header, box and hint.
  - `views/agent/exec_stdin_keys.rs`: unfocused, only Ctrl+O and Ctrl+X are consumed. Focused, the keys route to the box input.
  - The stdin buffer is a separate `MultiLineInput`, kept in a small sub-struct on `AgentView`. `views/agent.rs` is 293 lines, so add at most one field and route `cursor_position` to the box input when it is focused.
  - New Action, e.g. `ExecStdinControl { agent_session, exec_session, control }`, plus a reducer in `app/dispatch_exec_stdin.rs`.
- **Contract snapshot**: a new Action variant appends to `refac019_actions`. Add its name to `tests/refac019/appended.rs` `APPROVED_APPENDED_TAILS` (see BUG-370) and run `refac019_contracts` and `board024_additive_api_contract` in the same batch.
- **Regression set for this area**: exec_stdin_push_chunk_bug171, exec_stdin_probe_freshness_bug209, rig016_steer_queue_keys, the tui115 test, inline_hitl_prompt_rpc411, refac019_contracts, board024_additive_api_contract.

## Related filed cards
BUG-362 (stdin writer drops io errors), BUG-364 (TUI probe/send errors dropped), BUG-369 (exec-stdin lock poisoning, default write fakes success). TUI-117's red error depends on BUG-362 to report real write failures.
