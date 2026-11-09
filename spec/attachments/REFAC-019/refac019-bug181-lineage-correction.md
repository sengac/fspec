# REFAC-019 BUG181 lineage correction — final native receipt

Completed the parent-approved SAME-NAME Scenario Outline correction. The old
new-scenario/global-generator approach below is superseded historical evidence,
not the procedure used. No generator, new scenario name or status change used.

## Approved semantic correction and new evidence

The old creation-only Given/When/Then could not honestly describe the deletion
guard, so its earlier native rejection was correct. Parent explicitly approved
expressing the existing generically named live-frame scenario as an outline
with two Examples: {manual:0,auto:0} → write manual ref → {manual:1,auto:0}, and
{manual:1,auto:1} → delete manual ref → {manual:0,auto:1}. The original creation
acceptance remains intact; deletion acceptance is now explicit rather than
falsely borrowing creation steps. Scenario name and 10-scenario count unchanged.

Exactly 3 existing required @step comments per function now match the truthful
outline templates. No additional inline Example comments were added. Concrete
setup/mutations/assertions and descriptive function identities bind each body
to its correct Example. Exact noncomment executable-token equality against
original HEAD was checked for BOTH functions: no assertion or code weakened,
removed or changed. Actual ranges remain creation 164-198 and deletion 147-182.

Native validator source reads raw outline steps without expanding Examples;
both actual native link requests now passed full step validation with the
approved feature/comments. This is materially new semantic evidence, not an
unchanged retry of the rejected creation-steps/deletion-body request.

## Exact final operations

1. apply_patch only: change existing scenario to Scenario Outline, add its two
   explicit Examples, replace the 6 approved @step lines across the two tests.
2. Native format scoped to this feature: formattedCount 1.
3. Native validate and validate-tags scoped to feature: both valid/0 errors.
4. Native link-coverage with BOTH test/impl flags appended creation mapping:
   test rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs:164-198 →
   impl rust/fspec-tui/src/transport/embedded/worktrees_scheduling.rs:[71..77].
5. Native link-coverage with BOTH flags appended deletion mapping:
   test rust/fspec-tui/tests/bug181_live_checkpoint_counts/polling_and_surface.rs:147-182 →
   impl rust/rpc/src/shared_construction.rs:[6..14]. This is the exact nine-line
   physical provenance of the original RPC lib.rs:[819..827] entry.
6. Readback verified all 4 rows in order: historical creation 176-209,
   historical deletion 406-440, corrected creation 164-198, moved deletion 147-182.
7. Native unlink-coverage on original scenario/root testFile removed FIRST row
   176-209. Readback verified 3 remaining rows and next first match 406-440.
8. Same native unlink removed FIRST row 406-440. Readback verified EXACT final
   2 mappings in steps 4/5, preserving their separate impl arrays and functions.
9. Final native validate valid; validate-tags valid; audit-coverage exitCode 0:
   "All files found (22/22) / All mappings valid". Zero new native failures.

## Final preservation and hashes

- 10 scenarios, 10 covered, coverage 100%; no staged mappings remain.
- Full canonical scenario-content SHA256:
  d07b4fc016fc2e82ab2ec563ee6b0f6c07ceb30248c55eef6fa119c8feff0711.
- Replacing only those final 2 mappings in memory with their exact historical
  rows reproduces pinned pre-correction canonical SHA256
  b5d4c58fb55e86e08562b343bb7664246479fe19c5aaa07ca6be7b82ab5b805e.
  This proves all unrelated scenario/test/impl arrays and multiplicity intact.
- Final sidecar SHA256:
  88d8349bfa27595e57f2d6217e29bb053c692b4f521445571147cbcb416ed56c.
- Final feature SHA256:
  0bd0c937687a4da15bce6c211118e6f4a389a7abc7254a4ac138026e138578a9.
- Final root test SHA256:
  becee2c610ffab85c6766b6a899a532ddd4dfb2153b8bd77dba0f1ba396ac7f6.
- Final polling/surface test SHA256:
  0f752439419db5da321d12e73b9d5ff7a998f8682f623b783513caa78dddcfa3.
- All 22 protected-file hashes still match. Implementation source hashes match
  the verified pinned provenance values recorded below; no production edits.
- No builds/tests run by this worker. Parent's pre-comment evidence is
  /tmp/refac019-bug181-final.log: 10 pass, 1 established 9-vs-10-subscriber
  baseline failure; both creation/deletion tests pass there. Parent owns the
  post-comment filtered-pair rerun. No commit/restart/status reset/skips.

## Historical superseded pre-write attempt

The following records the earlier blocked approach and original initial hashes.
No operations in its proposed new-scenario procedure were executed.

## Scope blocker

Native generate-coverage accepts only dryRun (generate_coverage.rs:43) and scans
all spec/features. It has no feature-scoping argument. Repository-wide read-only
precheck found missing unrelated sidecars for client-mcp-read-only,
client-mcp-full-rpc and indexing-lock-diagnostics, plus real scenario drift in
agent-loop-interrupt-cascade. Other regex differences may be docstring examples;
the missing files alone prove generation would write nonowned metadata.

Do not pass an invented feature argument, stage/copy sidecars, or execute global
generation under BUG181-only ownership. Parent coordination/authorization for
the generator's real scope is needed. Attempted parent message returned busy;
this receipt/final response reports the blocker without waiting on the parent.

## Verified semantic correction and exact proposed native operations

Existing creation scenario: Embedded transport delivers a live checkpoint-count
frame end-to-end into the BoardStore. Existing rows in order:
1. bug181_live_checkpoint_counts.rs:176-209 → embedded/worktrees_scheduling.rs:71-77.
2. bug181_live_checkpoint_counts.rs:406-440 → rpc/src/lib.rs:819-827.

First row is the creation function, currently physically at
rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs:164-198 (attribute included).
It remains in the root file; its historical range no longer brackets the function.
The second row belongs to the moved deletion guard at
rust/fspec-tui/tests/bug181_live_checkpoint_counts/polling_and_surface.rs:147-182.

Once generator scope is coordinated:
1. Native add-scenario on live-checkpoint-counts-in-the-board-header:
   Embedded transport delivers a decremented frame after a checkpoint delete.
2. Native add-step given:
   an App wired to an EmbeddedFspecBackend on a temp git repo with 1 manual + 1 auto checkpoint, bootstrapped to { manual: 1, auto: 1 }
3. Native add-step when:
   the manual checkpoint ref is deleted while the App is running
4. Native add-step then:
   the BoardStore reflects the decremented counts
5. Native generate-coverage with coordinated real scope; preserve all old data.
6. Native link-coverage deletion with testFile
   rust/fspec-tui/tests/bug181_live_checkpoint_counts/polling_and_surface.rs,
   testLines 147-182, implFile rust/rpc/src/shared_construction.rs, implLines 6-14.
   These exact nine references preserve original row 2 provenance. This is a
   deliberate scenario-meaning correction: {1,1}→delete→{0,1}, not
   {0,0}→create→{1,0}; no comments or assertions need edits.
7. Native link-coverage creation with testFile
   rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs, testLines 164-198,
   implFile rust/fspec-tui/src/transport/embedded/worktrees_scheduling.rs,
   implLines 71-77. Both flags append the corrected same-file mapping.
8. Read back both complete staged mappings, then native unlink-coverage on the
   old creation scenario/testFile once to remove historical row 1, read back,
   then again to remove historical row 2. The appended creation row must remain.
9. Native validate, validate-tags and audit-coverage for this feature; exact final
   readback must have one creation function and one deletion function, with
   the distinct original implementation lineages above.

## Initial hashes

- Feature: 3a653ffe70c968d39d559bffb54078186643c0543c5cf9696f8474e413cedead.
- Sidecar: 1875439dbfc02c4c500f0309966d436dacee179eaac543d7e64729fb3506b93c.
- Root test: 391a6a9619625fb7a6537c838376d5bb28cdd03eabf1f59f5e472a99c45f8038.
- Moved polling/surface test: 87cc798b1e22732db66541ff30846a8374a404f3c567021b76ce4186b274609e.
- shared_construction.rs: 5c96a6c75abf8074f6fc561737c59c6014450b0546a987fd63ed8014a4c30664 (matches plan).
- embedded/worktrees_scheduling.rs: 364baf32ded2bee1f6e1f9e2ce02568687427b5c9029f7765f3515a9e95c9b06 (matches plan).

No rejected mapping request was repeated. No builds, commits, status resets,
skips, source/test edits or protected-file changes occurred.
