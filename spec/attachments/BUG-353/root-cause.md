# BUG-353 root cause

**Symptom (BUG-351 run, 2026-11-02).** `reconnect_resubscribe_rpc415::each_broadcast_stream_delivers_a_post_reconnect_event_to_the_app` panics at rust/fspec-tui/tests/reconnect_resubscribe_rpc415.rs:106 ("Reconnected must trigger the RPC-011 list_work_units refetch"). After `Action::Reconnected`, `mock.list_work_units_calls()` does not rise within 2s. The other three tests in the file pass. The BUG-351 edit to this file only replaced the literal 11 with `common::BOOTSTRAP_SUBSCRIBER_TASKS` (same value), so the failure is pre-existing.

**Hypothesis.** RPC-445 (e178ad5f) made the board workspace-scoped (app/dispatch_workspaces.rs: `spawn_load_workspaces` on bootstrap and reconnect). The reconnect re-bootstrap may now fetch through the workspace path instead of `list_work_units`. Next step: read the Reconnected handler (grep `Action::Reconnected` in rust/fspec-tui/src/app) and decide whether production lost the refetch or the test counts the wrong backend call.

## Investigation (2026-11-02)

**Production is correct.** `App::handle_reconnected` (app/dispatch_reconnect.rs:146-156) still spawns the RPC-011 re-bootstrap: `tokio::spawn` of `backend.list_work_units()` followed by `Action::WorkUnitsLoaded`. RPC-445 only added `spawn_load_workspaces` alongside it.

**Cause: a race in the test.** The test runs on `flavor = "multi_thread", worker_threads = 4` and read `baseline_list = mock.list_work_units_calls()` AFTER `app.dispatch(Action::Reconnected)`. A worker can run the spawned refetch before the next statement. The baseline then already includes the refetch, the counter never rises above it, and the 2s wait fails at line 106. This explains why the test failed in BUG-351's run and passed in the first BUG-355 batch run.

**Red, deterministic.** With a temporary 200ms sleep between dispatch and the baseline read (the refetch always wins), the test failed at :107 in the BUG-353 red run.

**Fix.** Read the baseline before `mock.reconnect_all()` / `app.dispatch(Action::Reconnected)`.

**Green.**
- With the same forced 200ms interleaving, the test passed 4/4 (run 3).
- Without the sleep, it passed 4/4 (run 4).
