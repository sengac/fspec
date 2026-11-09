# BUG-407 root cause

`scenario_shared_callbacks_are_plain_function_pointers` in
`rust/sessions/tests/wt004_session_tool_callbacks/work_unit_stage_lookup.rs` has `@step` comments ("Given the fspec
binary crate source after this fix", "Then the tool layer's callback types accept the shared functions unchanged")
that match no scenario of the WT-004 feature, and no coverage entry maps it. Its compile-time check (assigning the
shared functions to the codelet-tools callback type aliases) is real, but its only runtime assertion is
`type_name::<IsolationContext>().contains("IsolationContext")`, which cannot fail. Either add a scenario for WT-004
business rule 2 and keep only the compile-time check, or delete the test.
