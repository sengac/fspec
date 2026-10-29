# BUG-284 repair preparation and reviewed causal RED

## Status and approval boundary

BUG-284 remains **Testing**, not Implementing or Done. The authorized preparation
is now reviewable: concrete durable admission design, exact CLI decomposition
proposal, complete acceptance specification, focused compiled failing tests and
source/durability evidence. **No production repair has been implemented.**

The remaining approval boundary is explicit human authorization for mechanical
decomposition of `rust/cli/src/interactive/stream_loop.rs` into the focused
modules listed in `cli-split-proposal.md`. The existing file is 2,564 lines and
its history-clone/prompt-append handoff must change for a complete admission fix.
The repair request alone did not authorize that newly surfaced major refactor.
No production edit, alternate handoff, indirect replacement or bypass has been
made while approval is pending. No other approval is inferred from this report.

## Acceptance specification and traceability

`spec/features/durable-input-admission.feature` defines fourteen scenarios.
Native syntax/tag checks pass. Native coverage audit finds all nine mapped test
references; the independent bounded-range verifier finds exactly one test and
every exact `@step` comment once in each mapped range. No implementation mapping
has been added. Nine of fourteen scenarios have test mappings (**64% traceability,
not acceptance completion, instrumented coverage, or nine passing tests**).

Five scenarios remain deliberately unmapped rather than represented by skeleton
or source-shape tests:

- Cancellation after acknowledged admission/runtime installation but before dispatch.
- Receipt identity, retry/conflicting identity, unknown version, image and duplicate-text semantics.
- Manifest rename/sync acknowledgment loss and unresolved-authority settlement.
- Process death at admission promotion, runtime installation and dispatch boundaries.
- Parallel effects, two actual automatic compactions and cold continuation on the repaired queue path.

The provider-setup mapped test covers construction failure only; its alternative
pre-prompt compaction-failure case also remains to be exercised. The separate
new pending-submission process-death test covers the pre-dequeue boundary, not
all later crash windows. Before semantic production changes, complete these
remaining failing acceptance tests under the approved runtime design.

## Reviewed RED: nine tests, seven causal failures, two passing controls

Authoritative receipt: `red-reviewed.log`; exact command, UTC start/finish,
exit `101`, and test/spec/dependency source hashes accompany it in
`admission-red-receipts.tar.gz`. The `red-reviewed-sources.sha256` manifest was
captured before that run. All production-source hashes remained unchanged.

Command (run only through the isolated launcher, not a production credential environment):

```text
/tmp/fspec-bug284/record red-reviewed cargo test --offline --profile ci-test -p codelet-agent-loop --features test-support --test bug284_durable_input_admission -j2 -- --nocapture
```

| Test | Observed result before any production edit |
| --- | --- |
| Six queued supervisors canceled before admission | Six originals retained once, no canceled runtime/wire instruction or repeated effect; actual selected compaction rejects with Codex assistant/tool-result/DAG lineage mismatch |
| Two supervisors arriving during interruption | Two originals retained once, no canceled runtime/wire instruction or repeated effect; actual selected compaction rejects with the same lineage mismatch |
| Cancellation during a real prompt hook | Original retained once, no dispatch of canceled input; actual selected compaction rejects with the same lineage mismatch |
| Cancellation while original persistence is held | Recovery-spool/message-lock barrier reached; single-worker async heartbeat remains responsive; original retained once; actual selected compaction rejects with the same lineage mismatch |
| Cancellation while awaiting runtime ownership | Original retained once and absent runtime; cold conversation replay differs from runtime |
| Genuine accepted prompt followed by provider construction failure | Original retained once; no new successful provider request/effect; cold conversation replay differs from runtime |
| Child killed after acknowledged submission, before dequeue | Actual child process killed and persisted state reopened; acknowledged pending original is absent (zero recoverable records, expected one) |
| Untouched accepted history control | PASS: real streamed effect/checkpoint, exact replay and selected compaction publish generation 1 without repeating the effect |
| Four historical derivative controls | PASS: all four unmodified divergent requests fail strict lineage; known-history controls accept; altered assistant/invocation/outcome/missing/ancestor projections reject; checkpoints and fixture bytes remain unchanged |

All four selected-compaction failures occur inside production
`Session::commit_selected_compaction`, not a stub's guessed error or a
serialization/constructor failure. Its failure path preserves the durable
manifest and provider checkpoint. The positive control uses the same path.

The queue cancellation tests run the **actual `agent_loop`** and native durable
persistence on the same BackgroundSession that owns a genuine Codex checkpoint
obtained through loopback streaming and a create-new once-only tool effect.
No checkpoint is manually seeded. After cancellation, a fresh instruction
completes on the retained runtime with genuine Codex history, then selected
compaction reconstructs the durable history and exposes the mismatch.

Important limit: initial/fresh accepted Codex requests are constructed by the
test using existing `run_agent_stream` and explicit numeric-loopback transport;
they do not pass through the production provider constructor from the regular
input queue. This is a causal queue/persistence/selected-lineage regression,
**not** proof of full repaired task-loop admission, automatic threshold triggers,
two successive automatic compactions, process-cold continuation, or complete
dispatch race coverage. Those remain acceptance work, not claimed successes.

## Test review corrections and superseded exploratory receipts

Earlier receipts are retained rather than overwritten. `red-initial` included
one historical-control setup error: the real checkpoints end at an invocation
whose genuine outcome occurs in the subsequent reconstructed request. A control
built only from saved checkpoint items omitted that actual outcome. The reviewed
control appends the exact corresponding outcome from the immutable derivative;
it fabricates no success and changes no historical file or checkpoint.

The independent range verifier also caught a test-edit mistake: the fresh
continuation step appeared twice in the six-input test and was absent from the
two-input test. This was corrected in the tests, not by weakening expectations.
Native bindings were relinked to the final ranges. The verifier now requires
each exact step once. In `red-reviewed`, both 6/2 cases execute one fresh
continuation and both fail the actual strict selected-lineage check. Earlier
`red-final`, `red-with-crash` and `red-authoritative` logs are exploratory and
superseded by `red-reviewed`; do not mistake their labels for current evidence.

One formatting check found the newly inserted module out of alphabetical order;
the declaration was reordered before reviewed RED, final check and strict Clippy.
No production change was made to obtain any of these results.

## Passing scoped checks

- `cargo check --offline --profile ci-test -p codelet-agent-loop --features test-support --test bug284_durable_input_admission -j2`: exit 0 (`check-reviewed`).
- Same scoped `cargo clippy` target with `-- -D warnings`: exit 0 (`clippy-reviewed`).
- Changed integration target and all child modules: rustfmt check exit 0 (`formatting-reviewed`).
- Existing BUG-276 admission-cancellation target: 5/5 pass unchanged.
- Existing BUG-282 canonical outcomes target: 3/3 pass unchanged, including two selected compactions/cold continuations and seven ancestry-negative variants.
- Native feature syntax/tag checks pass; native coverage audit 9/9 referenced test bindings; independent mapping/file-size verifier passes.
- All new Rust files are under 300 lines; largest is the 232-line fixture.
- `git diff --check` passes. Production admission/persistence/CLI handoff/provider source diffs remain empty and pre-production source hash verification passes.

Existing BUG-282 controls are preservation evidence, not a substitute for the
missing new automatic-compaction composition scenarios. No workspace-wide test
run or deployed/live repair is claimed.

## Isolation, historical evidence and protected work

The archived launcher clears inherited environment and uses synthetic HOME,
Fspec/Codex/XDG/data/runtime roots, offline Cargo, ci-test/debug0 and two build
jobs. The build cache is reused at `/tmp/fspec-bug280281-target`; there was one
build owner and no concurrent Cargo build observed when starting. Test cwd is a
synthetic project. Real Codex fixture HTTP clients explicitly target numeric
loopback. Non-loopback proxy endpoints are set to a closed local port. No
production credentials, native session prompts or live model API are used.

The four attached projected fixtures are independent prompt-free derivatives
from the reviewer. Their original source hashes and equality/order provenance
remain attached; `extraction-verification.json` is the independent extraction
receipt. They preserve causal projection relationships, not raw prompt bytes,
stream partitions or a runtime repair. Tests never read/write the private failed
sessions or modify their originals. Historical incompatible evidence remains
strictly rejected; no migration, lineage relaxation or prefix filter was added.

BUG-283's final hash manifest still verifies in full. Concurrent TUI/RPC/Cargo
and REFAC-019/BOARD-024 source was not edited by this work. `spec/work-units.json`
was changed only through native fspec mutations for BUG-284, which remains open.

The live executable was rechecked through `/proc/922084/exe` and still hashes to
`d2c910442adbe130e69e921b151c4613b6fb445f87e05ecd3857f83c7064eb4a`.
There has been no install, restart, commit, push, original-history rewrite or
provider guard bypass. Source preparation and passing test-quality checks do
not constitute installed or live-validated repair.
