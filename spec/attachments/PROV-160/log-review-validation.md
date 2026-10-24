# Staged and unstaged log-error review

Date: 2026-10-20

## Evidence and scope

Reviewed the combined working tree against HEAD, including staged, unstaged and
untracked changes. The remaining `~/.fspec/logs/fspec-combined.log.2026-10-20`
contains the latest startup, profile-probe 404 warnings and token-estimate
fallbacks, but no ERROR entries. Earlier rotated log excerpts were recovered
from the original audit conversations, including sessions
`4c61ecd5-e732-4be7-88c6-67885b97c938` and
`59756080-5a9a-4301-b218-7f7692e13d30`.

The historical failures include index publication errors, intact messages omitted
from the saved index, compaction admission identity drift, missing tool outcomes,
network setup/read failures and abandoned streamed output. Malformed compaction
candidates and exhausted correction budgets must continue to fail closed; this
review does not weaken those checks or treat external profile 404s as code fixes.

## Additional fixes from this review

- Rebuild a derived message index on lookup miss or identity mismatch, once per
  observed file generation, while holding the shared message lock. Cleanup
  verifies required originals first and refuses deletion if recovery fails.
  Four real-filesystem regressions failed before implementation and now pass.
- Release a deleted session's retained writer lease, preserving the local
  in-flight ownership guard. Both direct-store and singleton cleanup regressions
  failed before implementation and now pass.
- Preserve the active TUI rollback checkpoint when another prompt is queued, and
  dismiss a modal when rollback restores its inspected chunk to an empty
  post-tool placeholder. Both new regressions failed before implementation;
  all seven rollback/modal tests now pass.
- Preserve typed offline recovery during OAuth refresh connection failures,
  without retrying rejected or malformed credentials. Skip already-completed
  checkpoint calls before dialect normalization on interruption. Mark a committed
  tool response recovered before local execution can accept completion and drop
  the remaining stream. Added failing regressions before these fixes.
- Validate a model switch before credential/profile environment mutation; restore
  that environment on preparation or prepublication failure. Adopt a published
  manifest selection when only durability acknowledgement fails. Real directory
  permissions exercise both write and acknowledgement failures, rather than
  merely making the initial manifest read fail. Four failing regressions now
  pass with all eight identity scenarios and five re-resolution tests.
- Link the previously unmapped streamed-response rollback scenario to its
  executable next-turn lineage test. Register the existing agent-manager group
  tag on the new idle-wait race specification and normalize changed Rust files.

## Validation

Tests use explicitly scoped Cargo packages/targets with development and test
debug information disabled. No workspace-wide test run was used.

- Provider baseline: 54 passing tests across 15 targets, including both selected
  transports, DNS/offline recovery, deadlines, metadata, lifecycle and ancestry.
- New provider/interruption boundaries and adjacent regressions: 19 passing
  tests across eight targets; this includes reruns of baseline targets.
- Core persistence, cleanup, message-envelope and invocation-identity targets,
  agent-loop recovery/persistence targets, CLI, TUI, sessions and patched Rig
  serialization tests were also exercised. See the logs for individual results.
- `cargo check` passes for the standalone fspec binary and the six affected
  NAPI-free packages. Library Clippy passes, with one existing unrelated
  `collapsible_match` warning in TUI token state.
- All 1986 feature files parse. Changed feature coverage reports 100% scenario
  mapping coverage, and referenced test/implementation files exist. These are
  traceability checks, not a claim of measured branch coverage.

Logs: `/tmp/fspec-review-current-providers.log`,
`/tmp/fspec-review-boundaries-final-green.log`,
`/tmp/fspec-review-final-core.log`, `/tmp/fspec-review-current-agent.log`,
`/tmp/fspec-review-current-cli.log`, `/tmp/fspec-review-tui-rollback-green.log`,
`/tmp/fspec-model-switch-failure-green.log`,
`/tmp/session-delete-cleanup-green.log`, `/tmp/fspec-review-current-markers.log`,
`/tmp/fspec-review-final-check.log`, `/tmp/fspec-review-current-clippy.log`,
`/tmp/fspec-review-current-coverage-details.log`.

## Remaining limitations

- NAPI compilation still fails because `RequestContextSnapshot` lacks
  `ToNapiValue` and `FromNapiValue` at `rust/rpc-types/src/lib.rs:1501`.
  The production NAPI preparation helpers pass their NAPI-free integration
  tests, but this does not certify the full legacy NAPI crate.
- `rpc080_agent_loop_persistence` has 11 passing tests and one stale source-shape
  assertion requiring token persistence inside `StreamEvent::Done`. HEAD already
  lacks that call; this review leaves that unrelated contract conflict visible.
- Repository-wide tag validation still reports 526 existing violating features.
- The current profile-probe 404 is an endpoint/configuration failure, not repaired
  by the Codex changes. Zero-usage token estimation is a diagnostic fallback.
- No live message history was rewritten or deleted, and no commit was created.
  Recovery of an existing omitted original occurs when the updated code reads
  it; passing isolated regressions is not evidence of a live-database migration.
