# Compaction completion coordination — 2026-10-18

## New P1 checkpoint blockers — independent cross-layer review

See `cross-layer-checkpoint-audit.md` for source-verified F1/F2, exact call paths,
affected cards, repair ownership and required runtime/fresh-process regressions.
F1: production Codex checkpoint-only `content:[]` records fail CLI's Anthropic
message deserialization in either captured prefix or retained suffix
(CMPCT-059/072/083; CLI owner). F2: cold restore installs committed G+1 then
last-wins suffix checkpoint replay can overwrite it with G
(CMPCT-061/078/080/083; providers/sessions owners). The gate stream helper also
omits production `bind_provider_persistence`, concealing genuine checkpoint writes.
These are NEW source-review blockers, not executed RED/GREEN evidence. The
canonical identity failure below can mask them; fixing it is not sufficient for
acceptance. No board or acceptance changes accompany this research update.

## Active final083 gate blocker — runtime4 (05da8402)

The complete 17-row matrix executes but cannot reach lifecycle acceptance:
`/tmp/cmpct083-final-runtime4.log` is 0/9 cold-runtime + 0/8 negative, no ignores.
CLI312d7375 MUST canonicalize selected identity consistently in
`session/selected_compaction.rs`, `session/compaction.rs` and
`session/restore_compaction.rs`. The normal saved `manifest.provider` is composite
(`openai/o3`, `anthropic/claude-sonnet-4`, etc.); current runtime names are
provider-only (`openai`, `claude`, etc.). The selected/prepared source comparisons
currently reject every real row. Gate will not alter persisted provider values
to evade real resume. Owner notification is busy/not queued; this is the mailbox.
Detailed evidence: `runtime-api-gaps.md`, `completion-2026-10-18.md`.
Target-scoped gate Clippy queued in `/tmp/cmpct083-final-clippy1.log`.

## Provider blocker for parent routing — 08:43 AEST

Provider final run `/tmp/cmpct-provider-final-green.log`: 16 targets execute,
48 tests pass, one fails (082 poisoned CompactionHook still emits success).
Core owner c13f7921 must change `core/src/compaction_hook.rs` poisoned-lock
branches to cancel rather than return silently. In particular line198 still
logs "skipping compaction check". Real behavioral reproduction is
`providers/tests/cmpct082_physical_admission.rs` (zero-send/no-false-completion).
Rig cancellation itself is fixed and three-phase 082 cancellation is green.
Provider owner will rerun this target after the core fix, not edit core ownership.

071 hook/Copilot override and custom/stub physical serializers are implemented;
stub tool submission API is request `additional_params.stub_tool_call` with
`{id,name,arguments}`, not a global setter. Stub and custom parity tests pass.
082 unknown Codex defaults independently reproduced behavioral RED
`/tmp/cmpct082-missing-codex-limits-red.log`; provider and manager Codex defaults
now preserve unavailable zero, with CLI target green verification in progress.
Direct messages to core and parent cannot queue while busy; this is the handoff.

## Current execution ownership — resumed completion pass

Historical session IDs below are unavailable for messaging; replacement owners
retain exactly the same exclusive file boundaries. No board transition is
authorized until acceptance and quality evidence is verified by the coordinator.

- Core/tools: `c13f7921-2032-47f9-9351-8901e1093e51`.
- CLI runtime: `93bc5d52-b861-4493-8e05-85a764507dce`.
- Agent loop: `520252a1-9e67-48da-a3ea-89cf02756359`.
- Providers and non-provider Rig modules: `dca1b90b-7d8a-4d14-bcb4-c5236e024f5d`.
- Rig provider subtree and builtin serializer tests: `de7a9d26-0be5-4a70-a578-e550f9c8a2e1`.
- Sessions/RPC/TUI except gate tests: `6202ac81-e7d8-45dc-8e28-523f28f654bc`.
- Final gate tests/specification: `eb10bc74-4796-422b-a171-ada06fd03b7b`.

Plan: finish owned implementation gaps with behavioral red/green evidence;
integrate real runtime and cold-process acceptance; verify scoped quality and
traceability; then move all 29 initially active cards through remaining states.
Use the shared `ci-test` cache with `-j 2`, retain logs, and avoid broad builds.
The coordinator owns `spec/work-units.json` exclusively. Preserve all initial
staged/unstaged work and do not commit or stage changes.

Coordinator verification for this pass: `fspec validate` passes all 1882 feature
files (`/tmp/cmpct-current-feature-validation.log`). Global tag validation has
527 pre-existing/unrelated failures; owners must make their active features clean
without unrelated repairs. `/tmp/cmpct-resume-coverage.jsonl` inventories missing
scenario/test and implementation mappings; every owner must refresh its sidecars
after finishing actual behavior. Mapping presence is not measured line coverage.
The missing `HttpClientExt::prepare_request_body` hook still blocked the latest
historical final-check and clippy logs: these are not successful quality evidence.
An inherited shell process (PID 46221 when inspected) waits for that hook, then
runs the seven core targets and tools dispatch/inject targets into
`/tmp/cmpct-core-post-hook-*.log`. Core owner should reuse its results instead of
duplicating those exact runs. Independent read-only cross-layer reviewer
`ed5245da-62fc-4162-b7ed-12a3f471f2d8` owns no files and will report missed
runtime contract violations to the coordinator.

Latest execution: provider baseline now reaches behavioral tests; 078 positive
rows fail `MissingContextCapacity` in
`/tmp/cmpct-provider-resumed-baseline.log`. Keep fixture limits explicit rather
than restoring guessed production defaults; run remaining targets with
`--no-fail-fast` so one failure does not hide subsequent rows. Loop baseline is
temporarily compile-blocked by in-flight CLI Session initializer fields; CLI
owner must complete both constructors before a meaningful runtime run. Shared
disk has 33 GiB free; keep the existing profile/cache and avoid broad builds.

CLI owner: sessions requests
`Session::restore_compaction_boundary(&SessionManifest) -> typed Result`.
Restore authoritative pending identity/inference barrier before replay; a settled
boundary must not claim pending ownership. Exact reconciliation must preserve
newer restored occupancy and saved spend, not replace them with commit estimates.
Details and behavioral target are in CMPCT-068/session-ui-audit.md.

Coordinator review questions for owning agents (confirm against current edits):
- Core: directory-sync failure after manifest rename returns
  `ManifestCommitError::Promoted`; ensure compaction callers expose/retain the
  promoted compaction identity rather than classify it as pre-promotion failure.
- Core: `publish_compaction` invokes its callback while the global store update
  lock is held. A callback must not re-enter `load_session`; bound/reject such
  ownership re-entry or move publication outside the held store mutex.
- CLI: current `retained_reminders` uses serialized substring detection and
  `partition_for_compaction` uses first text markers. Preserve authoritative
  typed reminders without dropping quoted/mixed-content messages or admitting
  empty preservation obligations. Do not let legacy heuristics authorize loss.

The remaining scope is 29 active CMPCT cards (052–083 excluding completed
063, 076 and 077). Existing staged changes are preserved; no commits requested.

## Resumed coordinator checkpoint — 08:23 AEST

Existing seven implementation owners and the read-only reviewer remain active;
do not duplicate their file ownership. The coordinator recovered exact ownership
and review questions from original envelopes and is continuing integration.

- Provider owner: gate's latest `runtime-api-gaps.md` explicitly requests the
  real test-support StubModel serializer, `last_request_bytes()`, and a scripted
  tool submission stream. This is necessary for the seventh live adapter row;
  do not replace it with a fabricated test-side provider. Keep existing stub
  defaults compatible with unrelated tests.
- Sessions owner: gate additionally needs providers test-support enabled in
  sessions dev-dependencies. Latest `/tmp/cmpct083-runtime-first.log` cannot
  compile those imports. The same log still reports ignored production
  reset_provider_state Results in background_session and handle_impl; propagate
  the failures rather than discarding them.
- CLI owner: `/tmp/cmpct-runtime-selected-green1.log` actually has two remaining
  failures: 071 missing-capacity capture exhausts retry budget, and 082 unknown
  limits become `(272000,4096)`. Please retain explicit unknown-limit rejection
  and make fixtures independent of global registered provider state.
- Gate owner: first expanded log is compilation-only, including Copilot client's
  generic type annotation mismatch. Preserve all seven positive/eight negative
  rows while the stub production seam lands.

Core check/clippy logs now finish successfully. This does not yet establish
completion of the cross-layer gates or resolve the promoted-identity and
publication-lock review questions above. Owners should produce final evidence
reports in their attachments when their exclusive slice is ready for acceptance.

## Exclusive file ownership

## Coordinator verification — 08:38 AEST

### Coverage-only ownership transfer

To keep implementation owners focused on the real failures, `ed5245da` now
exclusively owns CLI/loop/sessions/RPC/TUI active feature `.coverage` files:
052/053/054/056/057/058/059/060/061/062/067/068/073/079 and whole-request071,
explicit-request082 and complete-token069 sidecars. Implementation owners retain Rust, feature scenarios and
attachments, but must not concurrently edit these sidecars. Provider coverage
helper `b78ed78e` retains provider-only feature sidecars, excluding these three
shared features. `ed5245da` preserves their existing provider mappings.
Gate083 and core055/064/070/072 sidecars retain their original owners. This is
a traceability-only delegation, not permission to claim tests passed.

Independent reviewer returned two NEW P1 runtime gaps for the CLI owner:

- `execute_compaction` calls `reset_session_to_reminders` in
  `interactive_helpers.rs` before promotion, clearing live messages/turns and
  resetting the provider. Codex reset advances/persists an empty checkpoint.
  Both stream-loop recovery branches still reach it. Prepare an isolated
  builder working view; cancellation/failure after preparation must leave the
  prior manifest generation, checkpoint, DAG and suffix unchanged. Add actual
  recovery-route behavioral RED before repairing this, not helper-only proof.
- `run_compactor_sub_agent_round` currently commits with `|_| Ok(())`, then
  emits only legacy completion. It acknowledges publication without publishing
  the identity-bearing committed context snapshot (unlike TurnCompaction).
  Publish authoritative committed context through this route before clearing
  pending ownership. Actual route tests must assert observer refresh and that
  publication failure keeps the exact identity and inference barrier.

These are not permission for a new summarizer or replacement protocol. Retain
the selected agent-owned DAG and existing exclusive ownership. The read-only
reviewer changed no files and ran no Cargo.

Core ownership regression is now behaviorally green, including reentry and
post-rename identity (`/tmp/cmpct-core-ownership-regression.log`). Builtin
serializer final tests and final test-target Clippy pass. Keep these owners'
file boundaries unchanged while completing downstream runtime evidence.

- CLI/loop: core now exposes `CompactionCommitError::promoted_identity()` for
  both Promoted and Publication errors. Actual runtime callers must retain the
  exact pending identity/inference barrier after irreversible promotion; verify
  this route, not only core's filesystem regression.
- Sessions/CLI: `/tmp/cmpct068-cold-wired-green2.log` now fails all three cold
  targets during restore with `restored selected identity differs`, before the
  pending-barrier assertions. Compare canonical saved provider/model identity
  with the reconstructed Session identity; retain strict identity validation.
- CLI: `/tmp/cmpct-runtime-selected-green5.log` still fails071's three positive
  rows due to missing manifest files (shared data-directory race), and082's
  missing capacities still resolve to272000/4096. Provider implementations in
  `providers/src/model_limits.rs` explicitly retain Codex defaults; coordinate
  with that exclusive owner rather than changing082's expectation.
- Providers: `/tmp/cmpct-provider-final-green.log` passes the first15 targets
  but still fails `inaccessible_admission_state_blocks_the_streaming_agent`
  with `unavailable guard emitted completion`. Cancellation itself now passes.
- Gate: `/tmp/cmpct083-runtime-expanded-red3.log` remains compile-only: missing
  anyhow dependency usage, Vec<String> error conversion, private operation field,
  and OpenAI client type annotation. Finish these owned test API seams promptly
  to execute the actual seven-provider/eight-negative matrix.

All29 cards remain open pending their complete evidence; filenames containing
green/final are not proof of success. Parent has recovered the summary and is
continuing shared traceability/specification verification, not adding owners.

## Coordinator verified integration failures — 08:29 AEST

The following logs executed behavioral tests and are NOT green despite their
filenames. Please finish these specific blockers before another broad audit.

- CLI/loop: `/tmp/cmpct-loop-selected-green.log` fails five targets. Successful
  direct/deferred/cross submissions reject `selected request admission identity
  changed`. Check canonical provider/model identity across source capture,
  `install_selected_preparer`, and test fixtures (selected test agent uses o3).
  Do not remove identity validation to make positive tests pass. Terminal053/054
  still return `Ok(false)` or raw `Unsupported provider: maximum context length
  exceeded`; use declared typed-overflow fixtures, not broad text recovery.
- CLI: `/tmp/cmpct-runtime-selected-green3.log` still fails071 and082. The071
  failures now include manifest conflict expected4/committed0 and missing files,
  suggesting shared test data-directory reset races: serialize all global-state
  mutations with the same serial_test key.082 resolves an unknown model to
  `(272000,4096)` instead of unavailable capacities; coordinate provider defaults
  rather than weakening the expectation.
- Sessions/CLI: `/tmp/cmpct-sessions-cold-current.log` still fails both pending
  tests: restored inference becomes ready and an actual request is sent before
  reconciliation. Saved-spend/newer-occupancy preservation does pass. Inspect
  later projection/token event handlers for clearing the restored barrier.
- Providers: `/tmp/cmpct-provider-cancel-green.log` still fails both082 targets.
  Cancellation's wiremock expects a request even though none is sent; poisoned
  admission currently emits completion (`unavailable guard emitted completion`).
  Preserve zero-wire-request/no-false-completion acceptance; distinguish fixture
  expectation repair from the actual missing typed terminal error propagation.

Keep final evidence reports precise and include failing intermediate runs.
Parent retains all29 cards open. No duplicate implementation owners are added.

- `bbf1af42`: core and compaction tools; 055/064/070/072.
- `68ffd563`: CLI only; 052/057/058/059 runtime, 071/079/082 runtime.
- `8236e3ca`: agent-loop only; 053/054/056, and agent-loop 052/068/069/079 seams.
- `a09f4feb`: providers and non-provider rig patch modules;
  065/066/069/074/075/078/080/081/082, plus custom serializer for 071/083.
- `95c5d624`: rig-core `src/providers/**` exclusively and new
  `providers/tests/cmpct071_builtin_serializers*` targets; actual physical
  serializer parity for built-in Anthropic/OpenAI/Gemini/Copilot adapters.
- `7b2c9024`: sessions, rpc-types and TUI; 060/061/062/067/068/073.
- `4c8b08bc`: sessions `cmpct083*` tests and final gate specifications only.
- Parent: board transitions, shared validation and integration coordination.

## Initial verification

- `fspec validate`: all 1880 feature files valid.
- `fspec validate-tags`: 531 existing feature files have violations; do not
  repair unrelated features. Validate all active CMPCT files separately.
- Core integration baseline using `cargo test --profile ci-test -p codelet-core`
  with explicit CMPCT test targets and `-j 2` is blocked by 12 existing provider
  custom module extraction errors. Log: `/tmp/cmpct-finish-core-baseline.log`.
  This compilation failure is not behavioral red evidence.
- Shared build cache is `rust/target/ci-test`; do not start new target directories
  or broad workspace builds. Provider owner has first compiler slot to repair
  extraction imports. Parent will rerun shared target-scoped baselines afterward.

No card is complete merely because it has coverage mappings or an existing
implementation. Record scenario-specific red/green evidence and exercise the
actual runtime path. Do not weaken acceptance criteria to match helper-only tests.

## Coordinator responses to initial audits

Messages cannot be delivered while agents/parent are running. Use this file and
the per-owner audit attachments as the shared coordination mailbox.

User explicitly approved targeted internal refactors in oversized compaction and
provider modules, keeping new modules under 300 lines. This approval does not
authorize unrelated cleanup or changing the selected DAG architecture.

Critical shared 071/083 blocker: built-in Rig providers have no `prepare_request`
override, so selected admission fails by default. New serializer owner 95c5d624
owns the entire rig provider subtree; a09f4feb retains custom model preparation.
Gate/runtime owners must use the selected physical serializer, not a synthetic
common envelope. Stub positive row needs an explicit tested serializer contract.

- Provider owner: please fix existing custom extraction imports first. The
  initial core baseline fails there before any test execution. Compiler slot is
  available; use `--profile ci-test -j 2` consistently, not release-slim.
- Runtime owner: keep focused internal extractions behavior-preserving and
  below 300 lines. Broad restructuring or API redesign needs user approval;
  propose it before proceeding. Core reports your durable commit currently
  supplies empty required-file/reminder obligations: capture authoritative
  obligations for all actual routes, not route-name validator loops.
- Core owner: run your new red targets once provider imports compile. Use
  ci-test, adding `cmpct070_cold_boundary` to the shared target list. Do not
  claim core validator tests prove direct/deferred/self/cross runtime routes.
- Sessions/UI owner: your added restoration and event-order tests are necessary;
  proceed after scoped behavioral red. RPC DTO baseline passes (1 test), log
  `/tmp/cmpct-finish-rpc-baseline.log`.
- Gate owner: preserve the full seven-provider/eight-negative-row requirements;
  report inaccessible production seams instead of substituting synthetic proof.
- Active feature tag fixes needed: 064, 067, 074, 075 need a registered
  feature-group tag; 083 uses unregistered `@story`. Owners should repair only
  their own active feature files. Overall tag log is
  `/tmp/cmpct-finish-tags.log` (531 unrelated/baseline failures total).
- All owners: refresh actual test/implementation mapping ranges and record
  historical-red provenance honestly. Initial coverage inventory is
  `/tmp/cmpct-finish-coverage-baseline.jsonl`. Parent keeps every card open until
  actual acceptance and quality checks pass.

## Recovery checkpoint — 07:49 AEST

The coordinator has recovered the session and retained all seven exclusive
owners. Shared ci-test compilation is progressing; do not switch profiles or
create alternate target directories. Core owner has been asked to resume its
prepared red tests and fixes rather than stop at an audit.

- CLI owner: sessions' restored-counter red is compile-blocked by missing
  `codelet_rpc_types` in the CLI dependencies. Please repair that manifest seam
  before shared tests can exercise behavior. Gate requests actual integrated
  candidate application and installed selected-preparer access; agent-loop has
  proposed `commit_selected_compaction` plus `install_selected_preparer`.
- Session owner: gate's `runtime-api-gaps.md` identifies pending-publication
  restoration and inference-barrier gaps in addition to saved counter fidelity.
  Please exercise and repair these with actual cold Session runtime tests.
- Agent-loop owner: keep focused extracted implementations below 300 lines and
  minimize the existing oversized loop's changes to delegation wiring. Do not
  undertake a broad decomposition. The CLI central seam must enforce current
  authoritative file/reminder obligations for every route.
- Every owner: leave a precise completion report listing all touched files,
  scenario-specific behavioral red and green logs, remaining blockers, formatting
  and clippy results, and sidecar updates. Do not describe compilation failures
  as behavioral red or scenario traceability as measured line coverage.

The goal remains all 29 active cards, not simply a green helper baseline.

## Integration checkpoint — 07:56 AEST

Coordinator context is pinned and active; retain existing exclusive ownership.
Please finish implementation and publish handoffs before further audit cycles.

- Provider owner: all shared builds now fail because OpenAI serializers call
  the not-yet-defined `HttpClientExt::prepare_request_body`. Add the agreed hook
  and Copilot override urgently, coordinating through the serializer handoff.
- Session owner: pending restore target is compilation-blocked by its reused
  provider fixture importing `futures::StreamExt` without a sessions dependency.
  Repair this owned test dependency seam before claiming a behavioral RED.
- Loop owner: `/tmp/cmpct-agent-loop-submission-red.log` is compilation-only
  (`queue` is unit and has no `apply`). Keep this separate from behavioral RED;
  exercise the existing runtime path once the submission API compiles.
- Core owner: current core seven-target run is GREEN (28 tests), but the
  following generate-compaction tools run failed at the missing provider hook.
  Inject-summary's seven selected unit tests passed. Retain these precise logs.
- Final owner reports must include exact files and scenario-specific proof;
  no card transitions until all required runtime seams and gate rows pass.

## Shared execution observations

- `/tmp/cmpct-runtime-audit-red.log` completed behavioral execution: six CLI
  targets fail (052/057/058/059/071/082); 079's four negative controls pass.
  This is usable behavioral evidence, not evidence that positive commits work.
- `/tmp/cmpct068-audit-rpc-green.log`: both RPC snapshot tests now pass.
- `/tmp/cmpct-provider-audit.log` is compilation-blocked by stale
  `RigAgent::Codex` and `codelet_core::streaming::StreamChunk` references in
  provider-owned `cmpct082_physical_admission.rs`; repair the fixture API before
  evaluating runtime behavior.
- Core055 sidecar currently has three test mappings but no implementation
  mappings. Please complete traceability without treating it as line coverage.
- Existing new Result-returning APIs have unchecked production call sites:
  agent-loop bind_provider_persistence, sessions reset_provider_state in
  background_session/handle_impl. Owners should propagate typed failure rather
  than ignore failure or explicitly discard it.
- Gate owner was idle after the audit and has been asked to resume actual
  runtime gate implementation against the coordinated API, not stop at gaps.

## Integration checkpoint — 07:54 AEST

All existing owners remain exclusive after coordinator recovery; do not spawn
duplicate implementation owners or change the shared profile/cache. The first
TUI dependency build is nearing completion; queued small targets should then
execute normally. Keep logs rather than rerunning expensive Cargo commands.

- Core behavioral RED is now verified in `/tmp/cmpct-core-audit-red.log`:
  four targets expose missing originals, malformed cold boundaries, malformed
  existing DAG metadata and false XML detection inside file names. Green is
  already queued. Session counter RED is verified in
  `/tmp/cmpct068-audit-restore-red2.log` (123 versus expected 333).
- Session owner may add `codelet-agent-loop.workspace = true` under sessions
  dev-dependencies for gate083's actual queued application path. Gate owner
  should proceed writing those tests; the production arrow remains
  agent-loop → sessions. Check the test-only cycle with scoped Cargo metadata
  or the next gate build; do not introduce a normal dependency cycle.
- Provider owner: serializer handoff requests an identity-default synchronous
  `HttpClientExt::prepare_request_body` hook, overridden by Copilot with the
  same cache/body rewriting used by physical send. Coordinate directly with
  serializer owner; otherwise Copilot prepared bytes are not physical bytes.
- CLI owner: publish the central commit/preparer seam promptly after your
  obligations RED. Both agent-loop and gate depend on the actual production
  application path. Ensure obligations are authoritative and cannot be passed
  as empty defaults. Keep operation identity/deadline across retries.
- Gate owner: compiler slot may be queued now on the shared ci-test cache;
  no need to wait for a parent message while parent is busy. Preserve genuine
  live submission, selected adapter, cancellation and fresh-process evidence.
