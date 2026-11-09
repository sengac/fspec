# BUG-394 root cause

`App` Paused probe in `rust/fspec-tui/src/app/dispatch_pause_hitl.rs` (the `tokio::join!` of `get_pause_state`,
`get_hitl_request` and `get_exec_stdin_request`):

- a failed `get_pause_state` or `get_hitl_request` is logged at `debug` ("silently dropped") and folded into `None`,
  so a real pause or HITL prompt is hidden and the session looks idle with nothing at normal log levels;
- `HitlPromptFetched`, `PauseStateFetched` and `ExecStdinProbeFetched` are sent with `let _ = action_tx.send(..)`,
  so a closed app loop is never noticed.

The exec-stdin read in the same join is BUG-364's (probe failures); this card owns the pause and HITL reads and the
three sends.

Fix direction: report each failed read at error level with the session id and surface it in the view (the pause /
HITL slot cannot be trusted), and route the sends through a logging helper such as TUI-117's `send_to_app`.

Update (BUG-364): the `ExecStdinProbeFetched` send in the Paused probe now goes through `send_to_app` together with
the new `ExecStdinProbeFailed`; the `HitlPromptFetched` and `PauseStateFetched` sends remain for this card.
