# Fixed-window backlog inventory

Window: **2026-10-25T07:47:53Z–2026-10-25T08:47:53Z**, inclusive UTC.
Verified with native `show-work-unit --format json` for all thirteen cards,
then a read-only all-card scan of canonical `spec/work-units.json`, including
every recorded `stateHistory` transition into backlog. The native query command
offers status/type/tag filters, not a full historical interval query.

All thirteen were present in `states.backlog`, with status `backlog`, and no
stateHistory entries at discovery. Creation-in-backlog evidence is the native
createdAt plus canonical backlog membership and absence of subsequent state
history. No additional creations or recorded backlog transitions fall in the
window. No near-boundary records were identified. No updatedAt filter is used.

| Order | ID | Title | createdAt (UTC, 2026-10-25) | Dependencies / sequencing |
| --- | --- | --- | --- | --- |
| 1 | BUG-259 | Prevent transient recovery-lock contention from interrupting streaming during shell spawning | 08:35:26.734Z | Independent; preserve BUG-253/255/256/257/258 ownership and recovery fixes |
| 2 | BUG-260 | Preserve Codex refusal history across follow-up requests | 08:36:07.388Z | CONT-011 child; before typed replay policy |
| 3 | BUG-261 | Preserve retryable network failures during Codex forced token refresh | 08:36:18.880Z | CONT-011 child; independent Codex recovery fix |
| 4 | BUG-262 | Preserve streamed OpenAI tool-call order before accepted Done | 08:36:31.077Z | CONT-011 child; before terminal composition |
| 5 | BUG-263 | Require honest OpenAI Chat terminal evidence before completion | 08:36:45.102Z | CONT-011 child; follows ordering repair |
| 6 | BUG-264 | Preserve OpenAI Responses failed and incomplete terminal semantics | 08:36:57.140Z | CONT-011 child; before Copilot dialect integration |
| 7 | BUG-265 | Preserve continuation budgets and goal rejection counts across recovery dispatch | 08:37:09.257Z | CONT-011 child; shared-loop accounting |
| 8 | BUG-266 | Settle custom provider stops without dropping trailing usage or waiting for EOF | 08:37:20.793Z | CONT-011 child; reconcile existing post-stop error contract |
| 9 | BUG-267 | Dispatch built-in-name scripted providers through their selected custom implementation | 08:37:33.975Z | CONT-011 child; typed dispatch identity |
| 10 | BUG-268 | Connect Copilot endpoint selection to actual provider requests | 08:37:45.161Z | CONT-011 child; follows BUG-264 |
| 11 | BUG-269 | Wire scripted OAuth automatic refresh into real completion requests | 08:37:57.964Z | CONT-011 child; preserve script-owned refresh |
| 12 | PROV-162 | Reconcile Codex protocol continuation with typed response history | 08:38:11.400Z | Related to CONT-011; approved policy needed before relaxing explicit restrictions |
| 13 | CONT-011 | Restore end-to-end auto-continue provider contracts and regression coverage | 08:35:45.372Z | Coordinator: close after child and affected-provider regression matrix |

There are no explicit dependsOn/blockedBy edges on these cards. Parent/child
and relatesTo edges do not authorize work on outside cards. CONT-010 and
PROV-161 are read-only context. BUG-254 remains specifying; Windows setup is
excluded. BUG-253/255/256/257/258 remain done.

Baseline: clean working tree at `b2475dcd`; that card-registration commit is
unpublished relative to the configured upstream. Runtime/model selection is
unchanged; no additional agent sessions are permitted. Initial free disk: 20 GiB;
builds must remain offline, scoped, ci-test, jobs=2, with a strict 6 GiB floor.

Evidence policy: each card retains its own real red/green output, coverage and
review evidence. This inventory establishes scope, not implementation completion.

Latest replacement evidence supersedes the historical execution milestones below:
BUG-259–269 and PROV-162 are native Done. CONT-011's final source, SDK import
correction, original-scope disk readback and complete combined review are recorded
in [validation and review](validation-and-review.md). Closure is established only
by the subsequent native Done and revision-bound final audit, not old progress
sections or a source-reader pass. BUG-254 and earlier Done cards stay unchanged.

## Execution milestones

- BUG-259: native Done verified. Red 1 passed / 3 failed; green 4 passed;
  affected matrix 26 passed, two subprocess fixture entry points ignored by
  their normal runners and exercised through child processes. Scoped check,
  Clippy, format, Gherkin/tag validation and coverage audit passed. Evidence:
  `../BUG-259/validation-and-review.md`. Feature has @done and no @wip.
- BUG-260: native Done verified. Red 1 passed / 2 failed; green 3 passed;
  affected lineage/terminal/reload/fork matrix 32 passed. Scoped check, Clippy,
  format, Gherkin/tag validation and coverage audit passed. Evidence:
  `../BUG-260/validation-and-review.md`. Feature has @done and no @wip.
- BUG-261: native Done verified. Final shared-fixture matrix 13 passed; required
  final-source regressions 11 passed plus 3 boundary confirmation passes. First
  boundary run had a retained 150 ms timing failure. Invalid external-routing
  fixture and UNKNOWN transmission remain disclosed. Approved local-only seam,
  genuine causal reds and final quality are in `../BUG-261/validation-and-review.md`.
  Held blocked transition was not retried or bypassed.
- BUG-262: native Done verified. Mechanical split with four passing provider
  controls; un-attributed and unregistered-context fixture runs are INVALID.
  Corrected normal attribution/controller: actual native Done accepted, causal
  red 2 failed / 1 JSON control passed; final source green 3/3. Affected controls
  total 24 passed / 1 unchanged CONT-006 source-location test failed, owned by
  this coordinator's stale-layout cleanup. Final Clippy/check/format and native
  spec/tag/coverage checks pass. See `../BUG-262/validation-and-review.md`.
- BUG-263: native Done verified. Five causal reds; all five final-source tests
  pass, including actual billed native Done and no-EOF DONE settlement. Affected
  matrix 26 passed / 1 known stale auto-continue source-layout assertion failed,
  left for coordinator cleanup. Native spec/tag/coverage and scoped quality pass.
  Evidence: `../BUG-263/validation-and-review.md`.
- BUG-264: native Done verified. Red 4 failed / 1 passed; mechanical split control
  passed; terminal matrix 13 passed plus dedicated Codex controls 10 passed.
  Final-source five tests, scoped Clippy/check/format and native spec/tag/coverage
  checks pass. Evidence: `../BUG-264/validation-and-review.md`. Dedicated Codex
  policy is unchanged; only its shared-final initializer gets a None default.
- BUG-265: native Done verified. Approved full mechanical extraction and runtime
  repair complete. Red before and after pure extraction: five failures/two
  controls; final seven acceptance tests plus 33 controls pass. Test-source
  witnesses compile and nine safe shape tests pass; strengthened dual-path
  dispatch guard passes. Five nested Cargo shape cases and the NAPI nested-check
  scenario remain excluded. CMPCT-041 reaches the moved accessors then fails a
  stale NAPI-reader predicate; focused immutable source proof is retained, not a
  full baseline Cargo run. Global tag validation reports 522 unrelated feature
  violations; native BUG-265 file-scoped tags pass. See final
  `../BUG-265/validation-and-review.md`, not the historical preparation logs.
- BUG-266: native Done verified. Approved content-stop/authoritative-transport
  boundary and mechanical type extraction preserve BUG-227 typed failure policy.
  Four causal provider reds/two controls plus actual admitted native-Done billing
  red precede repair. Seven acceptance tests and affected controls total 37
  distinct passes. Final check/Clippy/format and native scoped validation pass.
  An explicitly approved native duplicate-mapping reconciliation repaired the
  held transition without weakening gates; exact evidence is retained in
  `../BUG-266/validation-and-review.md`.
- BUG-267: native Done verified. Approved bounded no-override typed Custom
  dispatch, explicit facades unchanged. Two genuine shared-loop reds/three
  controls precede production. Five acceptance tests cover fifteen selection
  cases (fourteen successful local requests, one fail-closed incompatible
  override), plus 23 controls: 28 distinct tests pass. Default check, scoped
  Clippy/format and native spec/tag/coverage/review pass. See
  `../BUG-267/validation-and-review.md`.
- Remaining four selected cards: BUG-268, BUG-269, PROV-162 and CONT-011 are
  natively verified backlog, awaiting sequential execution; coordinator last.

The controller has cleared the preparation-only hold following the human's exact
approved project Rust clean. This session performed no cleanup, and the separate
old Documents/Codex target was not deleted. Offline bounded compilation resumed;
last observed available disk is 8.0 GiB against the strict 6 GiB floor. The user
now authorizes repeated Cargo clean only in the project rust directory when disk
blocks progress, executed by the supervisor after verifying no active writes;
this is not permission for agent cleanup or deletion of the separate Documents
target. Additional bounded
read-only discovery is in `remaining-source-preparation.md`; it changes no
remaining card's criteria or policy and does not substitute for runtime proof.

No new session, subagent, model selection, daemon restart, installation, security
policy change or git commit has been performed. No outside work-unit record has
been changed. The batch is not complete; BUG-259 through BUG-267 are native Done.
