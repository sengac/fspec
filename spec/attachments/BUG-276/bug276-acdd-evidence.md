# BUG-276 ACDD evidence

## Joint final-source verification

Native card remains Done without reopening. After BUG275/277 final production
changes, `/tmp/bug275-277-agent-owned-final.log` passes all 49 scoped agent-loop
tests, including **5 admission**, **3 provenance** and **8 featured RPC088** cases.
`/tmp/bug275-277-default-final.log` passes all **7 default RPC088** cases and the
five settlement controls. Final shipping default fspec binary build/check/Clippy
and NAPI check/Clippy pass; NAPI's 16 newly orphaned helpers were removed under
controller approval, leaving the original seven warnings. See BUG275/277 evidence
for final receipts, failure corrections and preservation audit. No status or source
changes were made to BUG272; no deployment, daemon restart or commit occurred.

## Final BUG-276 validation (replacement worker)

### Final narrow persistence review (supersedes six-default-test checkpoint)

Reopened legitimately once more for the controller's narrow review. The durable
partial-output test and its BackgroundOutput/StreamOutput/originals imports and
cold-read helper are now unconditional. It uses `session_fixture()` without a
streaming hook, stub registration or inference. Only the genuinely streaming
new-turn admission case requires `test-support`. Typed user reason, queued input
values, cold original count and partial-text assertions are all retained, as
are the two-session independence and existing cancellation/attribution controls.
No shipping feature default changes were made.

Verified actual final commands (same isolated/offline/ci-test/debug=0/jobs=2):

- `/tmp/bug276-rpc088-default-final.log`: **7 passed, 0 failed**, including durable
  partial output (7.58s compilation, 0.38s execution).
- `/tmp/bug276-featured-final.log`: featured RPC088 **8 passed**, prompt admission
  **5 passed**, provenance **3 passed**, zero failed; provenance actually executes
  both initiating causes with original valid attribution and auxiliary admission.
- Direct rustfmt check of both edited test/helper files and `git diff --check`
  pass. Earlier six-default/eight-featured logs describe older test states only.
  Intermediate default-import compile failures were corrected, never counted.
- BUG-277's existing server RED was confirmed terminally completed before this
  command sequence; no Cargo build was duplicated or overlapped. No further
  BUG-277 production edits were made while resolving this final review.

The full shipping/NAPI/Clippy checks below remain evidence of their exact older
source state. Joint current-source checks are still required after BUG-277/275.

### Controller default-path review correction

Reopened BUG-276 natively from Done to Implementing for the controller's review.
Removed RPC088's file-wide `test-support` gate. Only the stub-stream admission
and persisted-output fixtures are gated; constructor/handle isolation, notifier
wake, all three provider-forwarding guards and interruption serialization remain
enabled by default. Shared fixture setup now separates a no-provider-I/O session
constructor from the optional stub hook, without duplicating filesystem setup.

The isolation test constructs **two distinct sessions** in one synthetic data
directory, checks different flag/notifier Arcs, interrupts the first, verifies
the second's flag/epoch/reason remain untouched and that its notifier does not
wake, while the first notifier does. No cancellation/attribution assertions
were removed. An initial test-edit compile failure was corrected, not counted.

- `/tmp/bug276-rpc088-default.log`: actual default-feature target **6 passed**,
  zero failed, not a zero-test pass (8.30s compile, 0.05s execution).
- `/tmp/bug276-review-featured.log`: `test-support` RPC088 **8 passed**, admission
  **5 passed**, provenance **3 passed**, zero failed (18.66s compile).
- Both commands retain offline, explicit package/target, ci-test/debug=0/jobs=2
  and isolated HOME/XDG settings. Direct rustfmt check passes after these edits.
  These new runs, not the earlier 16/6 counts, validate the review correction.
  Production shipping/NAPI checks must be repeated after the later cards'
  production edits as part of joint final validation.

All commands use the isolated HOME/XDG paths documented below, offline Cargo,
`--profile ci-test`, `CARGO_PROFILE_CI_TEST_DEBUG=0`, jobs=2, and explicit package
and target selection. Production source did not change between these checks.

- `cargo test -p codelet-cli --test bug214_compaction_cancellation`: **6/6**,
  `/tmp/bug276-compaction.log`, including automatic/manual recovery cancellation,
  admission veto, late candidate and committed-generation controls.
- `cargo build -p codelet-fspec --bin fspec`: **PASS**, default production
  features, `/tmp/bug276-default-build.log` (6m05s). This actually links the
  shipping RPC/embedded server, CLI, TUI and fspec binary; no deployment/run.
- `cargo check -p codelet-napi --lib`: **PASS**, default/full runtime,
  `/tmp/bug276-napi-check.log` (3m50s), seven warnings outside edited constructor:
  six unused legacy persistence/callback functions and ignored `clear_history`
  result in `session_bindings.rs:1555`. No unrelated warning repair.
- `cargo clippy -p codelet-fspec --bin fspec`: **PASS**,
  `/tmp/bug276-clippy-binary.log` (2m50s). Dependency warnings are retained.
- `cargo clippy -p codelet-napi -p codelet-agent-loop -p codelet-sessions --lib`:
  **PASS**, `/tmp/bug276-clippy-libraries.log` (59s), same existing NAPI warnings.
  An earlier malformed command repeated `--lib`; Cargo rejected it without
  running Clippy (`/tmp/bug276-clippy.log`); only corrected passes count.
- Final post-format `test-support` agent-loop targets provenance, admission and
  cascade: **16/16**, `/tmp/bug276-final-tests.log`. Earlier selected controls add
  21 passing AgentManager/recovery/retry cases, not the seven unrepaired failures
  in their original log. Six old structural failures have exact-HEAD static
  baseline evidence; the provenance timeout has causal fixture evidence and
  real repaired runtime coverage, not an unsupported baseline-runtime label.
- `cargo fmt -p codelet-agent-loop -p codelet-sessions -p codelet-napi -- --check`
  finds only an untouched existing formatting difference in
  `napi/tests/system_reminder_restoration_test.rs:391`; edited files pass their
  direct rustfmt check. No unrelated formatting was applied.
- Native feature/tag validation, coverage audit (12/12 referenced files) and
  review pass (five scenarios, 100%, no findings). Added native sender and epoch
  implementation links. Reviewed all constructor, queue, retry and manual
  continuation paths, atomic epoch admission under interruption mutex, awaited
  startup persistence, checks after startup waits and before provider dispatch.

Limits: not every integration binary or feature combination in the workspace
was run (resource constraint and explicit scope). No live/provider inference,
NAPI Node-host execution, release-profile SLA, install/restart or deployment is
claimed. Test-support paths execute real stub streaming; default shipping code
and edited full NAPI consumer compile. Free space at final check: ~19 GiB,
above the strict six GiB reserve; no cleanup occurred.

BUG-277 is now natively testing, with five specified scenarios and tests; the
first unchanged-production abandoned-worker RED is recorded separately. BUG-275
has native story/scenarios/estimate prepared and remains specifying. Neither
later production repair is part of this BUG-276 validation snapshot.

## Source and preservation baseline

- Initial HEAD: `e486b5bf9af3ec55ad0c55aa56e3ee8b5c499067`.
- Initial working tree: modified `spec/work-units.json`; untracked
  `docs/session-search-debug-profiling.md`, BUG-272 follow-up and shared profiling
  attachments, and BUG-275/276/277 attachment directories. Preserve all originals.
- BUG-272 was read natively: canonical state **testing**, existing five scenarios;
  no reopening, transition or source repair authorized for that card.
- BUG-275/276/277 began **specifying** with related-to links only, no blocking
  dependencies. Native cards, attachments and dependencies read in full.
- Root AGENTS, bootstrap (3004 lines), TESTING, FOUNDATION and TAGS read.
  No nested AGENTS found. No extra sessions, subagents or model changes.

## Native front door

Attempting to register a nested `fspec client --mcp` failed at
`fspec_rpc_checkpoint_diff_files` because business `work_unit_id` conflicts with
harness attribution. Controller clarified that its existing outer MCP connection
already fronts this agent and the built-in `fspec` tool is the intended native
front door. Built-in `fspec` works, including native gates. No schema repair,
CLI mutation fallback, daemon restart or installation was performed.

## Specification and tests

- Set user story natively, generated `prompt-admission-cancellation.feature`
  context, refined its five scenarios as instructed by generation output.
- Native tags added: agent-core, session-management, phase-1, interrupt, wip.
- Native feature validation passed. Estimated 8; native transition to **testing**.
- Five regression tests authored in `bug276_prompt_admission_cancellation` with
  shared temp helper and deterministic local stub. Queue, hook, persistence lock,
  persistence failure and supervisor/recovery ownership scenarios.
- No production repair yet. No RED/GREEN outcome claimed at this checkpoint.

### RED verification

`/tmp/bug276-red-bounded.log` records genuine current-source RED: four failing,
one passing. Queueing dispatches twice instead of once; hook and storage
cancellation each dispatch a cancelled prompt; stale supervisor enters history.
The persistence-failure recovery control passes.

The initial test run required interrupting its owned test process because the
test's storage-barrier timer shared the blocked runtime. The bounded version
uses an independent controller thread to release the real message-store lease
even when the runtime stalls, and primes the empty store before holding it.
Its heartbeat was strengthened to execute on the sole Tokio worker rather than
on the test driver. Final strengthened RED run: `/tmp/bug276-red-final.log`.

Native per-feature tag validation passes. All five scenarios have native test
coverage links; final line mappings must be refreshed after formatting, removing
old test mappings first because link-coverage adds rather than replaces ranges.
Controller approved isolated native coverage generation to avoid unrelated
sidecars: `/tmp/bug275-277-coverage`, transfer exact native-generated sidecar only.
No JSON was hand-authored, skipped or forged.

## Build/resource boundaries

No Cargo processes initially, `rust/target` absent, no environment target override,
workspace config jobs=4. Task commands explicitly use offline, ci-test, debug=0,
jobs=2, package and integration target. Initial free space about 29 GiB;
six GiB reserve mandatory, no repository or cache cleanup permitted.

Initial RED compilation was interrupted before executing tests to make HOME
isolation explicit. It only compiled dependencies. Restarted with:

```sh
HOME=/tmp/bug275-277-home.Fn1mBd \
XDG_CONFIG_HOME=/tmp/bug275-277-home.Fn1mBd/config \
XDG_RUNTIME_DIR=/tmp/bug275-277-home.Fn1mBd/runtime \
CARGO_HOME=/Users/rquast/.cargo RUSTUP_HOME=/Users/rquast/.rustup \
CARGO_PROFILE_CI_TEST_DEBUG=0 \
cargo test --offline --profile ci-test -j 2 -p codelet-agent-loop \
  --features test-support --test bug276_prompt_admission_cancellation
```

Cargo dependency/toolchain roots are retained only for offline compilation;
test storage uses explicit temporary data directories. Output:
`/tmp/bug276-red-isolated.log`. Build continues in tool process `a11f01d1`.
Latest disk observation before restart: 29,602,648 KiB available; target 1.0 GiB.
No live message store or provider jobs used.

## Discovery notes for repair after genuine RED

`send_input` currently resets the shared interrupt flag before queueing;
`admit_prompt` resets it again after synchronous persistence. New user input can
therefore revive cancelled queued/active input. Existing interrupt epoch can own
queued prompts and supervisor messages; only admission of a matching new-input
epoch should reset the flag under the interruption-reason mutex. Retries retain
their original epoch; manual compaction and detector retry checks already exist.
Persistence must be awaited off the async worker, never abandoned mid-transaction.
Recheck ownership after hooks/persistence/inner-lock waits and before provider
dispatch. Existing active stream must not be revived by later `send_input`.

The old `rpc088_interrupt_cascade` includes source-shape assertions for the faulty
reset position and older unsplit source paths. Record actual failures before
reconciling that contract; do not weaken it or claim a baseline without running.

One erroneous native `format` call supplied `files` instead of supported `file`;
it formatted the project and changed five unrelated feature files. All five
format-only differences were reversed immediately with patches; subsequent
status confirmed only original user paths plus the new BUG-276 feature.
Future format calls must use singular `file`.

## Implementation checkpoint

Native status is now **implementing**. Initial epoch repair was tested, corrected
for a misplaced pre-persistence cancellation check (which lost cancelled queued
originals), and the supervisor regression was strengthened while in testing:
queue an additional supervisor message while interruption is already set, then
submit new explicit user input. `/tmp/bug276-supervisor-red.log` shows **four
passing, one failing**; stale supervisor work was still entering provider history.

The subsequent repair captures supervisor epoch plus cancellation-at-submission
under the interruption mutex. A `None` supervisor epoch is explicitly cancelled,
not recaptured at drain time. Explicit user input captures epoch before output or
status callbacks; only matching input admission clears the flag. Both direct
supervisor enqueue and idle-only AgentManager admission stamp target ownership.
PromptInput/manual continuation constructors and IncomingMessage literal users
in both agent-loop and NAPI handlers are wired.

Cancelled queued user input persists before interruption is finalized. Startup
persistence is awaited on the blocking pool, followed by an ownership check.
Additional checks follow inner-session startup waits and compactor retry setup.
Automatic recovery retains its admitted turn epoch; detector/manual continuations
retain their existing explicit epoch checks.

Controller approved necessary narrow constructor edits in preexisting oversized
modules plus bounded helper extraction required by these cards, not wholesale
splitting or a major storage redesign. Controller specifically requires the
already-interrupted supervisor case, deliberate enqueue ownership boundary,
exactly-one durable cancelled original, and all actual public consumer builds.

Current full regression command is unchanged from above, output
`/tmp/bug276-green.log`, running tool process `d5c2e9b1`. Do not claim success until
reading its result. Current available space ~25,957,084 KiB; target 3.6 GiB.
No other builds run concurrently. No cleanup performed.

## Replacement-worker causal controls

- Root AGENTS, card evidence, all three native cards/plans and current source/
  acceptance/coverage read. No sessions/subagents created or old session resumed.
- `/tmp/bug276-rpc088-baseline.log` executes the **unchanged original predicates**
  compiled with `rustc --test` against exact HEAD bytes in
  `/tmp/bug276-rpc088-head.0udBst`. All six structural failures reproduce, one
  structural census passes. Runtime wake test is deliberately cfg-excluded from
  this lightweight baseline; no baseline runtime success is implied.
- `/tmp/bug276-provenance-diagnostic.log` identifies the timeout causally:
  `interrupted=false`, `dropped=false`, one local transport request, and both
  tool results reject missing `work_unit_id` before invoking the interrupting
  tool. This is not a timeout extension or an unsupported baseline claim.
  Fixture now registers auxiliary admission and supplies `work_unit_id: NONE`.
  The same original three assertions/scenarios pass, including both actual
  initiating causes and exactly two outcomes. Diagnostic timeout context retained.
- Replaced obsolete RPC088 reset-location and persistence-name predicates with
  real queued cancellation/new-turn admission, persisted partial assistant content,
  typed reason, actual notifier wake, and constructor/serialization checks.
  Provider forwarding checks follow current OpenAI/custom and final macro call
  sites; canonical specification no longer requires the faulty blanket reset.
- `/tmp/bug276-controls-repaired.log`: **16 passed, zero failed** across provenance
  (3), prompt admission (5), and updated interrupt cascade (8). Original recovery,
  retry and AgentManager controls remain in `/tmp/bug276-controls.log` (21 passes).
- Current shipping build, NAPI check, Clippy and six compaction controls running
  serially under the same isolated offline ci-test debug=0 jobs=2 environment;
  logs `/tmp/bug276-{compaction,default-build,napi-check,clippy}.log`. No success
  claimed until completed. Other cards remain specifying, unchanged.

Remaining work includes strengthening any missing startup/abort settlement
controls, refreshing native test and implementation coverage ranges, causal
cancellation/recovery/compaction controls, current-source default RPC/CLI/TUI/fspec
compilation and Clippy, supported feature builds, review, native validation/Done;
then implement BUG-277 and BUG-275 in that order and joint validation. Neither
later card has been modified by this session.

### GREEN milestone and causal controls

`/tmp/bug276-green.log` confirms **5 passed, 0 failed** on current repaired
source. It includes the strengthened already-interrupted supervisor case and
the single-worker heartbeat while the real message-store lease is held.
No current-source default consumer or Clippy pass is claimed yet.

All five native coverage ranges were refreshed (old mappings removed first),
each links its exact scenario test to `agent_loop/prompt.rs:8-163`.
Additional implementation links for sender/epoch ownership remain useful.

An isolated offline ci-test debug=0 jobs=2 control build is running as process
`432e38b1`, log `/tmp/bug276-controls.log`, package `codelet-agent-loop`, feature
`test-support`, explicit targets: bug220_loop_retry, bug220_interruption_provenance,
bug246_recovery_admission, bug246_recovery_boundaries, bug246_user_admission,
bug249_admission_ownership, agent_manager_busy_messages, rpc088_interrupt_cascade.
It uses `--no-fail-fast`. Do not duplicate this build.
