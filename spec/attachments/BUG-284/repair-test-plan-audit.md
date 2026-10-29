# BUG-284 acceptance and causal RED audit

Audit snapshot: 2026-10-28T21:25:45.487933+00:00. Read-only; no source edits, builds,
native calls, approval attempts, or indirect implementation were performed.

## Current result

The feature `spec/features/durable-input-admission.feature` has 13 scenarios and
states the main admission invariant accurately. The design explicitly preserves
strict provider lineage and treats canceled originals as retained receipts
outside admitted conversation membership. It correctly refuses silent legacy
migration and separates admission from remote execution/completion.

Eight test functions are now present in `bug284_durable_input_admission.rs` and
its `bug284/{fixture,retention,cancellation,boundaries,historical,setup_failure}.rs` modules.
No compiler or execution output was reviewed here; this is a static audit, not
certification of an executed RED. The central six/two/cancellation composition
is materially sound, with the specific coverage defects below.

Exact source hashes and all eight function declarations are captured in
`/tmp/compaction-investigation/repair-fixtures-review/plan-audit-snapshot.json`.
The source was changing during preparation; validation must use the build owner's
frozen test/source receipts for the actually executed revision.

## New test implementation audit

**Valid causal composition.** `Harness::seed` obtains a real Codex checkpoint
through `run_agent_stream`, a loopback transport, EffectTool and native
BackgroundOutput. Canceled inputs then travel through `receive_incoming_message`
or `send_input`, actual queue receivers, `agent_loop` and prompt admission.
`Harness::compact` captures the actual durable compaction source and invokes
`commit_selected_compaction`, rather than fabricating the desired requested
history. On rejection it checks unchanged manifest/checkpoint and a lineage
error. The untouched accepted-history control independently compacts, which
rules out an always-rejecting seed/preparation setup. This is an appropriate
narrow first RED once compiled and executed.

**DAG control is currently mislabeled.** In `historical.rs`, the `ancestor` case
selects the first `role == user` item and changes its text. It is a generic user
ancestry mutation. All four source histories have no committed compaction, and
the tokenized fixture contains no established DAG ancestor. It cannot prove DAG
integrity. Rename it to user ancestry and add a separate genuine commit/restore/
continuation fixture that mutates the established DAG, following the corrected
BUG-282 negative fixture. Do not repeat the prior mistake of calling an arbitrary
message mutation DAG coverage.

**Seed request count is not a dispatch monitor.** `retention::retained` checks
that the seed server saw two requests. That fixture server ends its accept loop
after its two scripted responses, and the actual queued agent-loop provider
factory is not wired to this seed client/server. Thus `len() == 2` cannot by
itself prove zero canceled provider dispatch. Runtime absence, queued count,
no-error checks and effect-file preservation are useful but weaker than an actual
provider-attempt/dispatch observer attached to the queue's provider factory.
Ensure the isolated fixture cannot contact any real endpoint.

**The retention helper encodes the known pre-fix membership mistake.** It looks
for canceled originals through `get_session_messages_full(manifest)`. The design
intentionally moves receipt-only originals outside that conversation membership.
When implementing the schema, migrate this assertion to typed receipt retrieval,
checking exact UUID/content/blob evidence and Cancelled disposition, and assert
those UUIDs are absent from admitted membership. Do not delete the recoverability
assertion to make GREEN. Current `runtime.contains(text)` is adequate only for
these simple labels; use typed content/identity checks when adding escaped,
identical-text or image cases.

**The runtime-lock barrier must migrate with the fix.** The current test holds
`session.inner`, then waits for `manifest.messages.len() == before + 1` before
canceling. This is useful to expose today's persist-before-runtime behavior, but
the intended fix retains Pending outside messages and waits for runtime ownership
before promotion. Correct implementation would therefore time out on the old
barrier. Move the barrier to confirmed retained Pending identity/lock-wait state,
keep the owner lock held until cancellation, and keep the original exactly-once
and replay assertions. Preserve the old executed RED receipt; document this
fixture adjustment rather than relabeling its timeout as a product regression.

**Narrow controls, not the whole matrix.** Eight tests currently correspond to
untouched control, six canceled, two canceled, hook cancellation, persistence
cancellation, runtime-lock cancellation, historical rejection, and a provider-setup failure case. Already-admitted
interruption, pre-prompt compaction failure, identity/image/version cases, lost ack,
process death and successive automatic compactions are not implemented in these
files. Manual `compact()` uses a supplied DAG and a no-op publication callback;
it does not prove the automatic trigger/compactor/cold-runtime installation path.

**Historical test bounds.** Its cloned checkpoint request is a useful positive
provider-lineage control, and the restored CodexHistory snapshot equality is a
meaningful no-mutation assertion. Comparing an immutable parsed `evidence` value
with its own serialization does not independently prove original historical
files unchanged; retain the controller's private evidence hashes for that claim.
The invocation mutation changes arguments only, not name. The first refusal of
the old 233+9 derivative stops at encoding divergence; add the partial-resolution
counterfactual if claiming the nine-message overlap is also exercised.

**Provider-setup case covers one branch only.** `setup_failure.rs` was added
while this audit ran. It arranges absent isolated Codex authentication and then
asserts any `StreamChunk::Error`, archive presence and runtime/replay equality.
This can expose persist-before-provider-setup disagreement, but it does not
exercise pre-prompt compaction failure despite its combined test/scenario name.
Require proof the error came from the intended provider-construction boundary,
not an unrelated setup or storage failure. Its `CODEX_HOME` environment lookup is
a fixture precondition: if the isolated runner does not supply it, that failure
is not behavioral RED. Its unchanged seed-server count has the dispatch-monitor
limitation already described above.

## Acceptance gaps relative to the proposed design

1. **Receipt/original/blob garbage-collection roots.** The design moves Pending
   and Cancelled originals outside `manifest.messages` and explicitly adds GC
   roots. None of the 13 scenarios actually runs message/blob GC and proves
   canceled receipts, images and pending recovery originals still resolve after
   cold reopen. This is a data-loss risk specific to the chosen architecture,
   not an optional broad regression.
2. **Acknowledgment and queue failure.** The design requires durable Pending
   before acknowledging submission. Existing process-death wording does not
   explicitly assert the call cannot acknowledge a volatile-only submission, nor
   cover full/closed queue rejection after retention. Add a barrier/failure case
   that observes the submitting caller and reopens storage before queue dispatch.
3. **Hook refusal, not just hook cancellation.** The design promises hook-refused
   originals are retained with a reason. Scenario 3 exercises cancellation while
   a hook is held; it does not prove `allow_prompt=false` retention and no
   admission. Preserve both paths without labeling refusal as user cancellation.
4. **All front doors.** The design changes regular, supervisor, MCP and internal
   retry origins. Scenarios emphasize supervisor and generic inputs; require
   explicit rows for MCP/internal continuation and retry identity. Do not let a
   supervisor-only implementation satisfy an invariant claimed for all callers.
5. **Actionable legacy diagnostics.** The last scenario only asserts rejection.
   The design promises an actionable recovery-required diagnostic and no guessed
   migration. Assert its typed classification and evidence IDs/hashes, while
   keeping private prompt text out of errors; a generic error alone is weaker.

The following combined scenarios must enumerate all their branches rather than
pass a single convenient one:

- provider construction **and** pre-prompt compaction failure (lines 79–83);
- conflicting UUID identity **and** unknown receipt version (85–89);
- lost acknowledgment after rename **and** persistently unavailable storage
  (91–95), distinguishing definitely refused from promoted-but-unacknowledged;
- process death at each listed pending/admitted/installed/dispatch boundary
  (97–101), including death before promotion versus after rename;
- every altered assistant/invocation/outcome/missing-evidence/DAG control
  (109–113), not one generic bad-history example.

The successive-compaction scenario is appropriately demanding: two **automatic** compactions, parallel
completed tool effects and cold continuations. Two direct calls to
`commit_selected_compaction` would cover durable publication but would not meet
its automatic-trigger wording. Record that gap explicitly if only direct
compaction is available during the initial RED stage.

## Requirements for a valid first compiled causal RED

A narrow RED for the first two scenarios is sufficient to establish this defect
before implementation; it is not evidence for the other ten scenarios.

- Start an isolated real Codex runtime/checkpoint with a genuine streamed tool
  call/result and once-only temporary effect. A helper may seed that initial
  conversation through the actual stream loop, but must not invent checkpoint
  items or declare that an unsubmitted user message was requested.
- Keep and drive the actual PromptInput receiver through production
  `codelet_agent_loop::agent_loop`, normal supervisor queueing and interruption.
  Exercise the six-before-cancel and two-arriving-while-interrupted cases.
  Observe/dequeue the expected messages; a test that directly appends six/two
  malformed envelopes only repeats an already-known synthetic mismatch.
- Verify the production path archived each original once, kept canceled inputs
  out of runtime/provider dispatch, and emitted truthful cancellation. The bug
  is the composition of these individually intended behaviors.
- Load the actual persisted manifest/envelopes through the production replay
  path. Invoke actual selected compaction preparation/commit against that
  durable history and the genuine checkpoint. Do not replace request history
  with `session.messages`, construct a handpicked wire request, manually erase
  canceled entries, or manually seed the desired admitted set.
- Verify preconditions before asserting failure: actual Codex provider/model;
  expected queue count and cancellation boundary; nonzero genuine checkpoint;
  real completed outcome/effect; exact six/two persistent-versus-runtime gap.
- Require the executed error to be selected-lineage ancestry rejection, with no
  promotion/publication or additional effects. Compilation errors, empty-source
  errors and generic assertion failures are not the causal RED.
- Freeze test and relevant source hashes with the executed command/exit/log.
  If production changes appear later, re-read those hashes rather than assuming
  the recorded RED used the same tree.

## Fixture provenance and what it does not prove

The prompt-free derivatives in
`/tmp/compaction-investigation/repair-fixtures-review/` preserve a bijection of
projected items, their equality graph, order, call identities, type and role.
Their source hashes and token maps bind them to the preserved failures. Their
first rejected indices remain 490, 425, 3 and 3. This is stronger than an invented
four-row fixture and avoids leaking private prompts.

However those derivatives are **post-projection**. They remove original stream
partitioning, real admission metadata, wall-clock scheduling, process boundaries
and encoding payload bytes. Their counterfactual omission counts (6, 2, and
233+9 / 165 for earlier sessions) are causal probes, not an implementation policy
or a positive runtime regression. The actual earlier encoding relationship is
proven separately by exact legacy-transform equality; opaque outcome tokens no
longer test that transform.

Use the derivatives to prove strict lineage still refuses known historical
corruption, and separately compare valid admitted history to ensure the harness
cannot pass by rejecting everything. The 233+9 overlap should reject after only
one defect has been resolved in the diagnostic clone; a baseline rejection at
index 3 alone never reaches or verifies the later nine-message discrepancy.

The behavioral RED must obtain its gap through the queue/admission/persistence
code. Its opaque prompt labels can map to the evidence-derived six/two shape;
there is no need to import private prompts into repository tests. Do not claim
that label substitution preserves timing or full streaming semantics.

## What future passing fixtures would still fail to prove

- Stub-only provider tests establish cancellation ownership but no Codex lineage.
- Direct `run_agent_stream` tests with a discarded receiver establish streamed
  output persistence but omit the faulty input-admission path.
- Preparing from runtime history proves no agreement with durable replay unless
  actual persisted input is independently reconstructed and compared.
- A fresh Session in the same process does not prove child-process/crash recovery.
- A test hook simulating a pre-promotion error does not prove rename succeeded
  before acknowledgment failed; use observed real durability failpoints.
- A single clean compaction does not prove repeated automatic compaction,
  pre-prompt request de-duplication, or generation binding after cancellation.
- Concurrent tool definitions do not prove out-of-order completed results were
  persisted; capture the actual order and side-effect counters.
- Calling no real tool cannot establish completed effects are not replayed.
- Source-level GREEN does not prove the installed daemon has the repair or that
  preserved legacy sessions have been safely reconciled.

## Approval boundary

The oversized CLI refactor remains explicitly unapproved in the current design.
The parent supplied an actual auto-review rejection of using controller authority
to bypass that prerequisite. This audit neither authorizes a refactor nor suggests
an alternate production path around it. Only unaffected specification, test and
read-only evidence work was reviewed. No production or indirect workaround was
implemented.
