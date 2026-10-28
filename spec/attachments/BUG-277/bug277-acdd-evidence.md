# BUG-277 ACDD evidence

## Final validation receipts

- Current-source worker lifetime **5/5** and cold read integrity **2/2** pass in
  `/tmp/bug275-277-agent-owned-final.log`. The twelve-target agent run is **49/49**,
  including actual direct/server abandonment, queued read cancellation,
  default-false mutating-tool policy, real error preservation, original/blob
  identity and hash controls, captured compaction and prompt ownership.
- `/tmp/bug275-277-core-final.log`: **26/26** on final core source across tail
  recovery/property tests, shared index/cross-process compatibility, coherence,
  concurrent index publication and index recovery. Transient EINTR is retried only
  after a fresh cancellation checkpoint; cancelled cold discovery never caches a
  partial session list or false persistent unreadable diagnostics.
- `/tmp/bug277-responsive-final.log`: **2/2**, single-async-worker heartbeat and
  hook-denial controls. No live history, credentials, model egress or daemon was
  used for these measurements. Bounded worker-exit and competing-writer barriers
  are synthetic lifetime/occupancy evidence, not a release latency SLA.
- Final shipping source: actual default `codelet-fspec --bin fspec` build,
  check and Clippy pass in `/tmp/bug275-277-{binary,binary-check,clippy}-final.log`.
  NAPI check and Clippy pass in `/tmp/bug275-277-napi-{check,clippy}-final.log`,
  returning to **7** warnings after removing the 16 newly orphaned duplicate
  helpers/imports. Remaining RPC/NAPI warnings are in untouched code; no lint
  suppressions added. Both source front doors use one canonical handler.
- Controller approved bounded removal of the obsolete NAPI duplicate. Its 35
  helper tests were exact canonical duplicates before cleanup (only persistence
  import path differed). All 35 remain in the canonical module and now pass in
  `/tmp/bug277-canonical-helper-tests.log`. One obsolete prose predicate was
  updated to verify the actual inclusive 0-based show/search/originals contract,
  instead of requiring the literal word "turn". It now uses the existing Tokio
  test runtime rather than constructing another runtime. Assertions strengthened,
  no production schema or history policy changed.
- Native coverage audits: cooperative search **10/10 files valid**, read integrity
  **4/4 valid**. Both features have @done rather than @wip. Native check of the
  exact four changed features with current registry and read-only board reference:
  **Gherkin PASS, tags PASS, format PASS**. Repository-wide tag validation separately
  reports 522 issues outside these features; none are modified by this work.
- Preservation: HEAD unchanged, no staged diff, all nine original attachment/doc
  hashes match the controller baseline; BUG-272 remains Testing with its original
  updatedAt. No commit, push, deployment, daemon restart, cache cleanup, additional
  operational session/subagent or model switch. About 17GiB free; 6GiB reserve.

## Replacement continuation

Native status now **validating**. Latest seven-target agent regression had
28 passes and one failure. The failing captured-compaction builder fixture
omitted mandatory invoking work_unit_id; added NONE to its synthetic real tool
arguments, retaining both requests, captured-prefix/live-suffix and persisted
message assertions. `/tmp/bug277-compaction-final.log`: **5/5 pass**. No attribution
or permission enforcement changed. Core/search regression logs and current
pipeline details are recorded in BUG-275's short continuation evidence.

## Controller integrity review corrections

- Inspected `/tmp/bug277-index-publication-red.log` before starting another
  command: terminal failure (one test, missing cold-publication checkpoints),
  no pending Cargo/rustc process. Existing direct/server and non-opted-in control
  passes remain in `/tmp/bug277-server-controls-green.log` (five scenarios).
- Returned natively to specifying, added the two concrete user-experience
  examples and acceptance scenarios, validated, entered testing, and wrote
  `bug277_search_integrity.rs` before either production correction.
- `/tmp/bug277-integrity-red.log`: **2 failing, zero passing**. Cancelling the
  final healthy manifest produces one cached UUID instead of the original two
  on the subsequent unscoped listing **without reset**. Both helper readers
  independently report `Err(Interrupted)` on a single transient EINTR.
- Native implementing gate rejected two test files for one feature. Followed
  the gate's design intent, not a bypass: relocated the two already-specified
  integrity scenarios unchanged into `history-read-integrity.feature`, with the
  dedicated integration file as its sole mapping. Worker-lifetime retains its
  five original scenarios and sole integration file. Seven scenarios total.
  Native generate-coverage in the approved tiny project updates only these
  BUG-277 sidecars; byte-exact copies preserve all existing mappings. Native
  links and validation accepted; implementing transition then succeeded.
- Fixed cancellation classification by checking after `read_candidate` and
  before treating its result as a diagnostic or publishing discovery results.
  A failed cold constructor therefore cannot install the partial cache. The
  test also preserves unchanged healthy bytes and a real malformed-manifest
  isolation control; no false persistent unreadable issue is permitted.
- Both read helpers now retry only an **underlying** EINTR via their outer loop,
  with a fresh cancellation checkpoint. Checkpoint cancellation is outside that
  retry match. Tests cover complete/delimited bytes, remainder, and a reader that
  cancels during EINTR (exactly one actual I/O attempt; no cancellation spin).
- Added bounded derived-index publication checkpoints (256 entries) after the
  genuine cold-publication RED. They run inside the scoped read owner, before
  atomic promotion; ordinary durable writers have no cancelled read context.
  The recovery test rejects partial promotion and reopens the intact original.

Current run `/tmp/bug277-regression-agent.log` (process `0ad7928d`): isolated,
offline ci-test/debug=0/jobs=2, explicit agent-loop test-support targets worker
lifetime, read integrity, BUG-276 admission, provenance, original retrieval,
original blob integrity and compaction evidence lifecycle. No GREEN/quality claim
for these latest corrections until the command completes. Disk ~17.5 GiB free.

## Current implementation checkpoint

- Native status **implementing**. Five scenario tests and native coverage links
  exist. Test ranges need refreshing after the server-control extension.
- `/tmp/bug277-drop-red.log`: unchanged-production real lease abandonment RED.
  `/tmp/bug277-boundaries-red.log`: **4 failing, 1 passing** after adding only
  read-cancellation/observer scaffolding, before wiring any handler/storage read
  paths. Startup returned success; batch checkpoint absent; worker retained lease;
  byte checkpoints absent. Normal behavior control already passed.
- Added invocation-local ReadCancellation and drop guard; synchronous worker
  context carries it into actual core reads (no session-global competing owner).
  Mutex/sidecar lease acquisition retries every 5ms only inside such a context;
  normal writes retain blocking ownership. Message reads release global ownership
  per reference and check batches of 32. Reads use 64KiB byte checkpoints, JSONL
  scanning uses the BufReader's bounded chunks, index reads check every 256 entries,
  blob hashing checks chunks. Existing manifest snapshot references, per-message
  peer generation refresh, identity/hash verification and original errors remain.
  Handler checks precede action, session/message/blob/context work and publication;
  serialization has pre/post checks on the blocking worker. NAPI factory forwards
  to the canonical agent-loop handler instead of duplicating the repair.
- `/tmp/bug277-first-green.log`: initial **5/5**. Review then established that
  Rig ToolServer keeps execution after its caller drops, so direct-tool success
  alone was insufficient. Extended the existing scenario with both direct and
  real ToolServer calls; `/tmp/bug277-server-red.log`: **one genuine server-path
  failure**, direct path passes first. The process completed before later builds.
- Added a narrow default-false `cancel_on_abandon` policy in the patched Rig tool
  traits and service task; only SessionSearch opts in. Reply-channel cancellation
  is selected first for queued and active read calls, which drops the actual tool
  future and triggers the worker guard. No universal mutating-tool abortion.
  Native architecture note records this additional actual-path requirement.
- `/tmp/bug277-server-controls-green.log`: **5/5**, including direct/server lease
  abandonment, queued read abandonment before handler dispatch, and a synthetic
  non-opted-in counter/latch control. Abandoning the control's caller still allows
  the latched operation to complete once; a later genuine error is returned intact.
  Original attribution/admission/cancellation tests also pass in BUG-276's final
  post-Rig default/featured logs (7 default, 8 featured, 5 admission, 3 provenance).
- Additional cold-derived-index cancellation extension is now in the byte-read
  scenario, running RED in `/tmp/bug277-index-publication-red.log`. Do not claim
  it passes. Derived index publication also needs bounded cancellation checkpoints
  so a read-triggered rebuild cannot keep running after a completed scan. Normal
  durable writes must never inherit cancelled read ownership.

Remaining: finish that extension, refresh native coverage, run actual search/
originals/blob/coherence/responsiveness and cancellation controls, default shipping
build/NAPI/Clippy on current source, manual review and joint synthetic metrics.
No current-source quality/shipping pass for BUG-277 is claimed yet. New files are
under 300 lines. Current disk ~18GiB available, six GiB reserve preserved.

## Scope and preservation

Continuation of the same authorized three-card task at initial HEAD
`e486b5bf9af3ec55ad0c55aa56e3ee8b5c499067`. See BUG-276 evidence for the
preexisting source/attachment baseline, isolated build environment and strict
6 GiB disk reserve. No daemon, credentials, live history, deployment or new
sessions/subagents are used. No repository cache cleanup is authorized.

## Native specification

Read the existing card, all rules/examples/notes, original profiling/fix plans,
responsive-session-search feature/coverage and current tools/agent-loop/core
reading, lease, blob, framing and index paths. Native user story and generated
context were refined into five scenarios covering abandonment while waiting,
bounded message batches, startup/session boundaries, byte/blob cancellation,
and unchanged uncancelled behavior. Native format and validation pass; estimate
8, native status testing. No production repair has begun.

First actual-worker regression is
`rust/agent-loop/tests/bug277_search_worker_lifetime.rs`, using a real tiny
temporary history and real unavailable `messages.lock` lease. A wrapper signals
the blocking handler's actual return, independently of dropping the tool future.
It always releases the fixture lease and awaits worker cleanup before asserting.
RED execution queued behind BUG-276's resource-bounded validation pipeline,
log `/tmp/bug277-drop-red.log`. No RED or GREEN result is claimed yet.

## Intended repair boundaries

Invocation-owned cancellation must outlive handler-map removal and reach the
actual blocking worker on future drop, even before start. Read-only worker
context can propagate checkpoints to core without changing all existing handler
signatures or adding a competing session-global cancellation owner. Read loops,
byte reads and lease retries must check cooperatively; durable writes must not
inherit read cancellation or be interrupted midway. Preserve manifest reference
boundaries, existing peer-generation refresh, identity/hash checks, original
errors, routing, hooks and trimming. Bounded helper extraction only; no wholesale
storage redesign or unrelated oversized-module refactoring is authorized.
