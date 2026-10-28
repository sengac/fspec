# BUG-282 substantive safety review — superseding findings

## Supersession, not retrospective relabeling

This document supersedes the negative-coverage and completion claims in the
first-pass `bug282-findings.md` and `bug282-final-validation.md`. Those original
attachments, their hashes and their raw archive remain intact as historical
receipts. The first-pass review was incomplete despite its Done status.

The controller's criticism is correct:

- The old `change == "missing"` branch appended invented assistant text. It did
  **not** remove an outcome or prove missing-outcome rejection.
- The old `invocation` branch changed a tool-result `call_id`, not a function
  invocation's arguments or name.
- The old DAG branch added an unestablished DAG message rather than tampering
  an ancestor established by a real committed boundary.
- Object text containing quotes did not independently cover the removed
  outer-quote stripping branch for a canonical JSON-string outcome.

The preserved `accepted-red.log` still establishes two genuine positive-path
failures: exact canonical text divergence and the exact selected-compaction
ancestry exception. It is neither replaced nor invalidated by these corrections.
Its SHA256 remains
`f6c2947dff78696c5804e281ef8bd1e2bf81f8e0930ca0bcc3a074183fc25af1`.
The old 81-test receipts prove their recorded executions, not the missing
negative coverage claimed by the old fixture. They must not be used as that
coverage proof.

## Reopened workflow and preserved recovery receipt

Native supported transitions reopen Done → Validating → Specifying → Testing
→ Implementing → Validating. The three-scenario feature is revised and validated
before test edits. This is a test/specification review; no new production fix or
new production RED is claimed.

The interrupted prior provider attempt did not run review tools. Its supplied
bounded receipt is attached unchanged as `bug282-review-network-failure.json`,
SHA256 `a77c5dc8395b85110befeeaed85861eec14e13eae84c1b1088cb8d46ce4a5cad`.
It records request abortion, network-setup error, Idle and Done at RPC
20887–20890. This continuation does not restart the daemon, change model, spawn
another agent/session, or retry the old failed compaction session.

## Corrected executable coverage

The three original scenarios still map 1:1 to three executable tests. Helper
functions add cases inside those tests, not additional scenario mappings.

### Actual missing outcome original

The isolated real streaming run executes `write_effect` against a once-only
filesystem effect and persists its genuine outcome and checkpoint. A successful
subsequent physical continuation first proves the untampered history works.

The corrected fixture finds exactly one persisted original containing a real
`tool_result`. It removes **that original's manifest reference**, verifies the
manifest count decreased by exactly one and the reference is absent, and verifies
the underlying archived original and live checkpoint remain unchanged. There is
no added assistant text, checkpoint reset, fabricated outcome or compensating
checkpoint publication. Missing here means absent from the authoritative
conversation, not deletion of the archival evidence record.

The actual typed refusal is:

```
CompactionCommitError::Rejected(
  "incompatible provider state: checkpoint identity or source anchor"
)
```

The provider-state validator compares its persisted prefix hash to the changed
manifest references. Here the recorded prefix length still fits, but its hash
does not match. A different missing-original fixture may fail first with
`source prefix length`; that is also a safe refusal, but it is not the precise
result of this fixture. The exact typed assertion follows the actual unchanged
guard, not a substituted invalid state.

`corrected-acceptance.log` honestly records the initial wrong expectation of a
length-specific diagnostic: 2 passed, 1 failed. The genuinely missing state was
already being rejected. The next edit changes only the precise expected
diagnostic to the observed source-anchor refusal; it does not replace the
missing original with another mutation. This run is not called production RED.

### Actual invocation mutation

Separate `CallArguments` and `CallName` cases locate a real `function_call` in
the genuine completed checkpoint. Preconditions assert the real tool name and
parsed original arguments. The fixture changes the actual call's arguments or
name, retains item/call identities, and asserts all other checkpoint items and
the item count are unchanged. Both require exact typed rejection containing
`Codex checkpoint lineage mismatch: tool call ancestry`.

The old output-identity mutation remains as a separately named `OutputIdentity`
case. It is no longer described as invocation mutation.

### Established DAG ancestry

The DAG case first successfully commits and publishes a real admitted durable
boundary, asserts generation 1, cold-restores it, and executes genuine physical
continuations. The fixture then locates the **existing** checkpoint input text
containing the exact committed DAG summary. It asserts the real boundary and
checkpoint generation, changes that ancestor's text in place, and verifies the
persisted committed summary remains unchanged. No new fake DAG message is added;
the item count and all other items remain unchanged.

Selected compaction requires the exact typed assistant/tool-result/DAG ancestry
rejection. This proves refusal of changed established lineage, not merely refusal
of an extra invented message.

### Canonical JSON-string and object outcomes

The positive streaming test now executes two real `EffectTool` returns: the
existing object with escape sequences/Windows paths, and a genuine `Value::String`
whose contents are valid quoted JSON string text. Rig's actual serialization and
tool-result parser deliver the canonical JSON-string representation; the fixture
does not inject or rewrite the emitted result. The test explicitly asserts that
this canonical text begins and ends with a quote and parses as a JSON string.

For both returns, the event, runtime replay, durable outcome original, checkpoint
and actual physical continuation request must preserve **exactly the same text**.
This directly covers outer-quote retention as well as escape preservation. It
does not claim a newly executed pre-fix RED for this additional payload shape.

## Refusal invariants retained and strengthened

All seven negative cases retain:

- typed exact rejection and no promoted identity;
- no publication callback;
- unchanged post-injection manifest, checkpoint and referenced originals;
- unchanged underlying bytes-as-serialized values for **all** pre-injection
  originals, including the removed outcome record;
- ordinary inference still blocked.

Outcome and assistant-text mutations remain. No broad `contains("ancestry")`
substitution hides the missing-original diagnostic; the typed `Rejected` variant
and exact per-case reason are asserted. No production guard or assertion is
disabled, relaxed or changed in this review.

## Fresh validation versus retained regressions

Fresh corrected acceptance: **3 passed, 0 failed**. The raw output names all seven
verified negative cases. First/successive admitted compaction and cold
continuation remain green in the unchanged second scenario.

Fresh Cargo check, strict Clippy (`-D warnings`), rustfmt, scoped Gherkin/tag
validation and coverage audit pass. Coverage has three tests for three scenarios,
11 literal step annotations and all 7 mapped file instances found. This is
scenario traceability, not an instrumented line/branch coverage claim.

The prior 81 distinct test receipts remain preserved. Of those, 78 are unchanged
regression tests and 3 are the old acceptance executions now superseded by the
fresh corrected acceptance. Those 78 regressions were **not rerun** in this
review. Their production and test sources verify unchanged against the prior
source manifest, except the two intentionally revised BUG-282 test/helper files.
The new negative helper is additional test code only. The prior library build
receipt is retained, not described as a fresh build.

## Safety and provenance

All review commands use isolated temporary HOME, FSPEC user/credential paths,
CODEX_HOME, XDG config/data/runtime/search paths and TMPDIR, with inherited
provider key variables unset without reading values. Synthetic stores are real
temporary stores; HTTP endpoints are loopback-only. Cargo is offline, package
and target scoped, ci-test/debug0, -j2, using the same original-workspace cache.
No copied source tree is compiled against that cache.

Prior sealed artifact hashes and internal manifest verify unchanged. The
original failed session remains untouched and unretried; no protected session
logs or credentials are read. BUG-280 and BUG-281 remain Validating and their
handoff specification hashes verify. The denied documentation/executable
boundaries are not touched. No install, restart, commit/push, model/security
change or extra agent/session occurs. No permission denial or pending approval
is introduced by this review.

Final review source hashes, exact commands, exit receipts, review attempts,
selected retained prior receipts and refreshed scoped spec/coverage receipts
are registered with the new raw evidence and hash attachments. The bounded
historical compaction capture still does not prove exclusive incident causation
or live repair; that limitation is unchanged.
