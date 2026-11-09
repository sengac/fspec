# BUG-398 root cause (to confirm when worked)

`rust/sessions/tests/wt004_session_tool_callbacks/work_unit_stage_lookup.rs` (serial test, line ~74): after
`session_tool_callbacks::unregister_manager()` it creates an isolated session in `SessionManager::instance()` and
expects `get_isolation_context(uuid)` to find it through the singleton fallback; the lookup returns `None` (panic at
line 101). The other tests in the binary run in parallel with it unless they are also `#[serial]`, and they register
their own managers and set the process-global data directory, so the "no manager registered" precondition and the
singleton's data directory are not guaranteed while it runs. It is also possible the singleton fallback itself
changed (WT-004 / REFAC-023 split the handle and manager). Confirm by running the test alone, then fix the global
state isolation or the fallback accordingly.

**Diagnosis (2026-11-03, during BUG-396's run):** the scenario FAILS when run alone
(`cargo test --test wt004_session_tool_callbacks -- scenario_the_napi_isolation_lookup_falls_back`) and PASSES when the
whole binary runs with `--test-threads=1`. So it depends on process state that a sibling test sets up first (most
likely the data directory / HOME seeding or a registry initialisation done by an earlier test, not by
`manager_with_seeded_cache()` here), and in the default parallel run it races with siblings. Fix: make the scenario
set up everything it needs itself, then make it independent of ordering.

**Confirmed root cause (2026-11-03, BUG-398 worked):** the scenario never installed the codelet-tools effective-cwd
callback itself. `codelet_tools::facade::get_isolation_context` returns `None` when `GET_EFFECTIVE_CWD_CALLBACK` was
never set (`rust/tools/src/facade/wrapper.rs:734`), and only the sibling tests' `register_tool_callbacks` set it. Run
alone it got `None`; after a sibling it passed. That fail-open path is filed as BUG-404.

**The code under test was itself a fallback:** `session_tool_callbacks::session_by_id` fell back to
`SessionManager::instance()` when no manager was registered, for the NAPI front door, which no longer exists. Per the
no-fallbacks rule the fix removes that fallback (with no manager registered the lookup logs an error and finds no
session), and the scenario is replaced by "With no manager registered the isolation lookup does not consult the
singleton manager", which installs the callbacks itself. Red: it found the singleton's session (alone and in the
binary). The other NAPI-era singleton fallbacks are filed as BUG-405. REFAC-053 first split the WT-004 feature so it
has one test file (fspec refused the move to implementing with two `#[path]` fragments).
