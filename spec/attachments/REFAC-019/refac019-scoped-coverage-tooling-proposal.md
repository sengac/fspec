# Proposal: precise native coverage mutations without unrelated metadata loss

## Authorization and current boundary

The controller selected **Prepare Tooling Proposal** as a routine workflow step
under the user's existing full-completion request, after the native coverage API's
duplicate-row selector limitation and global generator scope were demonstrated.
This was not additional human authorization for tooling code or activation.
This document is a proposal and test design, not implemented tooling, activated
commands, passing acceptance evidence, or permission to restart the daemon.

REFAC-019 remains validating. BOARD-024 remains testing; its behavior has not
been implemented. BUG-283 remains done and protected. Existing failed sessions,
histories, immutable snapshots and running daemon remain unchanged. No install,
commit, push, security changes, validation skips or metadata path aliases are
part of this proposal.

## Two independently demonstrated tool limitations

### Exact duplicate-row selection

Feature:
`widen-sessionmanagerhandle-fspecservice-both-backends-stub-with-cross-transport-parity-tests`.
Scenario: `every new method has a tarpc service peer and a FspecBackend trait peer`.
Two original test rows represent distinct retained mapping occurrences. Each
original implementation range now spans these actual physical sources:

- `rust/fspec-tui/src/transport/backend/models.rs:5-96`
- `rust/fspec-tui/src/transport/backend/workspace.rs:8-72`

The first row is relocated. The second original row remains intact at
`rust/fspec-tui/tests/rpc037_cross_transport_parity.rs:1111-1238`, with original
implementation `rust/fspec-tui/src/transport/mod.rs:232-389`.

Native `link_coverage.rs` has two relevant behaviors:

- Supplying testLines appends one row containing at most one implementation.
- Implementation-only mutation finds the first row matching testFile; it cannot
  select a second same-file row by index, range or identity.

Thus appending the second replacement row and adding its second implementation
would update the first row instead. Silently merging the two original rows,
inventing a file alias or editing the sidecar outside native validation is not
an acceptable completion strategy. The original second row is preserved.

### Feature-scoped generation

`generate_coverage.rs` accepts only dryRun and scans every feature in the project.
There is no supported feature selector. Its current dry run does not report
scenario-set updates, so it is not a complete write preview.

A global generation would touch unrelated metadata. In particular,
`agent-loop-interrupt-cascade.feature.coverage` retains mapped older scenario
names which differ from the current feature. The current generator's scenario
reconciliation can drop those mapped rows. Missing unrelated sidecars would
also be created. Neither effect is necessary for the authorized cards.

BOARD's two new acceptance features currently have five source scenarios but
only two sidecar scenario entries. The other three cannot honestly be called
mapped until selected-feature generation exists or another supported solution
is separately approved. BUG181 did not use global generation: its existing
same-name scenario became an explicit two-example outline, retaining both
concrete creation/deletion assertions and both original mapping lineages.

## Proposed minimal native API

### Coverage row selectors

Extend native link-coverage implementation-only mode and unlink-coverage with:

- `testMappingIndex`: zero-based index within the named scenario's testMappings.
- `expectedScenarioHash`: required with an index; native readback supplies it.
- Existing testFile remains required and must match the selected row exactly.

Expose indices and the hash through an optional machine-readable native
show-coverage view. Define the hash using a shared canonical JSON encoder:
object keys normalized, array order and duplicate occurrences preserved,
unknown fields included. Return the updated hash after a successful mutation.

An out-of-range index, file mismatch, malformed hash or stale hash is an error
with no write. Mutation must hold the project's existing appropriate coverage
lock across read, precondition checks and atomic replacement; a precondition
checked before a separate unlocked write would not prevent lost updates.
All writers of that sidecar, including legacy append/update/unlink modes and
generation, must participate in the same lock protocol. Atomic file replacement
alone is not a transaction lock. Concurrency behavior needs explicit tests.

Keep existing unselected behavior compatible; do not make old callers silently
target a different duplicate. A selector is an explicit new capability, not an
implicit deduplication or range-based best guess. Use typed thiserror failures
and the same business entry point for CLI and native dispatcher.

The required migration then becomes ordinary validated native operations:

1. Read the scenario, row indices and current hash.
2. Append the second replacement test row with its first implementation.
3. Select that exact new row and add its second implementation, with a fresh hash.
4. Read back the complete two-file union and its original occurrence lineage.
5. Remove only the selected superseded original row with a fresh hash.
6. Verify the complete final multiset, not just coverage percentages.

No full-union JSON import, direct sidecar editor, temporary alias, skipped
validation or running-daemon modification is needed by the API design.

### Selected-feature generation and truthful previews

Add an optional `featureName` argument to generate-coverage. When supplied:

- Resolve one exact feature using established feature-path rules; reject absent,
  ambiguous or invalid targets rather than falling back to a global scan.
- Inspect and write only its corresponding sidecar.
- Preserve existing scenario mappings, duplicate occurrences and unknown fields.
- Report creation, recreation, addition and removal deltas in dryRun as well as
  real mode. Preview must never mutate files.
- Refuse automatic removal of a scenario that still has test/implementation
  lineage. Report that explicit scenario migration is required.
- Refuse to recreate an unreadable or invalid existing sidecar: unknown lineage
  is not proof that it is safe to discard. Preview a blocked repair instead.
- If any precondition/removal/parse conflict blocks generation, leave the entire
  selected sidecar unchanged, even when additions would otherwise be valid.
- Keep the legacy no-selector mode documented as global; this task will not use
  it to resolve selected-feature metadata.

Do not change the daemon schema, security configuration, session lineage,
workflow status enforcement or validation requirements as part of this work.

## ACDD test design before any implementation

Create focused acceptance specifications only after implementation scope is
approved. Each scenario maps to its own meaningful test using existing shared
temp-directory helpers and real feature/coverage files. Establish causal RED
before changing native command behavior. Proposed cases:

1. Add the second physical implementation to the explicitly selected second
   duplicate row; first row and all unrelated mappings stay byte-equivalent.
2. Remove exactly the selected obsolete duplicate; other identical occurrences
   and implementation arrays retain their full multiplicity.
3. Reject stale hash, wrong testFile and out-of-range index without any write.
4. Retain ordinary missing-file and step-comment rejection with a valid selector;
   no selector permits validation bypass.
5. Generate one selected feature's newly added scenario while unrelated missing
   sidecars and mapped scenario drift stay byte-for-byte untouched.
6. Preserve selected-feature unknown fields, mapping order and duplicate arrays;
   reject removal of a scenario carrying lineage without a write.
7. Dry-run reports the exact same planned deltas as real selected generation,
   including updates and blocked removals, while leaving all files unchanged.
8. A competing legacy writer or generator cannot overwrite a selected mutation
   using stale data; stale hashes and held locks prevent lost updates.
9. Corrupt/unreadable selected metadata produces a blocked preview and leaves
   every byte unchanged; no incidental repair or partial additions occur.
10. CLI and dispatcher argument marshalling reach the same native implementation;
   no duplicated business logic or guessed selector semantics.

Use integration-first tests in codelet-fspec-core, with targeted codelet-fspec
front-door tests only where needed. Run package-and-target-scoped offline
ci-test/debug0/jobs2 commands; no workspace-wide test invocation. Follow with
scoped check, Clippy, formatting, feature/tag validation and independent review.
Keep every touched maintained Rust source below 300 lines; obtain approval for
any necessary major decomposition instead of hiding complexity in large files.

## Activation is a separate, still-unapproved step

The running native tool does not gain a new selector merely because repository
source or a test binary changes. No supported live activation path has been
established under the current no-restart/no-install constraint. Do not route a
new shell executable or custom bridge around the required native Fspec tool.

Before implementation or activation, agree on its exact scope and a supported
deployment/activation plan. If the existing daemon cannot expose the tested
capability without a prohibited operation, stop at that boundary and request
new authorization; do not claim the proposal resolves the runtime constraint.

## Separate non-tool blockers and remaining work

The native row selector alone does not finish REFAC-019. The current mapping
receipt also holds RPC026's scenario/test disagreement about independently
seeded services and RPC050's obsolete positive WorkUnitDetached criterion
against the existing negative guard after BUG-205. The additional Mux audit
also holds an Actions-versus-Kanban dropdown criterion and an inherited test
misassociation. These need honest, task-scoped test/spec decisions, not weakened
assertions, guessed test locations or production rollback.

Additional in-bounds source/test provenance drift has exact read-only plans;
unapplied entries remain pending, not grandfathered as historical staleness.
Inherited semantic discrepancies are distinguished from new physical drift.
Finish those supported corrections, re-run the settled multiset/source audit,
attach exact evidence, and verify canonical REFAC-019 Done before BOARD behavior.

BOARD then needs typed completion GREEN, the deeper corrected canonical,
generation, scrolling, attachment/viewer and Mux assertions, explicit additive
contract proof with all frozen original prefixes preserved, native mappings,
quality checks, review and canonical Done. Its current RED is not completion.

## Concrete continuation evidence

- `refac019-native-mapping-rpc-onward-receipt.md`: exact retained duplicate-row
  union, native first-row selector behavior, and separately held RPC026/RPC050
  semantics; no original blocked rows removed.
- `refac019-bug181-lineage-correction.md`: generator's actual global scope,
  mapped unrelated scenario drift, and the completed same-name outline repair.
- `refac019-native-additional-before-rpc-receipt.md`: 16 supported additional
  provenance repairs and the separately held Mux criterion/misassociation.
- `refac019-closure-settled.json`: final read-only audit with pending findings;
  it exits nonzero and is not a claim of closure or a replacement baseline.
- `refac019-tooling-proposal-review.md`: independent read-only review and the
  locking, corrupt-sidecar preservation and evidence clarifications incorporated
  here. No tool implementation or activation was performed by that review.

These receipt basenames refer to attachments on REFAC-019. The exact read-only
provenance plans remain in `/tmp/refac019-*-impl-relocation-plan.json`,
`/tmp/refac019-extra-rpc-plan.json` and the mixed-span continuation plan.
