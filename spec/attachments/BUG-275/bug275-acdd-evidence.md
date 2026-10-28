# BUG-275 ACDD evidence

## Final-source validation

- `/tmp/bug275-277-agent-owned-final.log`: **49/49** across twelve agent-loop
  targets, including the final session-owned gate, outer-task abandonment,
  replacement-admission exclusion, normal queued next inference, failure state,
  truthful journal outcomes and the BUG276/277 regressions.
- `/tmp/bug275-277-default-final.log`: **12/12** (5 settlement, 7 default RPC088).
  Featured RPC088 remains **8/8** in the agent log, including actual independent
  session interrupt ownership. No default assertions are hidden by test hooks.
- `/tmp/bug275-277-cli-final.log`: **20/20**, including all eight interruption
  execution combinations, attributed namespaced JSON execution and separate
  completed/pending Unicode raw-history preservation. The initially added raw
  helper omitted its original user prefix and correctly failed lineage checks;
  retaining the real original prefix fixed that fixture without relaxing lineage.
- `/tmp/bug275-277-provider-final.log`: **7/7** (prov149 history and prov157 custom
  grammar/normalization/replay). `/tmp/bug277-canonical-helper-tests.log`:
  **35/35**, keeping every canonical helper case after NAPI duplicate removal.
- `/tmp/bug275-277-core-final.log`: **26/26** on final core source. The large
  4097-entry index control now requires at most four bounded writer calls rather
  than the RED measurement of 12,295. Flush precedes sync/promotion/directory
  synchronization; six I/O failpoints and both final-promotion cancellation
  checkpoints preserve original bytes, identity, hashes and cold reopen.
- Extra regressions: ordinary Codex interruption **1/1**, recovery admission
  **2/2**, compaction dependency cleanup **1/1**, truthful cancellation hook
  metadata **1/1**, six BUG214 compaction controls **6/6**. Logs:
  `/tmp/bug275-277-hooks-compaction-final.log` (first three targets),
  `/tmp/bug275-277-compaction-diagnostic.log` (hook pass),
  `/tmp/bug275-277-compaction-six-final.log` (all six controls).
- Extra cross-target cancellation initially failed before its provider barrier.
  Bounded diagnostics exposed the actual error: synthetic caller's registered
  global manager could not resolve its ID. Scoped its real temporary caller with
  register_auxiliary, retaining every barrier, deadline, identity, unchanged
  manifest and no-completion assertion. `/tmp/bug275-277-caller-final.log`:
  **1/1**, no production permission or attribution change. Diagnostic cleanup
  also aborts an owned operation on deadline rather than leaking it.
- Actual final default shipping executable build/check/Clippy all pass:
  `/tmp/bug275-277-{binary,binary-check,clippy}-final.log`. Final NAPI check/Clippy
  pass with **7** remaining warnings, not the earlier 23 after orphaning helpers:
  `/tmp/bug275-277-napi-{check,clippy}-final.log`. RPC unit-binding/large-error
  warnings are in untouched files. Final targeted agent test Clippy passes in
  `/tmp/bug275-277-test-clippy-final.log`; CLI fixture test Clippy passes in
  `/tmp/bug275-cli-test-clippy-final.log`. The latter first caught two denied
  expects and a lexical mutex scope across await in the new legacy helper.
  Replaced them with poison recovery/error propagation and a bounded lexical
  scope, without lint suppression. `/tmp/bug275-cli-fixture-final.log`: **3/3**
  after that final TEST-ONLY correction, with all original assertions intact.
- Native exact-feature check **PASS/PASS/PASS** (Gherkin/tags/format, all four new
  features); coverage audits pass for all four. Native status transitions and
  subsequent show-work-unit readbacks verify **BUG275, BUG276 and BUG277 Done**.
  Global tag validation's 522 unrelated findings are not changed or concealed.
- All new Rust files remain below 300 lines. HEAD remains e486b5bf; staged diff is
  empty; nine original attachment/doc hashes all match the supplied baseline.
  Raw index metadata is not a staged-content comparator. BUG272 remains Testing
  with updatedAt 2026-10-27T10:04:18.244Z. No commit, push, daemon/user-process
  changes, credentials, external model test calls, cleanup or operational agents.
  Disk about 17GiB free; six GiB reserve maintained throughout.

## Ownership and fixture contract review

`/tmp/bug275-lifetime-red.log`: genuine abandonment RED. The actual loop's
IdleOnDropGuard advertised Idle before the real message lease was released,
although the blocking emitter continued settlement. Added a session-owned async
gate, captured by the owned blocking emitter. Admission awaits that same gate;
IdleOnDrop reports Interrupted without Done while it is held. The abort control
also polls replacement admission before release, waits for the real tool outcome,
and awaits settlement ownership release before fixture teardown. Constructors
only gain internal initialized state; public constructor signatures stay intact.
Native testing → implementing for this correction. A first compile caught one
missed Arc forwarder signature; corrected, not a behavioral RED/GREEN claim.

Controller approved bounded TEST-ONLY raw execution fixture alignment through
structured input. Production API-key and OAuth constructors already call
with_strict_attribution, which makes custom registrations JSON (`raw_input=false`,
grammar=None). Executing raw text cannot supply mandatory invocation attribution.
Function fixture context now explicitly provides NONE and a real auxiliary scope.
Namespaced custom execution now models the strict JSON contract, preserving all
completed/cancelled identity, result ordering, replay and normalized-name checks.
`fixtures/legacy_raw_interruption.rs` separately exercises byte-exact Unicode raw
patch normalization, already-completed outcomes, interrupted pending raw calls,
original item/call identities and custom_tool_call_output replay without executing
a raw tool or changing production attribution. Existing prov157 tests retained.
Never call the raw fixture failure an unexplained preexisting product defect.

Controller also requested removal of the 16 newly unused NAPI helpers/imports.
Read-only comparison establishes all **35** NAPI helper tests are byte-exact
duplicates of the canonical agent-loop module after only persistence-path
normalization. Replaced the obsolete duplicate NAPI implementation with public
re-exports of all its production public helpers and factory. The canonical tests
remain unchanged and run explicitly; no lint suppression or helper-test deletion
from the canonical owner. This is bounded duplicate removal, not a storage rewrite.

Current sequential pipeline (no duplicate builds):
`/tmp/bug275-277-agent-owned-final.log`, `/tmp/bug275-277-cli-final.log`,
`/tmp/bug275-277-provider-final.log`, `/tmp/bug277-canonical-helper-tests.log`.
Inspect before claiming success. Final shipping/default/NAPI checks must follow
these latest ownership and NAPI source changes. Earlier successful binary/lint
receipts are not final-source receipts. Disk about 18GiB; six GiB reserve intact.

## Review checkpoint (latest)

- `/tmp/bug275-review-red.log`: two additional genuine REDs after the initial
  five GREENs. A real full agent loop advertised Idle following permanent
  archive failure, and cancellation after the final staged index entry bypassed
  promotion checks. Fixed a shared per-output persistence-failure flag, Interrupted
  outer finalization/no automatic retry, plus checkpoints after flush and before
  rename. No index format, hash or synchronization policy changed.
- `/tmp/bug275-277-agent-final.log`: **49/49** in twelve explicit agent-loop
  targets (featured RPC088 8, BUG275 5, BUG276 5, BUG277 7, provenance 3,
  interrupted tool journal 1, archive recovery 3, originals/blob 9, compaction
  evidence 5, Codex reload 3). `/tmp/bug275-277-default-final.log`: **12/12**
  (BUG275 5 and default RPC088 7). Both include the actual-loop barrier, queued
  genuine next input with no overlapping inference, and actual-loop permanent
  archive failure control. The subsequent tiny schedule-retry guard placement
  correction is present for default validation.
- Core five-target run `/tmp/bug277-core-final.log`: **26/26**, including the
  Unicode tail property regression (154.8 seconds, not a deadlock), peer/process
  coherence, compatible legacy/current indexes, tamper refusal, recovery and
  concurrent index publication. `/tmp/bug277-responsive-final.log`: **2/2**.
- Initial fixture-root retries were environment errors, not test/product passes:
  isolated runtime/config directories had to be recreated before using TMPDIR.
  The successful core run uses matching TMPDIR and FSPEC_INDEX_DIAGNOSTIC_ROOT.
- `/tmp/bug275-277-cli-final.log`: 19 passes and one still-failing target.
  Repaired function-call fixture context with explicit NONE and a legitimate
  auxiliary scope; call-pair, restored interruption, Gemini settlement (9) and
  immediate Done (7) pass. Outcomes fixture likewise omitted attribution and
  auxiliary context; this has now been repaired for function calls but not yet
  rerun. Raw custom-tool calls are a potential compatibility issue with mandatory
  attribution; do not weaken their assertions or claim the failure is preexisting
  without baseline proof. No permission/admission enforcement changed.
- Current production shipping pipeline: default fspec binary build passed;
  default binary Clippy passed with the existing RPC result_large_err warning;
  NAPI check passed (23 warnings); NAPI Clippy still running at last inspection.
  These receipts precede the pending lifetime review below and will require final
  rerun after any further production change. Provider history **2/2** passed.
- Native status returned to **testing** for an additional lifetime control:
  aborting the actual outer agent future while its owned blocking settlement
  waits on a real message lease must not let IdleOnDrop advertise Idle/Done.
  Existing test scenario extended in support/settlement_loop.rs (abandon flag),
  not a new scenario/test mismatch. Not yet executed; no RED claimed yet.
  Proposed bounded repair if RED: a session-owned async settlement gate retained
  by the blocking emitter, awaited by admission and consulted by IdleOnDrop.
- Native coverage range refresh initially added new mappings rather than updating
  old ones; validating gate correctly rejected incomplete old entries. Unlinked
  and relinked only each BUG275 scenario using native commands, one mapping each;
  validating succeeded before the lifetime review returned to testing.
- Native coverage audits pass for both BUG277 features. Global syntax validation
  succeeds. Global tag validation reports 522 unrelated invalid features; focused
  native validation in staging with exact current tags and a read-only board
  symlink gives **4 valid / 0 invalid** for all four new features. Do not change
  unrelated tags/features. No board or coverage JSON is edited manually.

## Replacement continuation checkpoint

Native BUG-275 specifying → testing → implementing; estimate 8. Five original
scenarios remain in `cancellation-settlement.feature`, mapped natively to one
integration file `rust/agent-loop/tests/bug275_cancellation_settlement.rs`.
Native coverage generated in the approved tiny staging project then copied byte
exactly; no unrelated sidecars or board JSON edited manually.

`/tmp/bug275-settlement-red.log`: **2 genuine failing, 3 passing controls**.
Actual temporary message-store lease held during an interrupted stub tool, with
an external two-second watchdog: no visible acknowledgement before release.
4097-entry compatible index publication requires **12,295 actual writer calls**.
Truthful success/error/unknown outcomes, cold reopen/retry, real archive-failure
recovery and six publication failpoints already pass. Only observation/failpoint
scaffolding was added to index I/O before RED, without buffering or reordered
acknowledgement. No provider network or live store is used.

Implementation in progress: nonterminal status acknowledgement bypasses durable
journal admission for BackgroundOutput; original Interrupted/Done semantics stay
after required settlement. Bounded shared-owner constructor additions make an
owned emitter share the same journal, assistant buffer and response checkpoint;
the CLI helper awaits spawn_blocking publication while retaining mutable session
ownership. Arc and ProgressiveOutput forward the new extension. Gemini and Rig
interruption paths use the helper, including synthetic cancellation admission.
64KiB BufWriter wraps the actual observed index sink; explicit flush still
precedes file sync, atomic promotion and directory sync. BUG-277 checkpoints
remain in the staging loop.

Current run `/tmp/bug275-settlement-green.log`; no GREEN claim until inspected.
Same sequential pipeline reruns five core targets and async search responsiveness
in `/tmp/bug277-core-final.log` and `/tmp/bug277-responsive-final.log`. Earlier
core invocation failed only its explicit fixture-root guard: first missing
FSPEC_INDEX_DIAGNOSTIC_ROOT, then macOS OS-temp path outside /private/tmp.
Current command sets both TMPDIR and the guard root to the isolated runtime dir.
Do not call those failures preexisting or successful. No build duplication.

## Preservation and remaining work

HEAD remains e486b5bf9af3ec55ad0c55aa56e3ee8b5c499067. No commits, deployment,
daemon restart, additional sessions, model changes, live history, cleanup or
credential access. Six GiB disk reserve; about 18GiB available. Native BUG-276
remains Done; BUG-277 moved to Validating, not Done. Its captured compaction
fixture now supplies real invoking attribution NONE without weakening any
assertion; `/tmp/bug277-compaction-final.log` **5/5 pass**.

Remaining: inspect GREEN, strengthen genuine worker responsiveness/new-turn and
failure admission controls as necessary; current-source scoped regressions,
default shipping binary build/check/Clippy and NAPI checks; native feature/tag
validation/coverage refresh, source review and all three native Done readbacks.
