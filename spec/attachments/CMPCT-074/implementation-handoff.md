# CMPCT-067/074/075 active implementation handoff

Owner: 9721cd0e-4b9f-48de-b7b1-dd9f51e8d99c. Started 2026-10-17.

Read shared contract, all three cards, existing Codex stream/lifecycle and hook.
Feature-first specifications and real scripted-wire failing tests are next.
No completion, coverage, or green claim yet; no commits.

## Resume checkpoint (9721cd0e)

Resumed: 074/075 seven green tests retained. New real backend-wire test
`fspec-tui/tests/cmpct067_runtime_fill.rs` exercises actual run_agent_stream;
currently building for behavioral red. 067 reducer already changed after red,
but producer still mixes final aggregate and physical occupancy. Implementing
outer Rig final metadata propagation and shared snapshot projection next.
071/065/078: preserving your serialization, ledger and generation APIs.

Current scoped evidence `/tmp/cmpct074075-green.log` confirms 074 three and075
four behavioral tests green after the captured red. 067 DTO/reducer exists but
runtime still emits mixed legacy fill; completing backend producer and event
projection next. Coverage remains unmapped and cards must not be marked done.
071 must retain actual runtime explicit admission configuration. 078 generation
is read through `CodexHistory::generation()`. No alternate transport/protocol.

Progress: three new feature files written, all cards moved to testing with fspec.
Behavioral red now confirmed: `/tmp/cmpct067-reducer-red.log` (three failures),
`/tmp/cmpct074-behavior-red.log` (aggregate occupancy cancels fitting request).
075 compiler red awaits promised 071 model configuration API; do not count it
as behavioral red. Need all three stages/coverage before completion.
074 will expose `GetTokenUsage::context_usage()` and optional `request_context()`
metadata plus Responses final `request_context`. 069 requested
`request_usage_records()`; 074 will preserve each physical ID/usage record there.
Main052: need approval for purely mechanical extraction of pre-existing oversized
hook/Responses stream files; no policy replacement intended. Busy message failed.
074 scripted physical Responses → actual CompactionHook integration tests written
first. Release-slim red run is blocked behind existing 30-minute stale cargo/link
activity (78039; rustc 81096/81097), not a test failure. Switching own validation
to ci-test; coordinator should inspect old worker-owned release-slim processes.
Need approval/coordinator extraction for existing oversized hook and stream-loop
files; proposed production edits there are small and do not add a second policy.

## Shared seams and proposed API

- 075 owns `providers/src/codex/streaming.rs` continuation boundary. 071 owns
  full selected-request preparation/admission. Please expose admission of the
  final `serde_json::Value` with explicit capacity/output reservation and current
  generation; 075 will call it on every initial/continuation serialized body,
  before transport. Same bytes must survive same-transport retries.
- 074 needs the latest physical-request occupancy separate from aggregate final
  spend. Proposed additive latest-request token metadata on the Responses final
  DTO, consumed by the core hook; aggregate `usage` remains unchanged for billing.
- 067 owns a shared request/model/generation context snapshot DTO and context-fill
  consumers. Unknown/post-commit estimate provenance remains explicit. Coordinate
  normalized inclusive input/output with 065; do not add cache/reasoning twice.
- 078 owns provider history/session generation and transport; 075 will not edit
  those files. Need a read-only generation accessor for request admission.
- Main 052: no change to your inference block gate planned. A rejected internal
  continuation returns failure, never synthetic Continue or successful completion.

## Dependencies

071 serializer and 078 generation access must be used rather than a second
incomplete request approximation. 065 normalization must preserve each measured
segment. Durable generation transition invalidates previous occupancy; it does
not erase measured session spend.
