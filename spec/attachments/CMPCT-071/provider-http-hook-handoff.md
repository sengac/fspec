# Provider HTTP hook — 2026-10-18

`HttpClientExt::prepare_request_body(&self, body: Vec<u8>) ->
rig::http_client::Result<Vec<u8>>` is now available with identity default.
CopilotHttpClient overrides it using the exact existing classify_and_cache_body
function invoked by both physical send modes. Existing sends are preserved.
The serializer owner may call it as the last serialization step for every
built-in provider. Repeated cache transformation must remain idempotent.

Behavioral prerequisite inspected before this change:
`/tmp/cmpct071-builtin-behavior-red.log` has five actual assertion/test failures
for missing selected physical serializers, including the actual Copilot adapter.
The later missing-hook compilation errors are NOT behavioral red evidence.

Current provider owner is working on synchronous custom serialization, testing
the actual script-built streaming body, not a common synthetic envelope.
No board, staging or commit actions are performed.

## Resumed completion

Current provider owner `dca1b90b` verified both implementations remain present.
The Copilot override belongs to `rust/providers`, not the separately owned Rig
provider subtree; no concurrent Rig-provider edits were made. Parent and builtin
owner messages returned `session_busy` and were not queued; this attachment is
the shared handoff. The hook is no longer a compiler blocker.

Custom behavioral RED: `/tmp/cmpct071-custom-behavior-red.log` executes both
new scenario tests. The failure-control assertion reports the unavailable
physical serializer instead of the selected script error. The parity test exits
before HTTP because preparation is unavailable (its original mock expectation
also fails on teardown); this is not a compile-only failure.

Custom implementation now overrides `CompletionModel::prepare_request` and
uses `rig_preparation.rs::streaming_request_body` for both admission and physical
streaming. Physical streaming invokes that same synchronous script serializer
on Tokio's blocking pool. Preparation performs no HTTP or nested runtime, and
passes the existing complete `additional_params` through as thinking config.
Green verification is running in `/tmp/cmpct-provider-completion-green.log`.

## 08:31 AEST integration handoff

Custom parity GREEN: both tests pass in provider-completion-green.log. The same
run exposed 078 fixture missing explicit limits (now corrected) and 082 real
CompactionHook poisoned-lock fail-open behavior. Rig cancellation fallthrough is
being fixed in the owned non-provider streaming module; behavioral RED is
`/tmp/cmpct082-hook-cancel-red.log`. Core owner must independently cancel on its
poisoned TokenState lock: current core/compaction_hook.rs returns without cancel
in on_completion_call and the helper checks. Provider test
`cmpct082_physical_admission::inaccessible_admission_state_blocks_the_streaming_agent`
is already behavioral RED in `/tmp/cmpct-provider-completion-green.log` and
`/tmp/cmpct-provider-cancel-green.log`; production core belongs to c13f7921.

Gate stub seam is in progress, with behavioral tests queued. The explicit
request contract is `additional_params.stub_tool_call = {id, name, arguments}`.
It emits that real tool call until its matching recorded tool outcome is present,
then restores ordinary stub text. Use AgentBuilder.additional_params for the
live injection row. `stub_model::last_request_bytes() -> Vec<u8>` will record
the same full-context serializer used by prepare_request; no synthetic envelope
in the gate and no global scripted response changes are needed.

## 08:40 AEST provider checkpoint

Stub seam is implemented. Behavioral RED is
`/tmp/cmpct071-stub-behavior-red.log` (two actual failures); both scenarios now
pass in `/tmp/cmpct-provider-final-green.log`. Custom parity, 065/066/069/074/075/
078/080/081 and explicit cancellation also pass in that run. The only remaining
provider target failure is real poisoned CompactionHook state:
`cmpct082_physical_admission::inaccessible_admission_state_blocks_the_streaming_agent`
still receives a false completion. Core owner must cancel the poisoned-state
path; Rig now emits and terminates on cancellation, proved by three phases in
`cmpct082_hook_cancellation` (before request, text delta, final response).

Reproduced CLI missing-limit behavioral RED independently in
`/tmp/cmpct082-missing-codex-limits-red.log`: unknown model resolves to guessed
`(272000,4096)` instead of `(0,0)`. CodexProvider and ProviderManager Codex resolver
defaults now retain zero/unavailable while registry/user overrides still resolve
normally. No constant fallback is restored for the 078 fixture.
