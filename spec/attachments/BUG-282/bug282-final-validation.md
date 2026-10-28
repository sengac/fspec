# BUG-282 final validation

## Result and limitations

Verified source fix for native persistence changing canonical textual tool
outcomes before selected Codex compaction validates ancestry. The new causal
fixture reproduced the exact preserved ancestry exception before production
edits. It now passes without modifying any lineage, DAG, checkpoint-generation,
admission or atomic-publication safety control.

The failed live session remains untouched. The supplied bounded RPC capture
does not expose its entire checkpoint/conversation, so this proves a concrete
reproducible cause of the same failure, not exclusive historical causation or
repair of that session. No daemon restart, installation or live retry occurred.

## ACDD and validation receipts

- Rules/examples/architecture → formatted valid feature → executable tests →
  behavioral RED → native implementing transition → minimal fix → GREEN →
  native validating transition → regression and quality checks.
- Accepted RED: `accepted-red.log`, **1 passed, 2 failed**, exit 101. One positive
  failure proves exact event-byte mutation; the other reproduces selected
  compaction's `assistant, tool result or DAG ancestry` rejection. Compiler,
  timeout and invalid negative-fixture attempts are retained but not called RED.
- First GREEN and final acceptance: **3/3 passing**, exit 0. Final strict Clippy
  required only an equivalent borrowed expected slice and an earlier lexical
  release of the test's request-capture mutex. Assertions are not weakened.
- **81 distinct passing tests**: new acceptance 3; CLI regression 17; provider
  regression 17; core atomicity/checkpoint regression 9; native persistence/reload
  regression 12; existing unchanged handler unit module 23. Actual child-process
  repetitions are deduplicated in `distinct-passing-tests.tsv`.
- Final check, strict Clippy, library build, formatting, final acceptance and
  preservation all exit 0. Relevant negative tests still refuse missing outcomes,
  changed model/lineage, stale sources and admission-time mutation. Publication
  failure and lost acknowledgement reconcile only the exact promoted identity.
- Scoped feature and tag validation pass; coverage audit finds all 6 mapped
  files. Three scenarios map 1:1 to three tests, with 11 exact step annotations.
  This is traceability coverage, not instrumented code coverage.

All Cargo invocations are offline, package/target scoped, ci-test/debug0, -j2.
The shared original-workspace cache is never used by a copied build tree.
Fixture user configuration, credential paths, data/session stores and temporary
roots are isolated; the final runner also isolates XDG search directories and
unsets inherited provider API-key variables without reading their values.

Key commands (the archived runner supplies isolation and preserves exit codes):

```text
cargo test --offline --profile ci-test -j2 -p codelet-agent-loop --test bug282_canonical_tool_outcomes
cargo check --offline --profile ci-test -j2 -p codelet-cli -p codelet-agent-loop --lib --test bug282_canonical_tool_outcomes
cargo clippy --offline --profile ci-test -j2 -p codelet-cli -p codelet-agent-loop --lib --test bug282_canonical_tool_outcomes -- -D warnings
cargo build --offline --profile ci-test -j2 -p codelet-cli -p codelet-agent-loop --lib
rustfmt --check --edition 2021 --config skip_children=true cli/src/interactive/stream_handlers.rs cli/src/interactive/stream_handler_tests.rs agent-loop/tests/bug282_canonical_tool_outcomes.rs agent-loop/tests/support/bug282.rs
```

Exact regression commands, per-command source hashes, isolated environment
paths and all raw output are in the evidence archive. No standalone fspec
executable build, workspace-wide test or broad lint suppression was attempted.

## Change and preservation audit

Only production behavior: remove the second quote/escape decoding from the
already-canonical streamed ToolResult text. The existing native persistence
consumer now receives unchanged text, as do runtime/checkpoint ancestry and
continuations. No provider/checkpoint/commit production file is changed.

The oversized handler's existing test module was mechanically extracted, with
byte-for-byte preservation verified against HEAD. Handler, extracted module,
new test and helper are respectively 259, 180, 237 and 232 lines. The two added
agent-loop dev dependencies support reuse of existing production replay decoder
source inside the integration fixture; Cargo.lock records those test dependencies.

Every prior Rust manifest entry except the three deliberately owned changed
files verifies. All BUG-280/281 handoff specification hashes verify; both cards
remain **validating**. The original manifest itself retains SHA256
`720c6378ba6debb4363c77f0b17f9f36eada287aa4155c243c66ac7c763ffdb0`.
No pre-existing BUG-280/281 source, test, feature or evidence was rewritten.

The failed session, protected session logs and credentials were not accessed
or altered. No extra session/agent, model change, history/token edit, fallback,
truncation workaround, fabricated result, commit, push or security change.
No permission denial or pending/uncertain approval exists in this investigation.

## Evidence hashes

| Artifact | SHA256 |
| --- | --- |
| Preserved failure JSON | `669fb95222e0a58a4cad1d14e62b052355ef1bdac5adc9c669c5b771ac43d5dd` |
| Preserved bounded RPC bundle | `b5e7bdaf1a49e491dc14bd6aa9c2dfbeb510a4af240417d5981a8f772fae3086` |
| `bug282-findings.md` | `603a5b439ba0643d15f3057e18cc29e4b62866c3ee0e81b03d25e2c83f669a12` |
| `bug282-raw-evidence.tar.gz` | `044cbf8c0d5680e63f825a31b2e0ebf6c14b79385dbb0eb37cccf89d6da64e50` |

The archive contains `artifact-files.sha256`, verified before packaging, covering
all raw logs, command receipts, per-stage source manifests, accepted RED source,
pre-fix handler, final owned-source archive, tracked patch and lint-only test diff.
`bug282-final-hashes.sha256` separately covers the final Markdown and bundle
bytes. Archival source copies are evidence only; nothing is compiled from them.
