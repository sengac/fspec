# BUG-262: Chat tool-call ordering and immediate Done

## Finding and history

Source-confirmed, longstanding: unordered accumulation predates the completion
contract (`d69d89a9`); immediate Done was introduced in `7c95aa84`.
The native Chat SSE decoder can emit Done before earlier-indexed ordinary work.
Immediate accepted completion drops the stream, preventing that work from running.

## Evidence

- `rust/patches/rig-core/src/providers/openai/completion/streaming.rs:167`:
  HashMap keyed by provider tool index.
- The same file at lines 317 and 355 emits unsorted `into_iter()` at both flush sites.
- `rust/cli/src/interactive/stream_loop.rs:1119`: accepted Done drops the stream.
- `rust/patches/rig-core/src/providers/openai/completion/nonstreaming.rs:105`:
  JSON-to-stream path instead preserves vector order.

Affected consumers: native OpenAI, ZAI and native Copilot. This is not a proven
last-month introduction or a dedicated Codex decoder issue.

## Acceptance/test direction

Send fragmented real SSE calls with index 0 ordinary work and index 1 Done.
Assert observable side effects and actual acceptance order, not tool-definition
presence. Cover finish_reason tool_calls and EOF flush paths. Preserve provider
index order, IDs, argument reconstruction, nonstreaming parity and history pairing.
Accepted Done must still terminate immediately and cancel genuinely trailing work;
do not solve ordering by draining executable calls after completion.

Existing contracts: CONT-001, CONT-005, CONT-006, PROV-140.
See [full audit](../CONT-011/fspec-auto-continue-investigation.md).
