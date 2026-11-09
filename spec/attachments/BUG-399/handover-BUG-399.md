# BUG-399 handover (2026-11-03, session 2)

Status: implementing. Red seen for all three scenarios of spec/features/session-creation-failures-are-errors.feature
(@wip) in rust/rpc-server/tests/bug399_session_creation_errors.rs; test mappings linked, impl mappings not yet.
That test file is committed and FAILS until the fix lands (intended red state):
- the trait still declares `fn create_session(&self, role: Option<String>) -> SessionId;`
- RPC create_session over MCP returns `{"value":"rpc-no-session-manager"}` without a manager
- RPC create_session returns `{"value":""}` for a malformed default model.

## The fix (bug399-fix.patch in this folder, made against 71d28629d)

- core handle_trait_1.rs: remove `create_session`; `create_session_checked` becomes a required method.
- core stub_handle_1.rs: the stub implements `create_session_checked` (lock poisoning -> Err, broadcast send with
  no subscribers -> debug log).
- sessions handle_impl_1.rs: delete the legacy `create_session` (the empty-id sentinel).
- rpc service_declaration/workspace.rs + service_implementation/interaction_sessions.rs: RPC `create_session` returns
  `Result<SessionId, String>` and delegates to `create_session_checked` ("Session creation requires a session manager"
  without one). Name/order unchanged: wire_order_frozen.txt untouched; only the create_session RESPONSE type changes.
- rpc mcp_dispatch/group_01.rs: create_session maps the Err to `InvokeError::Daemon`.
- ~60 test call sites switch to `create_session_checked` (`?` or `.expect("the handle creates the session")`);
  failure-expecting tests (session_creation_failure_hygiene x2, prov141 cloud scenario, rpc422 persistence failure)
  assert the Err; the prov141 feature step becomes "Then session creation fails with an error"; decline tests
  (no default model) keep asserting Ok(empty) (BUG-410). RPC test callers (rpc-server x7 files, rpc-embedded) add
  `.expect("the daemon creates the session")`.
- sessions/tests/handle_impl.rs TRAIT_METHODS + the matching step in
  implement-sessionmanagerhandle-for-codelet-sessions-sessionmanager.feature: `create_session_checked`.

The patch predates REFAC-054..058, which moved helpers out of prov123, tui099, exec_stdin_prompt_p2, prov145, rpc070
and rpc073 tests: hunks for those files will not apply; redo them by hand (same replacements). rpc070/rpc073 are at
299/297 lines; the BUG-399 edits add about one line each, keep them at or under 300.

## Blockers (split-first rule): do these REFAC cards first, one commit each

- REFAC-059: split rpc422_session_persistence.rs (648 lines) by feature.
- REFAC-060: split exec_stdin_push_chunk_bug171.rs (854 lines) by feature.
- REFAC-061: give rpc002_session_persistence.rs (357 lines, maps to no scenario) a feature and get it under 300 lines.

## After the splits

1. Apply the fix; link impl coverage for the three scenarios (trait, RPC impl, MCP group_01).
2. Green: bug399 + every touched test target (sessions ~25, agent-loop 6, fspec 3, rpc-server 8, rpc-embedded 1,
   fspec-tui cont008 / rpc055 / automatic_compaction_progress_bug210 / escape_compaction_bug214 /
   truthful_compaction_activity_bug213) and rpc449_wire_variant_order, mcp_coverage, narrow clippy.
3. Contracts: the refac019 rpc_declaration / rpc_implementation snapshots change for the frozen `create_session`
   member. Run refac019_contracts with INSTA_UPDATE=always, check that only the create_session digests moved, then
   run refac019_contracts and board024_additive_api_contract (fspec-tui) normally before committing.
4. Report the wire-type change (create_session response: SessionId -> Result<SessionId, String>).

## Closed (2026-11-03, session 3)

Applied after REFAC-059..061. The rejected hunks (prov145, rpc002, exec_stdin_prompt_p2, exec_stdin_push_chunk_bug171,
rpc422 lifecycle/sess002) were redone by hand in the files those refactors created. tui099 gained a
`create_checked` support helper so the rustfmt-expanded calls keep it under 300 lines; rpc343 (unformatted at HEAD,
BUG-382) had only the changed call formatted. Coverage line shifts in 45 features were remapped once from the
HEAD→worktree diff (JSON edited directly, BUG-375). Contracts: only the create_session digests in
refac019_rpc_declaration and refac019_rpc_implementation moved. Pre-existing failures filed: BUG-422 (tui002),
BUG-423 (prov141 profile provider), BUG-424 (unused env macro warning), BUG-425 (duplicate RPC create methods).
