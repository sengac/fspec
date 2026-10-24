# Current CLI completion pass

Owner `bea9ef97`, coordinator `ad8aee9e`. Exclusive CLI ownership; no board,
staging, commits, core/provider/session edits or acceptance weakening.

## Active repairs

- Actual Session stream terminal returns at `interactive/stream_loop.rs` lost
  source identity with `anyhow!("Agent error: {e}")`. Existing 083 negative
  feature and runtime assertions demonstrate authentic RED in
  `/tmp/cmpct083-current-gate-matrix.log`: 6/8 negative cases pass, WebSocket
  upgrade and checkpoint lineage typed-chain assertions fail. Both CLI terminal
  returns now preserve the source with `e.context("Agent error")`.
- Existing selected publication and fitting complete-request regressions are
  behavioral RED in `/tmp/cmpct-cli-owner3-acceptance.log`. The selected builder
  previously passed an unconditional publication success closure. It now builds
  the identity-bearing durable snapshot and calls `publish_committed_context`
  before reconciliation and legacy completion. Failure leaves the exact promoted
  identity pending and inference blocked. New code is isolated below 150 lines.
- New `structured-terminal-overflow-classification.feature` and
  `cmpct054_structured_overflow.rs` exercise the real classifier using structured
  HTTP failures, nested context, negative machine codes and forged bare text.
  RED invocation is queued in `/tmp/cmpct-cli-current-overflow-red.log`; no
  classifier repair before behavioral RED. Terminal054 rerun is queued in
  `/tmp/cmpct-cli-current-terminal-red.log`. The shared Cargo cache is currently
  rebuilding dependencies for the gate owner; waiting is not test success.

Core retains explicit nested tool `call_id` blob projection ownership; CLI adds
no workaround. Existing actual pre-prompt isolation and exact provider identity
repairs are retained. Checkpoint-only Codex continuation projection excludes only
genuine empty assistant metadata envelopes, preserving immutable originals and
conversational suffixes; the real gate remains required.

## Traceability handoff

## Restore preflight API proposal — current owner

CLI reserves `Session::preflight_compaction_restore(&mut self,
manifest: &SessionManifest, envelopes: &[String]) -> Result<(),
CompactionCommitError>`. It will block inference, validate the durable boundary,
selected identity, replay envelope conversations and binding identity, then call
a nonmutating provider validation method before installing boundary or tokens.
Sessions should call this before `restore_compaction_boundary`,
`bind_provider_persistence` and `restore_provider_state`. Existing actual
`cmpct061_restore_atomicity` fresh-process tests own the acceptance promise.

Provider dependency requested from owner `e22f772a`:
`ProviderSession::validate_restore(envelopes: &[String], committed:
Option<(u64, &HashMap<String, Value>)>) -> Result<(), ExtensionError>`.
The check must validate every extension's retained checkpoint and proposed
committed generation without touching shared extension Arcs or blocking the
provider's current history. Binding failure checks must also precede mutation.
The direct request was rejected busy, not queued. This proposal is not an
implemented API or completion claim; parent should relay and settle the exact
provider seam with its owner. CLI will not duplicate Codex checkpoint business
logic or use shallow-clone validation.

## Current verified classifier RED

`/tmp/cmpct-cli-current-overflow-red2.log` finished exit101: 3 passed,
1 failed behaviorally at `structured_http_overflow_survives_streaming_wrappers`.
Typed HTTP bodies containing the three accepted overflow codes were ignored.
The repair now inspects only typed Rig HTTP/status or ProviderError bodies and
preserves code-based negative controls; no diagnostic substring fallback.
GREEN is queued in `/tmp/cmpct-cli-current-overflow-green.log`.
`cargo check --profile ci-test -p codelet-cli -j2` finished exit0 in
`/tmp/cmpct-cli-current-check.log`; Clippy and fourteen-target acceptance remain
pending. Disk29GiB. The outer054 builder RED is real in
`/tmp/cmpct-cli-current-terminal-red.log` (0/1, two filtered), not a pass.

## Actual post-media gate inspection

`/tmp/cmpct083-owner-post-media-matrix.log` now finished exit101, cold10/11
and negative7/8. All typed source and blocked-readiness checks now pass;
the remaining cold Codex row reports unusable finite-positive limits in the
fresh child. The remaining lineage negative fails the outer manifest-byte
assertion, not the runtime typed-error assertion. Gate owner should inspect
`negative_transport.rs`: its deliberate `history.commit(different-model)`
uses the bound persistence writer after the outer immutable-state baseline was
captured. That may persist the fixture's intentional corruption itself; retain
all genuine no-request/no-completion/unchanged-failure checks when correcting
the baseline. This is a source-level diagnosis, not a fixture repair or pass.
The gate owner direct message was rejected busy, not queued.

## Completed current CLI verification

The fourteen-target command in `/tmp/cmpct-cli-current-acceptance.log`
finished **Cargo exit0: 44 passed, 0 failed**:

| Target | Passed |
| --- | ---: |
| cmpct052_inference_handoff | 5 |
| cmpct057_operation_lifecycle | 4 |
| cmpct058_selected_overflow | 2 |
| cmpct059_promoted_ack | 1 |
| cmpct059_provider_identity | 6 |
| cmpct059_reminder_obligations | 3 |
| cmpct059_runtime_commit | 4 |
| cmpct059_selected_publication | 1 |
| cmpct059_structured_tool_identity | 2 |
| cmpct071_whole_request_fit | 4 |
| cmpct079_no_substitute_success | 4 |
| cmpct079_pre_prompt_isolation | 2 |
| cmpct079_recovery_isolation | 2 |
| cmpct082_explicit_admission | 4 |

Classifier GREEN in `/tmp/cmpct-cli-current-overflow-green.log` finished
**Cargo exit0: 4 passed, 0 failed** after the genuine 3/4 RED. This confirms
all three typed HTTP overflow codes through agent/source wrappers and preserves
negative diagnostic/forged-text controls. Exact command:
`cargo test --profile ci-test -p codelet-cli --test cmpct054_structured_overflow -j2`.
This new feature/test is now stable for parent transfer to traceability owner
`bfd53429`; sidecar ownership remains delegated, no concurrent CLI edits.

Current `cargo clippy --profile ci-test -p codelet-cli --lib -j2` finished
exit0 in `/tmp/cmpct-cli-current-clippy.log`. Owned fifteen-target test Clippy
is queued in `/tmp/cmpct-cli-current-clippy-tests.log`. Scoped terminal054 actual
builder verification is queued in `/tmp/cmpct-cli-current-terminal-green.log`.
`cargo fmt -p codelet-cli --check` passes exit0 after the classifier cleanup.
New feature format, syntax and scoped tags all pass via `fspec format`,
`fspec validate` and `fspec validate-tags` with `file` set to its exact path.

These are selected acceptance results, not a complete package test claim.
An inherited compatibility conflict remains for coordinator decision: historical
done `context-overflow-error-classifier.feature` / `cmpct044_overflow_classifier_test`
requires untyped diagnostic substrings to authorize overflow, whereas the newer
explicit structured-only runtime contract forbids this. That old target was not
included in the current command and is not claimed passing. No historical
scenario or regression is removed or weakened in this pass.

All selected CLI targets are locally GREEN, but cross-layer completion remains
blocked by genuine Codex cold limits, the gate fixture's lineage baseline audit,
and the pending provider/CLI/session nonmutating restore-preflight seam. No
board transition or 100% aggregate ACDD readiness is claimed.

Parent may transfer the stable prior CLI features/tests listed in
`cli-current-owner-handoff.md`, plus `cmpct059_promoted_ack`, provider identity,
structured tool identity, and pre-prompt isolation, to current traceability owner
`bfd53429`. Preserve all assertions and exact identity acceptance. New 054
classifier feature/test remains CLI-owned until its RED/GREEN verification.
Sidecars remain delegated, not concurrently edited here.

No fresh GREEN, quality completion, or card readiness is claimed yet. All Cargo
commands use shared `ci-test -j 2`; latest available disk is 42 GiB, above 20 GiB.

## Resumed verification checkpoint

The authentic negative gate progressed beyond source-chain checks and now
requires readiness to remain blocked. `interactive/request_failure.rs` checks
the typed source chain for `CheckpointError` and `WebSocketUpgradeRejected`,
blocking the Session before generic retry/tail-stripping paths. Both error
returns preserve the original source rather than rendering it into a string.
The gate owner's `/tmp/cmpct083-owner-post-media-matrix.log` is already queued;
do not duplicate it or infer GREEN from a pending Cargo command.

`/tmp/cmpct-cli-current-overflow-red.log` failed during an in-flight core edit
(`blob_processing.rs` E0425), not behaviorally. Its retry
`/tmp/cmpct-cli-current-overflow-red2.log` remains queued. The classifier has
not been repaired before this genuine behavioral RED. Scoped full CLI
acceptance is queued in `/tmp/cmpct-cli-current-acceptance.log` (fourteen named
integration targets, `--no-fail-fast`). Check/Clippy are already queued in
`/tmp/cmpct-cli-current-check.log` and `...-clippy.log`.

Package formatting now passes: `cargo fmt -p codelet-cli --check`, exit 0,
`/tmp/cmpct-cli-current-format-after.log`. This includes the inherited CLI
formatting backlog, without cross-owner formatting edits. Latest disk is
33 GiB. No readiness or acceptance completion claim yet.
