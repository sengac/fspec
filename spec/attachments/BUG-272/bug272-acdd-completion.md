# BUG-272: RED, minimal fix and complete validation

## Valid RED before production

The first attribution-failure runs are not accepted as causal RED. The fixture
now supplies top-level `work_unit_id: NONE` and registers the same scoped
auxiliary admission used by existing integration fixtures. Real tool execution
appends filesystem effects and can return a genuine error. Retention checks
compare exact outcome content as well as item/call identities.

Before production edits, the six consolidated scenarios ran with:

```
CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --offline --profile ci-test -j2 -p codelet-cli --test bug272_terminal_error_history
```

`/tmp/bug272-post-clean-consolidated-red.log`: **1 passed, 5 failed**. Missing-outcome
refusal passed. Other cases failed on removed calls/user input or false successful
return. The connection-close scenario proves two real filesystem writes and
genuine success/error outcomes before its failing propagation assertion.

The workflow initially rejected two test files for one feature. Consolidation
retains all six scenarios in the original test target; no gate was bypassed.

## Minimal production fix and GREEN

`rust/cli/src/interactive/stream_loop.rs:2349` excludes active Codex sessions from
legacy terminal-error tail surgery. The final explicit provider-error return
remains; earlier context-overflow/recovery paths and non-Codex surgery are
unchanged. Adjacent comments and `recovery_unrecoverable` docs now state that
boundary. No checkpoint, affinity, replay guard or compaction implementation is
changed. No outcomes, cancellation or successful completion are invented.

The scripted transport can genuinely close the continuation connection before
response headers. All **6/6** acceptance tests pass, including overload/auth/bad
request continuation, cold reload of conversation/checkpoint, multi-call genuine
tool errors, retained initial user input, setup/network failure with cold resume,
exactly-once replay/no repeated effects, and typed missing-outcome refusal.

Initial GREEN: `/tmp/bug272-post-clean-green.log`. Final isolated command, from
`rust/`, with the shared approved isolation variables and debug=0:

```
cargo test --offline --profile ci-test -j2 -p codelet-cli --test bug272_terminal_error_history
```

Final log: `/tmp/bug272-final-acceptance.log`, **6 passed, 0 failed**.

## Regressions, failures and corrections

All final scoped targets report **120 passing tests across 35 targets**.
Legacy tail controls pass 8+7 tests; accepted-batch/interruption/reconnect controls
pass 6; selected overflow passes 2. Replay/checkpoint, durable preservation,
missing evidence, lost acknowledgement and prior 273/275–277 controls pass.

The no-substitute target initially failed 0/4 before reaching its intended checks:
its fixture had no data directory, durable manifest/originals or selected model.
The fixture now seeds real temporary persistence and an explicit offline model,
serializes global-state tests, preserves every assertion and additionally checks
unchanged durable manifest bytes. No compaction production code is modified.
Configured rerun: 4/4; its warnings-denied Clippy also passes. The original
coverage snapshot is attached and its four scenario ranges were relinked through
supported CLI commands.

The index control initially lacked its mandatory diagnostic root (1 passed, 8 failed); the
proper temporary `TMPDIR`/root rerun passes 9/9 without source changes. OAuth
recovery initially had 2 passed and 1 failed (`None` versus `WaitingForNetwork` after 150ms).
The controller approved only its temporary subprocess-local CODEX_HOME/mock
keyring fixture triage. A quiescent full-target rerun passes 3/3 without changing
source, timeouts or assertions. This intermittent failure remains recorded and
is not claimed fixed.

All builds, check, four Clippy groups with `-D warnings`, owned formatting and
21 scenario trace chains pass. Exact commands/results and every failed/incomplete
check are in `bug271-272-274-validation-results.json`. All 2161 feature files
validate; five scoped tag checks pass, while the same 522 unrelated global tag
violations remain. No current required check is blocked.

See `spec/attachments/BUG-271/bug271-272-274-safety-and-history.md` for the single
authorized cleanup, honest fixture isolation correction and unchanged real user
config hash. No live affected-session repair or remote schema cause is claimed;
no commit, push, installation, deployment or daemon restart occurred.
