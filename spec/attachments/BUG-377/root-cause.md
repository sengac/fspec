# BUG-377 root cause

Found while implementing TUI-117 (`rust/fspec-tui`).

1. `app/dispatch_session_cycle.rs:146` `switch_to_session_index`: the incoming draft is
   `current_session_context().map(|c| c.input_draft.clone()).unwrap_or_default()`. A missing context right after
   `focus_session_index(idx)` is an invariant break, but it silently clears the composer instead of being reported.
2. `views/agent/input_area.rs:202, 237` default a missing session context's `input_transition.state` to `Idle`, and
   `:257` defaults its ghost draft to `""`; `views/agent/hitl_keys.rs:73` does the same for the cursor gate. The
   painted pane then shows a state the session does not have.
3. `app/dispatch_exec_stdin.rs:65` `probe_exec_stdin_for`: a failed `get_exec_stdin_request` is logged at `debug`
   as "silently dropped", so a stale or missing notice has no visible cause at normal log levels.

Fix direction: thread the missing-context case as an explicit branch (skip painting / log an error) instead of a
default value, and log probe failures at `warn` (or surface them like TUI-117's error row when the box is open).

## Scope split with BUG-364

BUG-364 already owns the exec-stdin probe (item 3: the debug-level "silently dropped" log and the discarded
`action_tx.send` results), so BUG-377 fixes items 1 and 2 only. The cursor gate in `hitl_keys.rs` reads
`current_session_context()`, which is `None` only when no session is open at all; it now uses the same explicit
no-session / open / missing classification as the painted row (`views/agent/pane_composer.rs`), with no behaviour
change for the no-session case (a fresh idle composer).
