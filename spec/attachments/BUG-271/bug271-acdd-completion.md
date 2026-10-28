# BUG-271: acceptance and closure evidence

## Current scope and source

The current core isolation, checked-creation, RPC and TUI implementation already
existed at dispatch. Its historical RED is retained in the card's original notes
and research; no new production repair is falsely claimed here. Fresh tests
reassess every requirement. No remote schema cause or live-session repair is claimed.

**13/13 acceptance tests pass:** core isolation 7, checked creation 1, UI/transport
5. The latter exercises both real embedded and WebSocket transports, including
startup/reconnect without a session, global persistence errors versus missing
model declines, durable healthy creation and roles alongside warnings.

Core cases cover empty/foreign-project history, strict damaged targets, supported
legacy defaults without rewriting, unknown-reference cleanup refusal, store-wide
errors, repeated cold loads and safe repaired-fixture discovery. Existing writer
locks, revision CAS, interrupted promotion, lost acknowledgement, unavailable
originals and durable DAG/checkpoint controls remain strict and pass.

## Commands and results

All commands use the explicit approved isolation variables and
`CARGO_PROFILE_CI_TEST_DEBUG=0` recorded in the shared safety attachment and JSON
receipts. From `rust/`:

```
cargo test --offline --profile ci-test -j2 -p codelet-core --test bug271_manifest_isolation --test bug274_manifest_diagnostics
cargo test --offline --profile ci-test -j2 -p codelet-sessions --test bug271_checked_creation --test prov101_no_selection_fallbacks --no-fail-fast
cargo test --offline --profile ci-test -j2 -p codelet-fspec-tui --test bug271_session_storage_diagnostics --test prov101_session_creation_decline --no-fail-fast
```

Results: first command 7+2 pass; sessions 1+3 pass; TUI 5+3 pass. Logs:
`/tmp/bug271-274-final-acceptance.log`,
`/tmp/bug271-272-274-sessions.log`, `/tmp/bug271-272-274-tui.log`.
The transaction target additionally passes 8/8; all 35 final scoped targets
report 120 passes, with nested child executions not added again.

The initial PROV-101 target had 2 passed and 1 failed, with exact error `Failed to persist session
manifest: manifest I/O failed: No such file or directory (os error 2)`.
Its explicit-model test passed alone. Earlier tests initialized the cached store
under a temporary directory subsequently dropped. Adding only
`reset_stores_for_tests()` after switching each fixture data directory fixes
the fixture; every original assertion remains unchanged. Diagnostic and
standalone logs: `/tmp/bug271-isolated-prov101-diagnostic.log` and
`/tmp/bug271-isolated-prov101-standalone.log`. Final full target: 3/3.

## Traceability and gates

The one-feature/two-test-file gate rejected the original organization. The
unchanged checked-creation scenario now resides in
`checked-session-creation.feature`; all five UI cases remain in
`session-storage-diagnostics.feature`. Original feature and mapping snapshots are
registered attachments. Core duplicate/stale ranges and three out-of-bounds
implementation anchors were detected by additional coverage assertions and
reconciled through supported CLI unlink/relink commands, not manual JSON edits.
No acceptance was removed. All 13 scenarios have one current test mapping and
bounded implementation mappings; feature/tag/coverage checks pass.

The full five-feature trace check is
`/tmp/bug271-272-274-coverage-complete.log` (21/21). All 2161 repository features
validate. Global tags still fail on the same 522 unrelated pre-existing files;
all five scoped features pass. No unrelated tag cleanup was performed.

Default and fixture builds, seven-package check, all four warnings-denied Clippy
groups, owned rustfmt and `git diff --check` pass. Exact commands, per-target
counts, earlier failures and superseding results are in
`spec/attachments/BUG-272/bug271-272-274-validation-results.json`.

Authorized cleanup, the honest pre-clean fixture-model correction, verified
temporary config writes and unchanged actual-user-config hash are recorded in
`bug271-272-274-safety-and-history.md`. No current required check remains blocked.
No commit, push, deployment or daemon restart occurred.
