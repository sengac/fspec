# BUG-242 approved full-repair evidence

## Contract and chronology

The user-approved architecture explicitly rejects merely moving MissingOutcome
to an earlier typed error. The initial failure-only implementation and its logs
are historical, not completion evidence for this repair.

On resuming, the previous final test log showed:
- `bug234_elapsed_runway`: 5 passed.
- BUG-242: SIGKILL before any test output, not GREEN.
- Codex replay: binary absent, never executed, not GREEN.

The board returned to specifying through the serialized wrapper. Discovery added
the approved truthful unknown-outcome rule and examples, removed the superseded
failure-only rule/example, and added an explicit superseding architecture note.
The feature was revised and formatted/validated before revised tests were written.
The board then entered testing. No revised production code precedes executable RED.

## Revised test boundary

Actual `CodexResponsesModel` and its native checkpoint/SSE/request serializer run
against a redirected HTTP endpoint, with real persisted originals and a Notify-
synchronized blocking actor. The native response includes encrypted reasoning,
assistant commentary, distinct item/call IDs, and a checkpointed batch.

Zero-result and partial-batch tests require physical tool-free synthesis with
explicit runtime handoff records, never purported actor success/cancellation.
They check native identities, model, output reservation, unchanged generation,
request admission, no repeated actor execution, and late completion isolation.
Genuine originals results remain exactly once; duplicate genuine results are
idempotent, while conflicting results and wrong native item IDs remain errors.

The completed-result scenario remains a baseline. Both publication-entry routes
retain a prior committed DAG and emit no completion when checkpoint model identity
changes while the actor is running. Invalid native lineage is still unrecoverable.

These publication tests use the real execution kernel at registered handler seams;
they do not claim to exercise remote credential setup or the complete direct
generator factory's success/deferred-apply path. DAG publication/coverage gates
are unchanged. Structural range validation is not proof that missing originals
were read; unknown records are not evidence and synthesis must not invent content.

## Execution

From `rust/`, with exit status preserved:

```sh
set -o pipefail
cargo test -p codelet-agent-loop --test bug242_checkpoint_settlement --profile release-slim -j2 -- --nocapture 2>&1 | tee /tmp/bug242-full-repair-red.log
```

The first revised run waited approximately 45 minutes for the shared artifact
lock, then failed compilation in another owner's in-flight `replay_media.rs`
(missing base64/url dependency declarations). This is NOT behavioral RED.
Those declarations subsequently landed in their owned manifest. The same scoped
command completed in `/tmp/bug242-full-repair-red2.log`: **2 passed, 2 failed**.
Both zero-result and partial-batch tests failed specifically because the old
typed-only barrier refused synthesis. This executable RED was attached before
the board entered implementing and before revised production edits.

The implementation now returns request-local runtime unknown-outcome messages
from the Codex hook, after reconciling genuine identity-matched outputs. It inserts
them before the synthesis instruction, then uses ordinary native request
preparation, full admission and streaming completion. Unknowns are not committed
by the hook; only normal provider completion publication records that request.
Native model mismatch, conflicting output and wrong item identity remain errors.

A streaming pre-tool assistant-message hook captures commentary and call scaffold
before waiting for the actor. Codex retains all observed calls in partial history
and does not construct empty result messages. Production Codex always uses this
streaming execution path; no unused nonstream callback is added. Other providers
retain the previous completed-call filtering behavior, including BUG-234's
existing regression contract and nonstream completed-result capture.

First GREEN/quality command:

```sh
set -e; set -o pipefail
cargo test -p codelet-agent-loop -p codelet-providers --test bug242_checkpoint_settlement --test bug234_elapsed_runway --test codex_replay_regressions --no-fail-fast --profile release-slim -j2 -- --nocapture 2>&1 | tee /tmp/bug242-full-repair-green.log
cargo check -p codelet-agent-loop --test bug242_checkpoint_settlement --profile release-slim -j2 2>&1 | tee /tmp/bug242-full-repair-check.log
cargo clippy -p codelet-agent-loop -p codelet-providers -p rig-core --lib --test bug242_checkpoint_settlement --profile release-slim -j2 2>&1 | tee /tmp/bug242-full-repair-clippy.log
```

The first test command failed compilation in another owner's in-flight core
`conversation_commit.rs:20`: `MessageStore::store_with_id` was not yet defined.
Because the command uses `set -e`, check/clippy did not execute. This log is NOT
behavioral failure or GREEN. It is preserved at `/tmp/bug242-full-repair-green.log`.
The core API remains outside BUG-242 ownership; retry after its owner pairs it.
No revised behavioral GREEN is claimed yet.

Current card status is **implementing**, not done or falsely validated. The
coordinator received the concrete shared blocker and request to activate/reassign
the MessageStore owner. BUG-246 and BUG-251 report the same prerequisite. Their
implementation is not duplicated or bypassed here. After the primitive lands,
rerun the three commands above with fresh `green2`, `check2`, and `clippy2` log
names; attach the completed results and move to validating for parent audit only
when the approved behavioral contract has actually passed.

Owned production scope: compaction execution/runway/settlement, Codex model and
settlement helper, and the two small rig hooks. No core persistence, provider
history/writer, session-provider, live-session or background-output source edits.

## Traceability and static checks

Four scenarios map exactly once to four tests, with 18 exact literal step
annotations and seven production anchors per mapping. This is scenario
traceability, not instrumented line/branch coverage. Coverage metadata is 289
lines. Owned new Rust modules and tests remain under 300 lines.

Targeted feature syntax/tag validation, rustfmt checks on eight owned Rust files
outside patched rig, and `git diff --check` pass. Patched rig has pre-existing
whole-file formatting differences; no unrelated reformatting is performed.
Earlier check/clippy passes in implementation-evidence.md apply only to the
superseded failure-only implementation, not to the approved revised implementation.

All board, attachment and coverage mutations use
`python3 /tmp/fspec-bug242-252-command.py`; all code/doc edits use apply_patch.
No commits, global config edits, live-session mutations or persistence ownership
changes. Parent retains final done audit; leave validating only with honest evidence.
