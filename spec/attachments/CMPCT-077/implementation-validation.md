# CMPCT-077 implementation and validation

Validated on 2026-10-17. This completes the provider-policy slice, not the
remaining durable DAG, lineage, replay or recovery repairs.

## Implemented policy

- Removed the native compaction capability and request trigger generation.
  Hosted search remains independently configurable.
- Native activation through `context_management`, including null or empty
  configuration, returns typed `NativeMemoryUnsupported` before transport.
- Native compaction items fail at direct response, streaming item and terminal
  boundaries. Requests, commits and restored checkpoints enforce the same policy.
- Existing native checkpoints cannot bypass rejection by changing models or
  replacing the incoming conversation prefix.
- All tools, including read-only calls, wait for terminal validation and checkpoint
  commit. A later unsupported item cannot follow local tool execution.
- Encrypted reasoning and negotiated hosted search still replay normally. Legacy
  native checkpoint fields remain readable only to reject incompatible state
  explicitly; they are not an alternate compaction implementation.

## Acceptance evidence

The four scenarios in `codex-sole-dag-memory-policy.feature` map one-to-one to
`rust/providers/tests/cmpct077_sole_dag_memory.rs`. Tests use shared temporary
directory helpers and a real checkpoint file. Rejections verify unchanged memory
and persisted bytes, no final completion, and no dispatched tool call. Native
activation also verifies zero network requests. Supported reasoning/search tests
inspect the next wire request and check opaque content is absent from Debug output.

The initial test run failed three policy scenarios for the intended reasons:
untyped activation rejection, native output handling and native checkpoint replay.
The positive non-compaction feature scenario already passed. Original red evidence
is retained in `/tmp/cmpct077-red.log` for this working session.

## Successful checks

Commands run from `rust/`, with `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_PROFILE_TEST_DEBUG=0` for Cargo checks:

- `cargo test -p codelet-providers --test cmpct077_sole_dag_memory`: 4 passed.
- `cargo clippy -p codelet-providers --test cmpct077_sole_dag_memory`: passed;
  shared baseline fixture reports an unused `stream_body` warning.
- Codex neighboring regression run: 53 tests passed across 19 explicit targets,
  covering transport, lifecycle, history, dispatch, options and hosted features.
- `cargo test -p codelet-agent-loop --test prov149_codex_reload`: 3 passed.
- `cargo check -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop`:
  passed.
- `cargo clippy -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop`:
  passed.
- Targeted rustfmt checks for changed provider Rust files and new policy/test
  files: passed. `git diff --check`: passed.
- `fspec format spec/features/codex-sole-dag-memory-policy.feature` and
  `fspec validate spec/features/codex-sole-dag-memory-policy.feature`: passed.
- `fspec show-coverage codex-sole-dag-memory-policy`: 4/4 scenarios fully mapped.

## Existing repository-wide failures

`fspec validate-tags` reports 1323 passing files and 526 files with existing tag
violations, including unrelated `@fanout` and `@wire-format` registrations and
missing feature-group tags. The sole-DAG feature has no reported tag violations.

Package-wide `cargo fmt -p codelet-providers -- --check` reports existing formatting
differences in unchanged files, including other providers, model cache code and
older integration tests. Those unrelated files were not reformatted.

## Remaining scope

CMPCT-078 owns full committed checkpoint lineage. CMPCT-079 owns failed DAG
recovery and fallback removal; this card does not claim failed-DAG end-to-end
coverage. CMPCT-080 owns replay repair removal, CMPCT-081 transport/mode fallback,
and CMPCT-083 the combined cold-resume/no-fallback acceptance gate.
