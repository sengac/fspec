# BUG-352 AST research

## fspec astgrep --pattern 'Action::SessionObserved($X)' --lang rust --path rust/fspec-tui/src/app
rust/fspec-tui/src/app/bootstrap.rs:242:41:Action::SessionObserved(info)
rust/fspec-tui/src/app/dispatch_session_lifecycle.rs:97:40:Action::SessionObserved(info)

## fspec astgrep --pattern 'Action::SessionCreated($X)' --lang rust --path rust/fspec-tui/tests/spawned_subordinate_session_registration_parity_rpc385.rs
rust/fspec-tui/tests/spawned_subordinate_session_registration_parity_rpc385.rs:143:18:Action::SessionCreated(id.clone())
rust/fspec-tui/tests/spawned_subordinate_session_registration_parity_rpc385.rs:147:18:Action::SessionCreated(id.clone())
rust/fspec-tui/tests/spawned_subordinate_session_registration_parity_rpc385.rs:172:18:Action::SessionCreated(id.clone())
rust/fspec-tui/tests/spawned_subordinate_session_registration_parity_rpc385.rs:181:18:Action::SessionCreated(id.clone())

The four hits are direct `app.dispatch(Action::SessionCreated(..))` calls in the duplicate and TUI-initiated scenarios. They are still valid, since `SessionCreated` remains the dialog path. The two stale waits are `matches!(a, Action::SessionCreated(s) if ..)` closures inside `wait_for_action(..)` at lines 114-115 and 225-226. ast-grep does not parse macro token trees, so it misses them; grep finds them.
