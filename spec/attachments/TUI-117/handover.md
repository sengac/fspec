# TUI-117 handover (2026-11-03)

TUI-117 has not been started (still **backlog**, no feature or test). Its prerequisites are done:
- **TUI-116 (2bb45d04b)**: notice + separate answer box. `views/agent/stdin_answer_box.rs` holds
  `StdinAnswerBox { input: MultiLineInput, focused_session: Option<SessionId> }` as `AgentView::stdin_box`
  (one view-level box, focus keyed by session). Keys in `views/agent/exec_stdin_keys.rs`; painting in
  `views/agent/input_area.rs` (notice row via `paint_notice`, box via `AgentView::paint_stdin_box`). Enter clears the
  box and emits `Action::ExecStdinSubmit`; `app/dispatch_exec_stdin.rs::handle_exec_stdin_submit` clears the slot on
  success and only logs a failure. Ctrl+C/Ctrl+D emit the new `Action::ExecStdinControl` (appended REFAC-019 tail).
  Test harness: `rust/fspec-tui/tests/tui116_stdin_notice_answer_box_support/` (no lint suppressions, `TestResult`).
- **BUG-362 (fe0092081)**: `write_exec_stdin` now returns the real write error (`unified_exec::write_stdin` awaits a
  per-write acknowledgement), so a failed write reaches the TUI as `Err`.

## What TUI-117 must change (from interview-decisions.md)
1. **Failed write keeps the text, red inline error.** Today Enter clears the box before the write. Keep the text
   until success: carry it back on failure (e.g. a new `Action::ExecStdinWriteFailed { agent_session, text, error }`
   appended after `ExecStdinControl`, approved in `tests/refac019/appended.rs`), restore it into the box and paint a
   red error row in the box (cleared on the next edit or success). The same applies to a failed
   `control_exec_stdin` (today only logged in `handle_exec_stdin_control`).
2. **Exit or new output closes the notice and box; unsent text goes to box history (Shift+Up).** The backend already
   pushes `StreamChunk::ExecStdinRequestCleared` when the request clears; on that clear, move a non-empty box draft
   into the box input's history and empty it. Check whether new command output clears the backend request
   (`rust/tools/src/unified_exec/exec_stdin.rs` detector) and add that if not.
3. **Ctrl+X mutes further notices for that command run.** Record the dismissed `exec_session_id` per agent session
   in the store; `handle_exec_stdin_prompt_fetched` ignores requests for a muted exec session.
4. **Per session.** Move the box draft and focus into the store's per-session context (like `input_draft`), so
   switching sessions shows that session's own notice, box text and focus. `AgentView::stdin_box` then becomes the
   live editor for the focused session only (swap on session switch, as the chat draft does).

## Files that will be touched and their sizes now
`views/agent.rs` 295 (at most one more line; split under a REFAC card first if more is needed),
`views/agent/input_area.rs` 267, `views/agent/exec_stdin_keys.rs` ~107, `views/agent/stdin_answer_box.rs` ~180,
`app/dispatch_exec_stdin.rs` 247, `components/action_board_card.rs` ~83.

## Regression set
tui116_stdin_notice_answer_box, exec_stdin_push_chunk_bug171, exec_stdin_probe_freshness_bug209,
rig016_steer_queue_keys, tui115_running_composer_steer_queue, inline_hitl_prompt_rpc411, refac019_contracts,
board024_additive_api_contract, source_shape_rpc013/018/020, rpc026_source_shape (all codelet-fspec-tui, one build).
Use `--no-fail-fast`: without it cargo stops at the first failing test binary.
