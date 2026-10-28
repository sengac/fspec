# BUG-278: causal and safety evidence

## Scope

This is the separately authorized compaction follow-up in the existing native
Codex session. The failed session `ba15d5a4-4ccf-4d70-9ec9-78594797286a` was
neither resumed nor retried, and none of its history or stores were repaired.
Its registered issue document remains the evidence source. BUG-271, BUG-272
and BUG-274 remain done; their earlier 120-pass validation is not a new run here.

## Cause

`rust/sessions/src/model_switch.rs::persist_selection` performs the real
`Session::reset_provider_state` transition before persisting a selected model
when there is no durable compaction. Reset advances the Codex provider watermark.
A subsequent real checkpoint can therefore have provider generation 1 while
`compaction_commit` is absent, so the durable compaction source generation is 0.
Provider checkpoints also exist before any compaction; responses within a
provider generation do not constitute new durable compactions.

The old `rust/cli/src/session/compaction.rs` passed durable source generation + 1
to `CodexHistory::prepare_generation`. That passed 1 to a provider already at 1,
correctly triggering the strict `generation must advance with a selected model`
refusal. The error is a caller-domain mismatch, not permission to relax the
provider's advancing-generation contract or to increment arbitrary maxima.

## Executed RED

- The original seven-scenario run compiled and reproduced the generation error.
- After fixture canonicalization, the controller-provided source snapshot was
  verified against `bug278-baseline-source-proof.json` and task-start HEAD.
  Only new test/helper files were copied into its test paths, never production.
- `canonical-baseline-red.log` executed the positive scenario on that verified
  pre-fix source: 0 passed, 1 failed, exit 101, with the exact original generation
  error. The positive test function is byte-identical to the final-source test;
  SHA256 is `f387337294d457d081faabc1db4e85391c7c5a943140bcc9dd30c57f91e36d48`.
- `binding-red.log` was a compiler failure and is explicitly **not RED**. The
  held compound transition did not execute. After repairing fixture compilation,
  `binding-red-executed.log` compiled and executed: 0 passed, 1 failed, exit 101,
  with `missing binding authorized cold continuation`. Only then was the single
  authorized standard transition retry made, followed by binding production code.

## Fix and authority

The existing durable commit/publication pipeline remains the single owner.
Provider preparation first verifies committed replay against the exact live
checkpoint, generation, selected model and full-current-DAG protocol. The
selected serializer validates original conversational ancestry and real outcomes
without applying the replacement request's token budget to an overfull source.
The next provider generation is the verified provider generation plus one;
the durable generation remains the authoritative durable source plus one.

The committed metadata includes `_codexCompaction`, with version, durable and
provider generations, source provider generation and a SHA256 source-state ID
over the generation and complete checkpoint. Preparation is repeated under the
manifest transaction after admission and must reproduce the exact candidate.
An offset checkpoint without the explicit binding is rejected. Older equal-domain
boundaries remain compatible. Core replay, provider installation and cold
preflight share the binding validator and distinguish durable from provider IDs.

No change weakens `history_generation.rs::prepare_generation`, and the existing
`bug212_selected_provider_checkpoints` invalid-lineage test remains unchanged.
No fallback, alternate model, generated tool outcome, history truncation,
ancestry reset or synthetic compaction success was introduced.

## Fixture corrections and limitations

The missing-capacity regression already failed on the verified pre-fix snapshot:
its high-level agent-preparer factory rejected unavailable limits during setup.
The corrected fixture uses the real selected serializer's lower-level preparer
to reach the intended compaction admission veto. Every refusal, preservation,
inference-blocking and no-completion assertion remains intact.

The old cold-process fixture persisted originals and post-capture tool work
without putting them into the runtime that made its real provider requests.
Captured synthetic checkpoint/replay traces established the discrepancy. Its
runtime now starts with its retained originals and receives the same appended
tool pair as its durable store. Codex submission calls use distinct real fixture
identities per response, rather than reusing one ID across cycles. No lineage
assertion was removed to accommodate these corrections.

The new positive control performs first and successive real durable promotions,
identity-bearing publication, fresh-runtime replay and real loopback Codex
transport requests. Captured requests retain genuine outcomes; temporary effect
files stay unchanged and no repeated tool call is emitted. This is deterministic
synthetic integration evidence, not a claim of live historical-session repair.
The existing cold-process gate additionally uses actual child processes.

The mutation test asserts that its mutation callback executed before accepting
the rejection as evidence. The permission-based lost-ack test is `cfg(unix)`;
no Windows execution is claimed. Publication and lost acknowledgement reconcile
only the exact promoted identity, without another generation.

## Safety and documentation registration

Every test/Cargo invocation uses isolated FSPEC_USER_DIR, XDG_CONFIG_HOME,
XDG_RUNTIME_DIR, TMPDIR and debug-0 CI settings; fixtures use temporary stores.
No agent HOME or CODEX_HOME was repurposed. A single compiler queue uses offline,
named packages/targets, `ci-test`, `-j2`, logs, exit receipts and a 6,000,000-KiB
free-space guard. Actual user-config SHA256 remains
`8eed6007c58b8f902efcf640fb8625a2662367cf4551f532ab354773650fce4c`.

The source-snapshot/shared-target run exposed stale-cache behavior during a
later working-tree run. That failed attempt is retained, not counted as final
validation. A byte-identical timestamp-triggered rebuild of the affected
libraries superseded it, without `cargo clean` or source replacement.

Documentation and final receipts are registered with the actual supported
`add-attachment` command: `workUnitId=BUG-278`, `filePath=<existing source file>`.
Registration is verified in the work-unit attachment list and stored relative
paths, rather than inferred from a file merely existing on disk. No commit,
push, install, daemon restart or unrelated cleanup is part of this task.
