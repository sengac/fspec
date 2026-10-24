# Independent tail-recovery audit

Status: done after the final finish audit; no commits. Earlier validating entries
below retain their original chronology and are superseded by the final section.

Read the repair plan, framing policy, seven scenarios and their filesystem/property
tests, prior RED/GREEN logs, and current framing/index/writer code. Added two
acceptance scenarios before the new `bug248_reference_evidence` integration
target:

- Missing, malformed and unreadable committed-session evidence must retain the
  JSONL and durably quarantined torn bytes.
- A torn metadata replacement of an already referenced identity must not be
  discarded when its top-level JSON key/value uses whitespace or escaped keys.

The existing `recognizable_id` implementation matches only the literal bytes
`"id":"`. The second scenario checks whether legal JSON representation changes
can bypass the referenced-replacement refusal. No framing production fix has
been made before obtaining executable behavioral RED.

Commands attempted:

`cargo test --profile release-slim -j 2 -p codelet-core --test bug247_stable_message_identity --test bug248_reference_evidence --no-fail-fast`

- `/tmp/bug247-248-audit-first-run.log`: core compiled, then agent-loop gate
  E0308 (`StreamChunk::error` received `&str`). BUG-246 owner notified successfully
  and fixed that call site.
- `/tmp/bug247-248-audit-red-green.log`: unrelated new rpc-types transcript APIs
  lacked `thiserror`/`serde_json` dependencies; their manifest has since changed.
- `/tmp/bug247-248-audit-behavior.log`: sessions transcript journal E0609 at
  `transcript_journal.rs:75`, accessing `.sequence` on `Option<TranscriptCursor>`.
  No target behavior executed in these invocations. Message delivery to the
  transcript owner and parent was rejected as busy.

Both changed feature files pass serialized format/validate. New tests are linked
to the acceptance scenarios, but no missing behavioral evidence is represented as
GREEN. Actual power-loss trials have not been performed.

## Executable RED and scoped repair

`/tmp/bug247-248-audit-behavior2.log` finally executes both new targets after the
external compilation blockers were repaired. Missing/malformed/unreadable
evidence passes, but referenced metadata recovery fails with the actual
assertion `whitespace replacement discarded`: reopening successfully removed
the referenced replacement tail. This behavioral failure preceded production
changes.

The repair adds `messages/framing/identity.rs`, a top-level serde map visitor that
observes the root UUID with JSON whitespace and escapes before the later framing
failure. It does not search arbitrary embedded strings, accept malformed records
for indexing, or change the truncation evidence requirements. The root identity
can only cause recovery to refuse removal. Both whitespace and escaped-key
representations remain in the regression.

The subsequent full scoped command is:

`cargo test --profile release-slim -j 2 -p codelet-core --test bug247_stable_message_identity --test bug248_reference_evidence --test bug247_message_durability --test bug248_tail_recovery --no-fail-fast`

Output is `/tmp/bug247-248-audit-green.log`; do not infer GREEN until its final
results are recorded. Journal owner `bc2c91d4-3bec-46f2-b35e-1a32328d3022` received
the API/behavioral-RED/repair handoff successfully. Other owner delivery remained
busy. No shared session/provider/journal source files were modified by this audit.

## Current integration blocker

The four-target `/tmp/bug247-248-audit-green.log` invocation did not reach tests:
concurrent agent-loop changes introduced undefined `session`, `id`, and `call_id`
values, a `journal` field in `LoopDetectionWiring`, and a missing `journal` field
in `BackgroundOutput`. These are outside this audit's source ownership. By the
time notification was attempted, the prior BUG-246/journal sessions no longer
existed. The parent received the exact failure list successfully. No GREEN is
claimed for that log.

The same four-target command is retried at
`/tmp/bug247-248-audit-final-tests.log`. Final core-only check/clippy output paths
are `/tmp/bug247-248-audit-final-check.log` and
`/tmp/bug247-248-audit-final-clippy.log`; an artifact-lock message alone is not a
passing quality result. The earlier core check/clippy passed before the identity
repair. The post-repair feature format/validation, direct rustfmt check, and both
coverage-path audits pass. All nine scenarios in each feature have exactly one
test mapping after removing stale ranges and duplicate links through the serialized
wrapper. No board status was advanced.

## Final recorded results

The retried full command at `/tmp/bug247-248-audit-final-tests.log` executes both
BUG-248 targets successfully:

- `bug248_reference_evidence`: **2 passed**, covering missing/malformed/unreadable
  committed evidence and referenced replacement identities with whitespace,
  escaped root keys, and escaped UUID values.
- `bug248_tail_recovery`: **7 passed**, including every generated Unicode byte
  truncation, interrupted quarantine, committed corruption, complete undelimited
  JSON, malformed physical lines, and concurrent repair/appends. Property testing
  took approximately 127 seconds in this invocation.

The same full command had one BUG-247 fixture failure, so it did not exit GREEN.
The corrected stable-ID-only rerun passes 4/4 in
`/tmp/bug247-stable-final-green.log`; no further production correction was needed.
BUG-247 durability also passes 5/5 in the full log. Together the recorded runs
validate all 18 distinct scenarios, without claiming a clean exit for the earlier
full invocation.

Post-repair core check and core-lib clippy both pass in the final quality logs.
The exact full command remains recorded above; the final corrected-test command
is `cargo test --profile release-slim -j 2 -p codelet-core --test bug247_stable_message_identity`.
Both cards remain `validating`, with no commits/done transitions. Actual power-loss
survival is not inferred from these deterministic filesystem/reopen tests.

## Final finish audit

The finishing audit verifies the actual prior Cargo RED/GREEN and core quality
logs rather than relying on the prose summary. No further BUG-248 production
change is needed. All **9/9** tests also pass against current full core sources:
reference evidence **2/2**, tail recovery **7/7**, including generated Unicode
byte truncations (138.64 seconds), in
`/tmp/bug247-250-core-finish-direct-regression.log`.

The full-source runner compiles the actual entire core crate against the matching
existing release-slim dependencies and links the tracked integration tests with
the real shared helper crate. The exact seven-target command and **28/28** result
are recorded in `spec/attachments/BUG-249/core-finish-audit.md`; together with the
five new admission/lifecycle regressions there are **33** distinct passing core
tests. Current full-core Clippy with `-D warnings` exits **0** in
`/tmp/bug247-250-core-finish-direct-clippy.log`. Scoped syntax/tag validation,
rustfmt, and all **18/18** coverage paths pass.

The fresh canonical repeat remains queued behind another owner's optimized test
build in `/tmp/bug247-250-core-finish-tests.log`. It has not executed the tests and
is not represented as a fresh Cargo GREEN. Completion uses the verified prior
canonical target/check/Clippy results plus current full-source regression and
quality evidence. No live-store mutation, new Cargo profile/target, workspace test,
or genuine power-loss experiment was performed.

The serialized `validating -> done` transition succeeds after recording this
evidence. BUG-249/250 remains validating for parent coordination.

## Final one-feature/one-test-file correction

The final goal audit finds that the original recovery feature still combines
seven tail-recovery tests and two committed-reference-evidence tests. BUG-248 is
reopened through specifying and testing to correct this traceability boundary.
The two evidence scenarios move unchanged through fspec commands into
`committed-tail-recovery-evidence.feature`. Every existing test and implementation
mapping is preserved; no acceptance criterion or executable assertion changes.

The evidence test file gains its correct feature documentation attribute only.
fspec adjusts its mapped ranges by the two documentation lines and refreshes the
original feature's stale aggregate test-file statistics. The actual command logs
are `/tmp/bug248-one-feature-factor.log`,
`/tmp/bug247-248-one-feature-relink.log`, and
`/tmp/bug247-248-one-feature-stats.log`. Implementing and validating transitions
successfully rerun mandatory coverage and one-feature/one-test-file gates without
skip flags. No production or test behavior is changed during this factoring.

The canonical coordinator execution in
`/tmp/bug242-252-follow-goal-canonical.log` freshly passes the seven tail-recovery
tests, including generated Unicode truncations in 131.98 seconds, and both
reference-evidence tests. It also passes all nine BUG-247 tests and checkpoint
preservation. Subsequent test changes are documentation-only. Original executable
RED evidence remains intact. Scoped rustfmt and whitespace checks pass. The card
remains validating until the final shared Cargo quality gates complete.

Those shared canonical gates now pass in
`/tmp/bug249-250-completion-cargo.log`, ending with `CARGO_COMPLETION_EXIT=0`.
They include current core/agent-loop/sessions library check and strict library
Clippy `--no-deps -- -D warnings`. No alternate compiler or target is used.
Both factored features pass mandatory fspec gates. BUG-248 is eligible for its
final validating-to-done transition.
