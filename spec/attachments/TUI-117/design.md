# TUI-117 design

Feature: `spec/features/stdin-prompt-recovery-and-per-session-state.feature`.

## Per-session answer box (view)
`StdinAnswerBox` becomes a map `SessionId -> SessionStdinBox { input, focused, error, history, recall }`
(`views/agent/stdin_answer_box.rs`, history/recall helpers in `views/agent/stdin_box_history.rs`). The session is
resolved from `last_exec_stdin`, which is cached at render for the focused pane only. Switching sessions therefore
shows the incoming session's own box with no swap step; the chat draft was already per session (RPC-024). The
render-time `unfocus()` is removed: focus is closed by the reducers when the request clears.

The view state lives on `AgentView` rather than the store because `MultiLineInput` is a view type; the store keeps
the request slot and the mute set.

## Failed sends
`ExecStdinSubmit` still clears the box on Enter. A failed `write_exec_stdin` emits
`Action::ExecStdinWriteFailed { agent_session, text, error }`; the reducer restores `text` into that session's box
(text typed since goes to the box history, so nothing is lost) and sets the red error row `Not sent: <reason>`.
A failed `control_exec_stdin` emits `Action::ExecStdinControlFailed { agent_session, control, error }` and sets
`Interrupt failed: <reason>` / `End of input failed: <reason>`. Any edit, a success, or a close clears the row.

## Auto-close
The backend detector already clears on exit and new output (`ExecStdinRequestCleared`). The shared clear reducer
`close_exec_stdin` clears the slot, unfocuses that session's box, and moves non-empty box text into its history.
Shift+Up / Shift+Down walk that history while the box is focused.

## Mute
Ctrl+X emits `Action::ExecStdinMuted { agent_session, exec_session }`. The store adds the exec session id to the
session's mute set and clears the slot; `handle_exec_stdin_prompt_fetched` drops requests for a muted exec session.

## Contracts
The three Actions are appended after `ExecStdinControl` and approved in `tests/refac019/appended.rs`. No RPC or wire
type changes.
