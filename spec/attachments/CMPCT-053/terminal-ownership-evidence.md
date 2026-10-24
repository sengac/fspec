# CMPCT-053/054 terminal ownership evidence

## Scope

CMPCT-053 owns terminal session mutation and cancellation cleanup. CMPCT-054
owns the compactor registration lifetime through the final terminal decision.
The implementation passes the existing mutable session owner into recovery;
independent callers acquire that mutex once through the public wrapper.

The registration has an operation-scoped drop owner, including task aborts.
Recovery returns typed unavailable, cancelled or builder-failed outcomes. It
does not itself select a replacement DAG. The shared builder round's removal
of substitute-success behavior belongs to CMPCT-079.

## Specification and test traceability

- `spec/features/terminal-recovery-single-owner-locking.feature`: three scenarios
  mapped to `rust/agent-loop/tests/cmpct053_terminal_lock.rs`.
- `spec/features/terminal-compaction-dependency-lifetime.feature`: three scenarios
  mapped to `rust/agent-loop/tests/cmpct054_terminal_dependencies.rs`.
- Both feature files and their registered tags validate successfully.
- Shared fixtures use the repository temp-directory helper and real outer loop.

## Red evidence recovered

`/tmp/cmpct053-recovered-runtime-red.log` records three failing scenarios against
the pre-fix implementation: ordinary terminal errors exceed the deadline,
failed recovery is not a typed failure, and cancellation cannot enter its
builder. The latter fixture was subsequently corrected to include a completed
user/assistant pair, which is necessary to admit compaction.

`/tmp/cmpct053-resume-red.log`, `/tmp/cmpct053054-preimplementation-red.log`,
and the historical CMPCT-054 metadata attempts hit dependency compilation or
incompatible cached metadata. They are not behavioral red or green evidence.

## Current validation

Command: `cargo test --profile release-slim -p codelet-agent-loop
--test cmpct053_terminal_lock --test cmpct054_terminal_dependencies -j 2`.

`/tmp/cmpct053054-validation.log` is blocked by the new
`StreamChunk::CommittedCompaction` variant missing from
`rust/agent-loop/src/stream_chunk_json.rs`. That serialization integration
belongs to CMPCT-068 and its owner has now supplied the missing arm. The
subsequent attempt at `/tmp/cmpct053054-current.log` compiles the library and
is linking the two scoped integration test binaries. No green test result
or completion is claimed before that run finishes.

Both scenario coverage sidecars have been reset and relinked to remove stale
duplicate mappings and outdated test line ranges. `fspec audit-coverage`
reports all mappings valid for both features. Scoped feature syntax/tag
validation and `rustfmt --check` pass for the helper, tests and shared fixture.

The historical CMPCT-054 attempts do not establish a behavioral red: their
metadata/dependency failures remain explicitly unverified ACDD evidence.
Do not mark that work unit done based on those failures or coverage alone.

The existing `agent_loop.rs` exceeds the file-size limit. Only narrow terminal
ownership/lifetime edits were made; a major extraction requires coordinator
approval and must be coordinated with other agents changing the same file.

No commits were made.
