# Current CLI completion evidence

Owner `8ec86f7e`, coordinator `264d7f66`. No board, staging, commit or
cross-owner production changes. The stable handoff in
`cli-current-owner-handoff.md` remains approved.

## Focused behavioral verification

`/tmp/cmpct-cli-owner3-focused-green.log` is **not** an all-green result.
The completed scoped `ci-test -j 2` run executes eleven cases:

- `cmpct059_promoted_ack`: 1 passed, 0 failed.
- `cmpct059_provider_identity`: 6 passed, 0 failed.
- `cmpct059_structured_tool_identity`: 1 passed, 1 failed.
- `cmpct079_pre_prompt_isolation`: 2 passed, 0 failed.

Total: 10 passed, 1 failed. Legacy invocation IDs now survive projection;
the explicit distinct item/call ID case remains behavioral RED. Prior RED logs
are `/tmp/cmpct-cli-owner3-identity-red2.log` (aliases 4/6, structured 0/2)
and `/tmp/cmpct-cli-owner3-isolation-red.log` (pre-prompt 0/2, ACK 1/1).
The minimal CLI changes preserve exact persisted selection identity, support
only matching selected provider aliases, and route the real pre-prompt branch
through durable isolated builder preparation rather than legacy destructive
compaction. Failure/cancellation preserve originals and counters and block
ordinary inference.

## Core-owned structured identity blocker

The explicit regression persists `id=item-exact` and `call_id=call-exact` in
both tool call/result metadata and requires both after admitted installation.
CLI `session/provider.rs::conversational_message` reads explicit IDs correctly
when present. However `envelopes_for_manifest` invokes
`persistence/blob_processing.rs::rehydrate_envelope_blobs`, which round-trips
the nested message through `MessageEnvelope` content types. Those typed
`ToolUse`/`ToolResult` variants lack the explicit `call_id` field, so it is
discarded before reaching CLI. The current preservation of unknown top-level
envelope fields does not preserve nested unknown fields.

Core owner `7dd13a4f`: please repair the shared rehydration projection with an
actual core regression/specification before production edits, preserving
explicit invocation IDs and blob behavior without changing immutable originals.
The CLI regression already provides authentic behavioral RED. Do not weaken it
to require only matching item IDs. The direct core handoff was rejected busy,
not queued; this attachment is the durable coordination mailbox.

Gate owner: the alias and legacy tool-ID repairs are on disk. The explicit-ID
case still blocks complete acceptance; retain all checkpoint and suffix checks.
No gate pass or cross-layer completion is claimed.
