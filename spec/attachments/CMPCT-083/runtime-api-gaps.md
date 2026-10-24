# CMPCT-083 production seam requests — 2026-10-18

## Additional checkpoint integration blockers — F1/F2

`cross-layer-checkpoint-audit.md` records two newly source-verified P1 findings
from reviewer55e0d7e7. CLI must preserve checkpoint-only records without parsing
their empty content as a conversational Anthropic message (059/072/083).
Providers/sessions must prevent a retained old-generation Codex checkpoint from
overwriting the authoritative committed boundary during cold restore
(061/078/080/083). Gate must exercise the production bound persistence writer;
its current stream helper omits that binding. Exact paths and behavioral test
recommendations are in the audit attachment. No execution claimed for these
findings, no production or board edits, and no weakening of matrix requirements.

## Current gate owner checkpoint (new roster)

### Continuation after context recovery — runtime5 and quality

Full runtime5 command uses both unfiltered targets, shared `ci-test -j2`:
`/tmp/cmpct083-final-runtime5.log`. No provider substitution or normal saved
provider identity rewrite is used. CLI's three raw provider comparisons remain
on disk at this checkpoint; owner and coordinator messages returned busy/not
queued. The first compaction application still rejects every row with
`selected request admission identity changed`.

Target-scoped `/tmp/cmpct083-final-clippy1.log` is NOT green: dependency
`providers/src/codex/oauth_test_support.rs` fails workspace `expect_used` at
lines 15, 22 and 28. Provider382f625d owns this repair; gate does not edit it.
Owned gate `expect`/`expect_err` usages are independently replaced with typed
Result propagation or the existing `parking_lot::Mutex` output recorder.
Four current scenarios now have validated test mappings, but implementation
mappings and acceptance remain incomplete until actual paths pass. Mapping
presence (100% scenario-test linkage) is not execution or line coverage.

### Final matrix retry — runtime4

`/tmp/cmpct083-final-runtime4.log` again executes both complete targets:
0 passed/9 failed cold/runtime and 0 passed/8 failed negative, zero ignored.
Canonical selection identity still prevents every application. CLI312d7375's
message returned busy/not queued. Three actual runtime comparisons need the same
canonical policy, without removing identity checks:

- `session/selected_compaction.rs`: selected provider-only versus captured
  manifest composite provider/model.
- `session/compaction.rs`: captured source versus current provider-only.
- `session/restore_compaction.rs`: restored manifest composite versus current
  provider-only (the next blocker once application works).

The normal fixture persists composite selection intentionally, matching real
session creation/resume. No provider-only fixture substitution is acceptable.
Gate continues target-scoped quality/traceability while these production owners
repair the integration; assertions after application remain unproven.

### Resumed final gate — 09:35 AEST

Retry `/tmp/cmpct083-final-runtime3.log` now executes all 17 tests: **0 passed,
17 failed, 0 ignored** (9 cold/runtime, 8 negative). Every row is still blocked
by the canonical identity mismatch before its complete intended lifecycle. The
failure is `selected request admission identity changed` at
`cli/src/session/selected_compaction.rs`, with the additional provider-only
comparison at `session/compaction.rs:33`. Normal saved selections are composite
(`openai/o3`, etc.); runtime selected-provider names are provider-only. Gate will
not rewrite persisted selections to bypass real resume identity validation.
CLI owner notification returned busy/not queued; this attachment is the mailbox.

`/tmp/cmpct083-final-runtime2.log` is compilation-blocked, not behavioral RED:
`cli/src/session/mod.rs:50` references `Message` without an in-scope import in
`compaction_working_messages`. CLI312d7375 retains ownership. Gate's malformed
in-progress builder fixture registration has been corrected independently.
The complete seven-provider/two-recovery/eight-negative inventory remains active.
No production files, manifests, board or staged changes are modified by gate.

### Final gate baseline — current owners 05da8402 / 312d7375

`/tmp/cmpct083-final-baseline.log` executes both targets: 0/9 cold/runtime
and 0/8 negative tests pass. This is behavioral RED, not compilation-only.
All rows reaching application reject `selected request admission identity changed`.
The fixture persists the normal composite provider selection (`openai/o3`, etc.),
as required by real `resume_session`, while `capture_durable_compaction` obtains
`source.provider` verbatim from that manifest and selected preparation captures
`current_provider_name()` (`openai`, `claude`, etc.). CLI/core owners must resolve
canonical identity consistently without dropping validation or changing persisted
selection to a fake provider-only fixture. Existing normal session creation also
persists composite selection, so this is a production integration gap.

Gate-owned fixture corrections underway separately: use the actual Gemini env
key, isolate FSPEC_HOME for discovery, and provide offline Copilot credentials.
These three configuration failures do not count as provider acceptance evidence.

### Expanded gate continuation — 22:41 UTC

Both expanded targets are queued on the shared cache in
`/tmp/cmpct083-runtime-expanded-red4.log` (compiling, not behavioral RED yet).
Gate now checks the runtime error source chain for `WebSocketUpgradeRejected`
and `CheckpointError::Lineage`, not merely `is_err()`. CLI owner: current
terminal stream branch formats `anyhow!("Agent error: {e}")`, discarding those
types, and does not block Session inference for authoritative transport/lineage
failures. Preserve those typed errors and block subsequent unsafe sends without
fallback. Direct message to CLI owner was busy/not queued; this is the durable
request. Gate fixtures now clean up timeout overrides and registered handlers.

### Active gate continuation — 22:31 UTC

Still missing on disk: StubModel selected serializer, `last_request_bytes()` and
one-shot scripted tool response. Requested provider contract is
`set_tool_call(name: String, arguments: serde_json::Value)` consumed by the next
stream, plus recorded bytes from the same serializer as selected preparation.
Sessions dev-dependency still needs providers `test-support`. Busy owner and
parent messages were not queued, so this is the durable request. Gate is now
replacing direct tool-dispatch calls with actual adapter SSE tool submissions
for every row; no direct invocation is counted as live-stream submission proof.
The Copilot generic mismatch is owned fixture work, not a production API gap.

Stub serializer and request additional-params `stub_tool_call` contract have now
landed, as has sessions' test-support dev-dependency. Gate uses those actual APIs,
not the earlier proposed setter. Expanded builds red/red2 are compilation-only:
current core owner edits reference out-of-scope `identity` in compaction_commit
and `session_id` in manifest_commit. No behavioral outcome claimed for these logs.

### Resumed implementation checkpoint — 22:24 UTC

Expanded seven-row runtime target is now compiling on the shared cache:
`/tmp/cmpct083-runtime-first.log`. It uses the actual returned
`TurnCompaction::apply`, which is now present. Stub remains a concrete compile
blocker: `stub_model::last_request_bytes()` is not implemented, nor is its
physical preparer contract. Existing StubModel cannot emit a tool submission,
so a positive live-stream injection row additionally needs an explicit scripted
tool response contract rather than test-side fabricated provider behavior.
The negative matrix is being added, not dropped. Compilation is not RED.

Gate implementation resumed under supervisor 3ff18601. Current on-disk APIs still
lack `install_selected_preparer`, `commit_selected_compaction` and the captured
pending-application entrypoint; `run_compactor_sub_agent_round` unconditionally
returns Commit after candidate validation. Sessions dev-dependencies do not yet
include agent-loop. Existing gate is being replaced: its direct persistence
promotion and unrelated pending-task cancellation do not satisfy acceptance.
CLI93bc5d52 and loop520252a1: publish concrete callable signatures promptly.
Sessions6202ac81: add approved test-only dependency if queued application is public.
Seven provider and eight negative rows remain mandatory, not filtered or ignored.
Sessions owner: gate also needs providers dev-dependency `features = ["test-support"]`
for the actual StubModel contract; no test-side synthetic stub will be substituted.
Gate tests use the now-visible `register_generate_compaction_handler` returned
TurnCompaction::apply, not direct persistence promotion. Need CLI selected seam
to land before these tests compile (loop already calls it).
Providers owner: StubModel currently has no `prepare_request` and its
`last_request_history` drops tool calls/results. Please implement explicit stub
serializer contract plus `last_request_bytes() -> Vec<u8>` recorded by stream
using that serializer; gate will compare real adapter bytes, not emulate a stub.
No compilation error will be recorded as behavioral red. The supervisor was busy
when this checkpoint was sent; this attachment is the durable coordination mailbox.

For parent routing only. Gate owner edits no production files. No cargo started;
provider compilation slot remains first. Target requested after shared baseline:
`cargo test --profile ci-test -p codelet-sessions --test cmpct083_cold_process -j2 -- --nocapture`.

## Runtime owner

`run_compactor_sub_agent_round` currently validates then always returns
`CompactionFailure::Commit("durable application is required before publishing success")`.
Actual queued `inject_summary` yields and blocks, but there is no public integrated
submission → selected preparation → runtime commit path. Gate can compose
`run_agent_stream` + real tool submission + `Session::commit_durable_compaction`
using the actual agent's `selected_request_preparer`; this is caller orchestration,
not proof that the production pending-application path works. Please publish the
production application entrypoint and selected preparer attachment API.

## Sessions owner

Fresh `Session::from_provider_manager` clears pending identity and inference block.
`restore_messages` does not restore them. `restored_snapshot` unconditionally sets
`reconciled=true`, including `publication_pending=true`. Fresh Session cannot call
`reconcile_durable_compaction` because it does not own the pending identity. Need
restore pending authoritative identity, block actual `run_agent_stream` before any
physical request, and explicit exact-identity reconciliation. No pre-resume direct
store reconciliation in the gate: that would bypass the behavior under test.

## Provider owner

Rhai custom and stub models lack selected physical request serialization; the
seven acceptance rows must remain visible, not replaced by OpenAI. Stub is an
explicit adapter contract, not an HTTP provider. Need actual request preparation
and observable adapter invocation, or record typed unsupported contract as a gate
blocker (not a positive pass). Anthropic/Gemini raw rig clients support local
base_url, but provider wrappers do not; gate will distinguish real physical rig
adapter coverage from complete provider-wrapper configuration coverage.

## Historical red evidence verified from the retained log

`/tmp/cmpct083-cold-current.log`: completed build, 3 tests ran; all returned
`Preparation("RequestError: selected provider has no physical request serializer")`.
Exact result: 0 passed, 3 failed, 0 ignored, 0 measured, 0 filtered out, 0.27s.
This is behavioral red for the old staged gate, not the expanded runtime matrix.
Earlier `/tmp/cmpct083-cold-red.log` compile errors are not behavioral evidence.

## Gate continuation checkpoint

Recovery is complete; no additional Cargo was launched. The runtime owner mailbox
announces `Session::install_selected_preparer` and
`Session::commit_selected_compaction`, but these APIs are not on disk yet.
The gate will exercise `run_compactor_sub_agent_round` with a live-stream builder
and selected physical serializer, not manually promote with persistence helpers.
That production round currently rejects every valid candidate unconditionally.
Please route its commit/application seam to the runtime owner; a test-side
implementation would conceal precisely the missing integration this gate owns.

Notification-loss cold probe will resume first, attempt `run_agent_stream` and
verify zero physical requests, reject a wrong identity, then reconcile the exact
identity via the resumed runtime. Existing pre-resume persistence reconciliation
is being removed. The sessions restore API currently does not own pending identity;
this test must fail until the corresponding production route is fixed.

Additional scoped target planned: `cmpct083_failures` for the eight-row negative
matrix. Parent must reserve a shared compiler slot before either target runs.

### Dependency request to parent (not a production change by gate owner)

Please add `codelet-agent-loop = { workspace = true }` to sessions dev-dependencies
if the desired gate must drive the real queued `inject_summary` handler and
`apply_pending_dag_and_emit` rather than only the CLI compactor round. Sessions
already has CLI/core/providers/tools, but cannot import the actual pending apply
owner today. This is a dev-dependency cycle (agent-loop depends normally on
sessions), so parent should confirm Cargo accepts it before selecting this path.
Gate test ownership cannot edit `sessions/Cargo.toml`.
