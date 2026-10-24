# CMPCT-055/059 versioned durable commit handoff

Owner: `7e9bffc3-3f48-459a-8be9-1da17dbb256a`.

The commit owner will live in `codelet_core::persistence::compaction_commit`.
It captures a `CompactionSource` from the durable manifest: target UUID,
generation, model/protocol identity, immutable message-ID prefix and coverage
watermark. `commit_compaction` checks that source against the current manifest,
preserves its uncovered message-ID suffix and promotes one new generation through
`SessionStore::update`. No mutation or completion is allowed before promotion.

The candidate must carry operation UUID, exact validated DAG, source contract,
token provenance, provider checkpoint and a whole-selected-request admission
certificate. CMPCT-071 owns constructing that certificate with the actual
provider serializer, reminders, tool schemas and verified suffix. The owner must
not substitute a session-message estimate for it.

Promotion returns a committed identity (target, generation, operation, manifest
revision). Publication failure retains that identity and blocks inference;
reconciliation accepts only that actual durable identity, never an arbitrary
clear. Cold resume uses the same manifest generation and checkpoint plus the
verified durable suffix, without raw-archive fallback.

API names are an early coordination contract, not a claim of implementation or
completed acceptance criteria. Runtime integration and behavioral red/green
evidence remain required.

## Immediate coordination requests

- Main: please delegate CMPCT-056 to a replacement worker if available; spawning
  here failed at the ten-session limit. Existing seven tests have no production
  fixes. Approve extracting cross-target orchestration from the >900-line handler
  into a new <300-line module if needed; no broad redesign intended.
- CMPCT-064 owner: I will add defaulted `SessionManifest.compaction_commit`
  metadata referencing a new type, leaving `CompactionState` literals compatible.
- CMPCT-071 owner: please record the concrete real serializer/certificate API
  here or in your handoff so the durable runtime route can call it before promotion.
- Behavioral prefix red: `/tmp/cmpct055-prefix-red.log` (legacy setter rejects an
  unchanged source prefix solely because appended suffix advanced revision).

## Available concrete API

`persistence::compaction_boundary::capture_compaction_source(&manifest, model,
protocol)` returns the immutable durable prefix/generation contract.
`persistence::compaction_commit::commit_compaction(candidate, selected_model,
admit)` runs `admit(&current_manifest, &candidate) -> Result<(), String>` inside
the manifest owner transaction **after** freshness checking and **before** any
replacement. This callback must serialize/check the complete request against the
current authoritative suffix. It is synchronous to avoid holding the store over
inference. `CompactionCandidate` carries `ValidatedDagCandidate`, source,
operation ID, `CompactionCheckpoint`, input tokens/reservation/provenance.

`publish_compaction(&committed_manifest, publisher)` retains promoted identity on
publisher failure; `reconcile_compaction(&identity)` clears only the matching
durable publication-pending state without creating another generation.

Runtime integration still required: candidate preparation uses071; installation
uses exact committed DAG+suffix, compatible provider transition, then authoritative
identity reconciliation to open052's inference gate. No helper-only completion.

Build coordination:059 cold tests are currently blocked by061's duplicate
`restored_replay` initializer in CLI `Session::new` and missing initializer in
`from_provider_manager` (see `/tmp/cmpct059-cold-red.log`). Core's dev-dependencies
compile CLI. Please repair that in061 ownership before final shared checks.

## Active continuation 2026-10-17

Core owner API above is implemented, not yet runtime-integrated. Current059
behavioral red is `/tmp/cmpct059-cold-red.log`: 3 pass, incompatible-prefix replay
fails because the reader accepted a guessed summary. The reader now invokes
the durable boundary validator; green verification underway. CLI initializer
blocker is repaired in current shared source.

071 concrete requested seam acknowledged: `Session::prepare_compaction_request`
must prepare exact DAG+authoritative suffix+required reminders for next generation.
055/059 will install from the returned committed manifest, reconcile exact
identity, then clear052 gate; no unconstrained clear method. Own new
`cli/src/session/compaction.rs` for runtime prepare/commit/install, leaving071's
selected serializer module/setup and079 lifecycle ownership separate.

071 please expose `PreparedSelectedRequest.input_tokens()` and
`output_reservation()` getters and synchronous
`Session::prepare_compaction_request(&[rig::message::Message], u64)`; runtime
commit callback invokes it after freshness with the current durable suffix.
I will capture callback certificate into locals and have core commit owner use
those values, not builder estimates. Please add constructors/setup to real071
integration fixture (current CLI tests never create durable session and are
superseded by mandatory durable boundary).

Runtime continuation: adding `Session::capture_durable_compaction(session_id)`
and `Session::commit_durable_compaction(source, operation_id, content, publisher)`
in `cli/src/session/compaction.rs`. Captured source must travel with pending
candidate, never be recaptured at application. Candidate replay assembled from
the current manifest under owner transaction; 071 certificate sets persisted
input/reservation. Provider generation prepared non-destructively before
promotion, installed only afterward. New core `session_replay::envelopes_for_manifest`
avoids recursive session-store lock during authoritative suffix preparation.
056 still needs cross-target bounded lock/cancellation worker; no capacity here.

## Resumed implementation contract

055/059 is adding `Session::commit_durable_compaction(source, operation_id,
content, prepare, publish)` where `prepare` synchronously receives exact
`&[Message], generation` and returns `PreparedSelectedRequest`. This avoids
colliding with071's Session preparer field/setup; caller can clone its071
preparer before borrowing Session mutably. `capture_durable_compaction` captures
the original source before builder inference. Installation uses rig's existing
Anthropic envelope conversion, not a new lossy text parser. The live compactor
will need071's synchronous `prepare_compaction_request` installed first.
