# BUG-253: Repeated Gemini tool invocations fail journal admission

## Review context

- Reviewed commit: `144653b0dc12ca096e835e86b620b109b29019bf` (`more fixes`).
- Review priority: P1 — normal tool-driven turns can be interrupted.
- Evidence: static source and caller inspection; no runtime reproduction or new regression test was executed during review.

## Problem and impact

The new stream journal assumes a provider tool ID identifies one invocation for
the entire background turn. Gemini's streaming adapter uses the function name
as the tool ID. A valid turn that invokes the same tool more than once can
therefore be rejected as a duplicate instead of executing its second invocation.
The result-ID map also conflates separate results with that same provider ID.

## Source evidence

Locations refer to the reviewed commit:

- `rust/patches/rig-core/src/providers/gemini/streaming.rs:177` constructs a raw tool call with the function name as both ID and name.
- `rust/agent-loop/src/stream_journal.rs:128` records requested calls using the provider ID.
- `rust/agent-loop/src/stream_journal.rs:144` keys durable tool-result identities by that same ID.
- `rust/agent-loop/src/stream_journal.rs:195` refuses dispatch if `started` already contains the ID; entries survive subsequent invocations in the turn.
- `rust/core/src/persistence/transcript/projection.rs:58` also rejects a repeated request/attempt/tool-ID key.
- `rust/cli/src/interactive/stream_handlers.rs:158` makes successful dispatch admission mandatory before tool execution continues.

## Reproduction to automate

1. Run a background turn through the Gemini streaming path with a deterministic provider fixture.
2. Have the provider request `read_file` for one path and capture the genuine result.
3. Have it request `read_file` for a second path in the same turn.
4. Observe that the repeated provider ID can fail journal admission or dispatch rather than execute the second valid invocation.

Expected: both invocations execute once, retain distinct durable identities,
and replay with their own arguments, results, and lifecycle states.

## Suggested repair direction

Introduce a distinct internal invocation identity and propagate its relationship
to provider IDs through admission, dispatch, results, cancellation, and replay.
Do not merely clear `started`: projection keys and result identities must also
distinguish genuine invocations while rejecting duplicate dispatch of one call.
Preserve provider-facing IDs and Gemini signatures where the protocol needs them.

## Acceptance and validation guidance

- Two sequential same-name Gemini calls succeed and have separate results.
- Repeated calls across response attempts remain distinct.
- Duplicate dispatch of the same internal invocation is still refused.
- Cold recovery never repeats an unknown side effect or merges the two results.
- Cover the actual provider-to-background-output path, not just a handcrafted journal.

Related specifications: `spec/features/incremental-transcript-journal.feature`,
`spec/features/process-crash-transcript-recovery.feature`, and
`spec/features/transcript-executor-admission.feature`.
