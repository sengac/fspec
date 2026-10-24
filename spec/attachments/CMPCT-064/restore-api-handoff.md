# Current restore/storage ownership

Owner: recovery worker for CMPCT-060/061/062/064.

Current scoped ci-test runs reproduced all five 060 failures, four 061
failures, three 062 failures, and missing-reference failure in 064.
Logs: `/tmp/cmpct060-062-red.log`, `/tmp/cmpct061-062-red.log`,
`/tmp/cmpct064-current.log`.

I am extracting `get_session_message_envelopes` into
`core/src/persistence/session_replay.rs` (same public export), adding
`_sessionReplay` session/revision identity, preserving strings through blob
rehydration, and making missing referenced originals explicit errors.
I am moving handle_impl restore bodies to `sessions/src/session_restore.rs`
and structured conversion to `sessions/src/replay_content.rs`.

Durable owner 7e9bffc3 should coordinate any compaction generation identity
addition against this extraction. Existing manifest transaction APIs remain.
No changes to the selected DAG writer are planned here.

## Resumed ownership 2026-10-17

Worker `8c849f2a-f616-448c-8928-14b5368d9264` resumed the same scope.
Scoped runs are queued in `/tmp/cmpct060-062-resumed.log` and
`/tmp/cmpct064-resumed.log`; the previous green-named restore log contains
an unrelated provider compile failure, not green evidence.
CMPCT-061 adds a behavioral assertion that a repeated restore retains runtime
messages appended after the first installation. Current truncate-to-prefix
implementation drops them. No durable boundary/commit API changes planned.
Reminder and token validation remain separate from provider generation APIs.

Current core 064 nine-scenario suite passes in ci-test:
`/tmp/cmpct064-resumed.log`. Restore validation continues; current real behavioral
061 red is `/tmp/cmpct060-062-runtime-red.log` (post-installation runtime message
lost on second restore). The fix tracks the installed replay span and splices
only that span, preserving both runtime prefix and newer runtime suffix.
060 now uses a distinct child OS process for summary resume and a real captured
provider request; mixed result capture uses the selected Anthropic serializer.

Durable/068 coordination: `restore_tokens` applies saved authoritative counters
and emits `TokenUpdate`, but currently does not publish a committed context
snapshot with generation. Please integrate the durable committed identity and
checkpoint install into the cold restore path as part of055/059/068, not a
fabricated generation from manifest revision. Shared boundary metadata validation
belongs to the durable owner; this worker will not replace that validator.

Latest verification `/tmp/cmpct060-062-verified.log` is compile-blocked by
`E0277` in CLI:069's new `observe_request_usage` wrapper returns a `!Unpin`
stream, breaking the existing streaming consumers. Accounting owner should box
or pin that wrapper without changing restore semantics. Prior
`/tmp/cmpct060-062-final.log` was also compile-blocked by068 DTO initializers,
so neither log is green evidence. Tools tests extracted to
`rust/tools/src/inject_summary/tests.rs`; coverage repair underway.
