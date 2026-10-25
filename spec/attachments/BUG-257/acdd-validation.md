# BUG-257 ACDD validation and review

## Discovery and behavioral red

Native Fspec records the story, rules, examples, answered boundary question,
six Gherkin scenarios, and sequential testing/implementing/validating states.
The first compiled scoped test run had **1 passing control and 5 behavioral
failures**: final/retained-prefix child restoration rejected missing checkpoints,
older boundaries were accepted, corrupt parent state was ignored, and child
checkpoint durability was bypassed. An initial fixture String conversion error
was repaired before accepting the compiled behavioral red as evidence.

## Repair and review

Fork construction validates the parent's content-addressed state (or legacy
checkpoint), requires an exact retained prefix, and reanchors native metadata to
the child's session identity, revision and forked message-reference hash. The
child root is durably published before committing its manifest. The parent root
is neither aliased nor modified. Retained reset provenance remains validated.
Missing/corrupt/unsupported earlier checkpoints fail before child publication.
Compacted native lineage currently fails explicitly rather than manufacturing
a child compaction transition or importing an unsafe checkpoint.

The oversized 980-line manifest file was mechanically split into included
type/store/registry/branch/message/state sections, preserving lexical module
visibility and public exports. TokenUsage stays in the original source path for
its existing source-shape test. A comparison against HEAD verifies identical
non-comment Rust tokens except the new two-line fork-state publication call.
Every touched Rust file is below 300 lines. Existing public API documentation
moved with the types and functions; no behavioral refactor accompanied the split.

Reviewed checkpoint selection, child prefix hashing (including changed fork
message sources), source revision, reset provenance, publication ordering,
explicit failure paths, legacy handling, compaction guards, and parent evidence.
The tests inspect actual on-disk session inventory after rejected publication.

## Commands and outcomes

All commands run from `rust/`, offline with ci-test and -j2:

| Command prefix `cargo test --offline --profile ci-test` | Outcome |
| --- | --- |
| `-p codelet-agent-loop --test bug257_fork_continuation --test prov149_codex_reload -j2` | 6 fork + 3 reload passed |
| `-p codelet-providers --test prov149_codex_history -j2` | 2 passed |
| `-p codelet-cli --test bug243_provider_state -j2` | 8 passed |
| `-p codelet-core --test cmpct064_manifest_transactions --test cmpct055_checkpoint_preservation --test cmpct055_generation_commit --test token_003_reasoning_accumulation_test -j2` | 24 passed, 1 pre-existing source-shape failure |

**43 passing tests; 1 unrelated pre-existing failure**, not an all-green broad
suite claim. Fork tests cold replay through the real SessionManager and capture
the actual serialized next HTTP request using the existing loopback-only offline
Codex fixture. Native reasoning, authentic tool IDs/results, model and selected
prefix survive; later parent content is absent; parent manifest/root bytes remain
unchanged. No external provider calls or outward-facing listener are used.

The unrelated failure is
`persist_token_state_signature_carries_reasoning`: it scans
`rust/agent-loop/src/persist.rs` for a function now defined in `persist_state.rs`.
The scanned file is unmodified and byte-identical to HEAD:
`1d55f994297f934228b09d52c290807896f0038c`. HEAD already reexports the function
instead of defining it there. This failure was not fixed, suppressed, or hidden.
The TokenUsage source-shape regression and all 9 runtime reasoning tests pass.

Successful quality commands:

```text
cargo check --offline --profile ci-test -p codelet-core -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j2
cargo clippy --offline --profile ci-test -p codelet-core -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j2
cargo clippy --offline --profile ci-test -p codelet-agent-loop --test bug257_fork_continuation -j2
```

Test Clippy reports unused helpers in reused transport fixtures; no lint was
disabled. Targeted rustfmt check and `git diff --check` pass. Native feature/tag
validation and coverage audit pass (12 mapped files found). No whole-workspace
or unscoped tests, release workflow, publication, manual stash or commit.

Disk free after scoped validation: **8.2 GiB**, above the 6 GiB floor.
