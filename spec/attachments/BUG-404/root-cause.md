# BUG-404 root cause

Worktree isolation is enforced only when `codelet_tools::facade::get_isolation_context(session_id)` returns
`Some(IsolationContext)`. It returns `None`, which the path validator treats as "no restriction", in three cases
that are errors rather than "this session is not isolated":

1. `GET_EFFECTIVE_CWD_CALLBACK` was never set (`rust/tools/src/facade/wrapper.rs:734`,
   `.get().and_then(..)`). A front door that forgets to wire the callback gets allow-all silently.
2. `session_tool_callbacks::session_by_id` cannot parse the id (`Uuid::parse_str(..).ok()?`).
3. The registered manager does not know the session (`manager.get_session(..).ok()`).

All three fail open. BUG-398 showed case 1 in practice: a WT-004 test that did not install the callback itself got
`None` when it ran first. The WT-004 scenario "Unknown session ids degrade to no isolation" pins case 3 as intended
behaviour.

Fix direction: make the callback return `Result<Option<IsolationContext>, String>` (or an enum with an explicit
`NotIsolated` variant) so only a found, non-isolated session means "no restriction"; a missing callback, bad id or
unknown session must refuse the file operation with a named error. Update the WT-004 scenario accordingly.
