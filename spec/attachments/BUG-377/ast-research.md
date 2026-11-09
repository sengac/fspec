# BUG-377 AST research

`grep -n unwrap_or_default` over the TUI-117 touch set:
- `app/dispatch_session_cycle.rs:146` (incoming draft)
- `views/agent/input_area.rs:202, 237` (transition state), `:257` (ghost draft)
- `views/agent/hitl_keys.rs:73` (transition state for the cursor gate)

`app/dispatch_exec_stdin.rs:65` — the probe's `Err` arm, `tracing::debug!(.. "silently dropped")`.
