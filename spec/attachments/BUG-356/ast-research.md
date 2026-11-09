# BUG-356 AST research
- Test fns in rust/fspec-tui/tests/app_bootstrap_rpc009.rs: 9, one per surviving scenario (list_work_units, subscriber spawn, work_units broadcast, active/background transcript chunks, InputSubmitted, Interrupt, Lagged, Action enum shape).
- `Action::FocusNext` is still declared in src/components/action_runtime.rs:74, but no dispatch arm mutates a RootView (src/app/dispatch.rs only handles FocusNextColumn). RootView no longer exists.
- Bootstrap's create_session is lazy: src/app/bootstrap.rs:108/119 only mention "the lazy `create_session`".
- Gate scope: fspec-core/src/commands/update_work_unit_status/step_docstrings.rs `matching_features` uses only the first `^[A-Z]+-\d+$` tag.
