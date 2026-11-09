# BUG-379 AST research

`grep -n 'list_work_units()' rust/fspec-tui/src/app/*.rs`:
- `bootstrap.rs:30` (propagates with ?), `bootstrap.rs:153` (Lagged resync, if let Ok),
- `dispatch_daemons.rs:97` (daemon switch, if let Ok), `dispatch_reconnect.rs` (fixed by BUG-361),
- `dispatch_fspec_runner.rs:22,49` (match with explicit Err results; fine).
