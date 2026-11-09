# BUG-352 root cause

**Symptom.** `spawned_subordinate_session_registration_parity_rpc385::a_spawned_subordinate_appears_as_a_new_tab_in_the_tui` fails at HEAD: "the session-created broadcast must reach the action bus as SessionCreated" (wait_for_action times out after 2s). The lagged-receiver test in the same file waits for the same action and would time out too, once BUG-351's count is fixed.

**Cause.** Commit 2699693b changed the session-created subscriber in rust/fspec-tui/src/app/bootstrap.rs. It now forwards `Action::SessionObserved(info)` (the full `SessionInfo`) instead of `Action::SessionCreated(id)`. `App::handle_session_observed` (app/dispatch_focus.rs) records the project, calls `initialize_session` (the idempotent tab append) and then hydrates work-unit focus. The RPC-385 tests still match `Action::SessionCreated(s) if s.value == ..`, so no action ever matches. The RPC-385 feature's architecture note still says the subscriber "folds each event into Action::SessionCreated(id)".

**Production is correct.** A broadcast still opens exactly one tab, via `handle_session_observed` → `initialize_session`.

**Fix.** The two tests wait for `Action::SessionObserved(info) if info.id == ..`. The first then dispatches it and asserts the single tab. The feature's architecture note names SessionObserved and `handle_session_observed`.

## Red / green (2026-11-02)
- Red: in the BUG-355 batch run, `a_spawned_subordinate_appears_as_a_new_tab_in_the_tui` panicked at :118 ("…must reach the action bus as SessionCreated") and `the_subscriber_recovers_from_a_lagged_broadcast_receiver` panicked at :225.
- Fix: both waits match `Action::SessionObserved(info) if info.id == ..`. The feature's architecture note now names SessionObserved and handle_session_observed.
- Green: `--test spawned_subordinate_session_registration_parity_rpc385` passed 5/5, and narrow clippy with `-D warnings` is clean.
