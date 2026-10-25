# BUG-263: Chat terminal evidence and accepted completion

## Finding

Source-confirmed composition gaps at `f0eb0759`. `2699693b` added mandatory
usage evidence and transport-error hardening, but terminal evidence and immediate
Done do not compose safely.

## Evidence

- `rust/patches/rig-core/src/providers/openai/completion/streaming.rs:182` ignores
  [DONE]; line 186 skips parse/schema failures, including mismatched error envelopes.
- Lines 365–381 validate usage but not a captured finish_reason before Final.
- Tool emission at lines 317 and 355 precedes later terminal usage validation.
- `rust/cli/src/interactive/stream_loop.rs:1119` drops accepted-Done streams.
- `rust/patches/rig-core/src/agent/prompt_request/streaming.rs:837` invokes finish
  settlement/context hooks only when the provider Final is consumed.

## Regression matrix

- Valid usage plus premature EOF without finish_reason must not be declared a
  clean completion or consume an ordinary auto-continue nudge as if successful.
- Malformed/error terminal frames retain actionable causes.
- Done-bearing missing/inconsistent-usage streams cannot conceal terminal failure.
- Valid Done still terminates promptly without executing trailing tools.
- Cover native Chat SSE and JSON-to-stream; ZAI and Copilot use the same decoder.
- Assert request usage, current context, history and one honest terminal outcome.

The correct protocol terminal boundary must be specified before implementing;
do not simply require a particular optional marker or weaken validation to pass
an incompatible fixture. Preserve low-latency tool behavior where safe.
Related: BUG-262 handles order, not terminal evidence.
See [full audit](../CONT-011/fspec-auto-continue-investigation.md).
