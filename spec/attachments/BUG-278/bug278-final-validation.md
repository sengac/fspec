# BUG-278 final validation

Date: 2026-10-28. Repository: `/Users/rquast/projects/fspec`.
This receipt completes validation of the separate BUG-278 follow-up. It does
not count the earlier three-card 120-pass run as new evidence.

## Result and implementation

The valid first and successive Codex durable-compaction paths now advance the
provider generation from verified provider checkpoint lineage, independently
of the durable compaction generation. Versioned `_codexCompaction` metadata
binds those domains and identifies the complete source provider state.
The existing admission, durable promotion, publication and reconciliation
pipeline remains the single owner. Admission rechecks the exact provider state
under the manifest transaction. Core replay, provider installation and cold
preflight share the committed-boundary validator.

The caller-domain cause, executed original/binding REDs and canonical pre-fix
positive control are documented in `bug278-causal-evidence.md`. The positive
scenario remains byte-identical to its verified pre-fix counterpart. Its function
text, with trailing whitespace removed, has SHA256
`f387337294d457d081faabc1db4e85391c7c5a943140bcc9dd30c57f91e36d48`.
`binding-red.log` is a compilation failure, not behavioral RED;
`binding-red-executed.log` is the executed missing-binding RED.

## Final executed checks

All 16 final Cargo commands exited 0. Exact commands and log hashes appear in
the companion JSON. Each label resolves to `.command`, `.log` and `.exit`
under `/tmp/fspec-bug278-isolated/logs/`.

| Final test group | Top-level tests passed |
| --- | ---: |
| BUG-278 acceptance | 7 |
| CLI regressions, including unchanged BUG-212 refusal | 33 |
| Provider lineage, physical freshness, preflight and reset controls | 24 |
| Core durable commit, original blobs and publication ownership | 13 |
| Full fresh-process compaction gate | 11 |
| Session restore, atomicity, lifecycle and pending publication | 17 |
| **Total** | **105** |

Counts use the last top-level result per integration-test target. Nested
child-process test output is not counted again. All scoped groups have zero
failures. The 11-case fresh-process gate passed in 40.27 seconds.

Production library checks passed for providers, core, CLI and sessions.
Both default and `test-stub-provider` fspec binary builds passed.
Clippy with `-D warnings` passed for those libraries, the named CLI/provider/
core/session regression targets, and both binary configurations.
No workspace-wide test or lint claim is made.

Owned-file rustfmt passed with the workspace's Rust 2021 edition and
`skip_children=true`; `git diff --check` passed. An initial Rust 2024 check
reported edition-dependent formatting differences; it changed no files and is
retained as `owned-rustfmt-wrong-edition.{log,exit}`, not counted as final success.
All 21 BUG-278-owned Rust files remain below 300 lines.

## Specification and coverage

`codex-durable-compaction-lineage.feature` passes syntax and registered-tag
validation. Coverage is 100%: seven scenarios, exactly one bounded test mapping
per scenario, one test file and 12 implementation files. The coverage audit
finds all 22 referenced file occurrences.

Final audits also pass for `whole-request-admission-before-dag-commit`,
`session-checkpoint-restore-atomicity`, `compaction-switched-provider-replay`
and `cold-compaction-publication-reconciliation`. Referenced test and
implementation ranges were checked against current file lengths. Pending
publication coverage now maps the complete test at lines 23-154.

The global tag check still exits 1: 1,640 files pass and 522 unrelated files
have existing tag violations. This is a limitation, not an all-project-green
result, and no unrelated tags or feature files were repaired for this card.

## Regression fixture corrections

In addition to the missing-capacity and coherent cold-history corrections
described in the causal attachment, legacy invalid-checkpoint fixtures now carry
actual native ownership metadata (`provider=codex`, empty assistant message).
Malformed version/protocol/null metadata is rejected during replay assembly;
the restore-atomicity controls use identity-bearing replay to reach provider
preflight and preserve unchanged-state and blocked-inference assertions.
The structurally valid future-generation case uses actual persisted replay.

The pending-publication continuation fixture now supplies a consistent 4,096
token reservation to its agents and runtime model-limit override. Its failure
must mention compaction, with zero requests before reconciliation; wrong
identity still fails and the exact identity allows exactly one next request.
Earlier failing session-regression attempts 1-5 remain in separate receipts.

The stale shared-target failure, earlier cold-fixture diagnostic failures and
other attempts remain available; only explicitly named final successful
receipts are counted. No assertion was dropped to turn these failures green.

## Safety and limitations

Every Cargo/test invocation used isolated `FSPEC_USER_DIR`, `XDG_CONFIG_HOME`,
`XDG_RUNTIME_DIR`, `TMPDIR` and debug-0 CI settings. The compiler queue was
serialized, offline, package/target-scoped, `ci-test`, `-j2`, with a minimum
6,000,000-KiB free-space guard. Actual user configuration remains SHA256
`8eed6007c58b8f902efcf640fb8625a2662367cf4551f532ab354773650fce4c`.
No agent HOME or CODEX_HOME was repurposed.

All 16 recorded pre-existing changed-file hashes still match their task-start
values. BUG-271, BUG-272 and BUG-274 remain done and their complete records match
the task-start snapshot. `history_generation.rs` and the CLI
`bug212_selected_provider_checkpoints.rs` remain byte-identical to HEAD.

The failed historical session `ba15d5a4-4ccf-4d70-9ec9-78594797286a` remains
read-only evidence. No resume, retry, live repair, fallback, alternative model,
ancestry reset or history truncation was attempted. Deterministic temporary
effects, loopback transport and child-process evidence are not claims of live
historical-session recovery or arbitrary external-tool exactly-once execution.
The permission-based lost-ack test is Unix-only; no Windows execution is claimed.
The previously documented unrelated intermittent OAuth timing limitation is not
retested or claimed fixed by this card.

No commit, push, installation, binary replacement, daemon restart, deployment,
`cargo clean` or unrelated cleanup was performed. The ordinary Fspec done
transition remains a separate supported operation, with its standard checkpoint
behavior unsuppressed. This receipt records completed validation, not a claim
that an unexecuted status transition has already succeeded.
