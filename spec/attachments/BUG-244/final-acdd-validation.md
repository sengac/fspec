# BUG-244 / BUG-245 final ACDD validation

## Scope and acceptance audit

This final audit supersedes the pending status in the earlier implementation
handoffs. Existing staged work is preserved. No production behavior was changed
during this audit, and no temporal, step-validation or coverage bypass is used.

An independent read-only review checked all eight feature files against their
actual test functions and implementation paths:

| Work unit | Feature | Scenarios |
| --- | --- | ---: |
| BUG-244 | active-session-first-resume | 1 |
| BUG-244 | lossless-transcript-delivery | 5 |
| BUG-244 | lossless-transcript-tui-hydration | 5 |
| BUG-244 | slash-command-resume | 4 |
| BUG-244 | transcript-hydration-live-state | 3 |
| BUG-245 | archival-model-replay-separation | 5 |
| BUG-245 | archival-transcript-history-access | 2 |
| BUG-245 | cold-empty-failure-display | 1 |

All 26 scenarios have one test mapping apiece, one mapped test file per feature,
and implementation mappings. All 98 scenario-step comments appear inside the
corresponding mapped test ranges. Test and implementation line references were
checked, not merely the existence of referenced files. The independent review
found no additional functional acceptance gap.

## Preserved test-first evidence

Earlier feature-first development and behavioral RED gates are documented in
`remaining-repair-evidence.md`, `delivery-implementation-evidence.md` and
`../BUG-245/implementation-evidence.md`. The original executable logs remain:

- `/tmp/bug244-restore-red2.log`: repeated restoration duplicates history.
- `/tmp/bug245-red3.log`: all three original archive/model scenarios fail.
- `/tmp/bug244-245-audit-red.log`: historical waiting isolation, bounded polling
  tasks and replacement-generation inspection fail their actual assertions.
- `/tmp/bug245-compaction-marker-red.log`: missing compaction metadata boundary.
- `/tmp/bug244-live-waiting-red.log`: live content leaves provider waiting set.
- `/tmp/bug244-245-completion-tui.log`: failed resume permits polling despite
  the strengthened existing acceptance criterion.

Dependency compilation and missing-artifact failures are not behavioral RED or
GREEN. In particular the prior authoritative attempts failed before execution;
they are not used to justify closing either card.

## Current-tree executable gates

The following sequence runs from `rust/`, serially against the existing shared
target directory with bounded parallelism. Output is retained in
`/tmp/bug244-245-finish-verified.log`. The entire sequence completed with
`VALIDATION_EXIT=0`: 56 top-level tests pass, including the fresh-process
archive assertions, followed by successful Cargo check and strict Clippy.

| Gate | Result |
| --- | --- |
| Six BUG-244/245 TUI and resume targets | 36 passed |
| Current header provenance and committed-projection targets | 5 passed |
| Focused cumulative-spend isolation regression | 1 passed |
| Four archive/model and adjacent sessions targets | 8 passed |
| Shared transcript fragment assembler | 6 passed |
| Three scoped libraries, Cargo check | Passed |
| Three scoped libraries, Clippy `--no-deps -- -D warnings` | Passed |

Both cards completed the normal validating-to-done transition through fspec,
with no new production edits needed. Their owned features carry `@done`, and
each card has its final evidence attached. Historical failed and
infrastructure-blocked runs are preserved rather than relabelled successful.

```sh
cargo test --profile release-slim -j2 -p codelet-fspec-tui \
  --test bug244_rendered_transcript --test bug244_transcript_delivery \
  --test bug244_replay_state --test bug245_history_access \
  --test session_restore_tui113 --test slash_resume_rpc049 \
  --test cmpct067_header_provenance \
  --test cmpct068_committed_projection --no-fail-fast
cargo test --profile release-slim -j2 -p codelet-fspec-tui \
  --test cmpct067_coherent_fill multisegment_spend_cannot_change_current_request_fill
cargo test --profile release-slim -j2 -p codelet-sessions \
  --test bug245_archive_model_separation --test bug245_cold_empty_failures \
  --test bug250_cold_terminal_display --test bug250_transcript_replay --no-fail-fast
cargo test --profile release-slim -j2 -p codelet-rpc-types --test transcript_fragments
cargo check --profile release-slim -j2 \
  -p codelet-fspec-tui -p codelet-sessions -p codelet-rpc-types --lib
cargo clippy --profile release-slim -j2 \
  -p codelet-fspec-tui -p codelet-sessions -p codelet-rpc-types \
  --lib --no-deps -- -D warnings
```

## Specification and formatting gates

- All eight features individually pass syntax validation, tag validation,
  `show-coverage` (100%) and `audit-coverage` (no broken links).
- Global syntax validation passes all 2113 feature files. Coverage and syntax
  output: `/tmp/bug244-245-finish-specifications.log` and
  `/tmp/bug244-245-finish-global-validation.log`.
- Global tag validation retains 526 unrelated violating files; all eight owned
  features pass independently. Output: `/tmp/bug244-245-finish-global-tags.log`.
- Scoped rustfmt passes all changed Rust files in the TUI, sessions and RPC-types
  crates: `/tmp/bug244-245-finish-scoped-format.log`. Newly added owned production
  modules stay below 300 lines. Both staged and unstaged `git diff --check` pass.
- Package-wide `cargo fmt --check` also inspects an unchanged BUG-207 test with
  an unrelated wrapping difference. That file is not modified here. Output:
  `/tmp/bug244-245-finish-format.log`.
- The previously explored RPC-101 target has seven existing expectations for
  local cumulative-token fill recomputation, contrary to the authoritative
  CMPCT-067 contract. Those failures remain recorded in
  `/tmp/bug244-245-completion-final-tui-green.log`; no fallback, weakened test or
  lint suppression is introduced. Current CMPCT-067/068 regressions are included
  explicitly in the final scoped run.
- The initial current-tree nine-target TUI command is retained in
  `/tmp/bug244-245-finish-current.log`: all 36 BUG-244/245-related tests pass,
  as do the header/projection checks and two coherent-fill tests. The third
  coherent-fill test, `stale_context_events_cannot_overwrite_a_newer_generation`,
  fails before its assertions because its unchanged synchronous fixture calls
  `BackgroundSession::new`, whose unchanged constructor requires
  `tokio::runtime::Handle::current()`. Both the fixture and this runtime
  requirement are present in HEAD. No unrelated fixture or production runtime
  behavior is changed. The final scoped sequence separately runs the existing
  multisegment-spend test, which directly protects the touched token-state guard;
  it does not claim the entire coherent-fill target passes.
