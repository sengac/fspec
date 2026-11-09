# BUG-326 root cause: resumed sessions overstate input tokens

## Problem

After a cold resume, a session's `token_tracker.input_tokens` is larger than the
live value by the last reply's output (and reasoning) tokens.

- `rust/core/src/persistence/request_settlement.rs:19` (`replace_request_context`,
  called on every physical request from `rust/core/src/request_settlement_hook.rs:128`)
  writes `current_context_tokens = usage.total_tokens`. rig's `total_tokens` is
  input + cache_read + cache_creation + output + reasoning
  (`settle_request` in the same file checks exactly that identity, `:45-58`).
- Every other writer stores input only: `SessionManifest::update_token_usage`
  (`rust/core/src/persistence/manifest_types.rs:138`) and the compaction commit
  (`rust/core/src/persistence/compaction_commit.rs:180`).
- Every reader treats it as input: `rust/sessions/src/session_restore.rs:179`
  (`tracker.input_tokens = saved.current_context_tokens`),
  `rust/cli/src/session/restore_compaction.rs:89`,
  `rust/sessions/src/committed_publication.rs:77` (`saved_tokens`).
- The live tracker's input is `usage.total_input()` = input + cache_read +
  cache_creation (`rust/core/src/compaction/input_accounting.rs:65`).

So a turn with input 180000, cache_read 1000 and output 7 leaves the live tracker
at 181000 but a resumed one at 181007.

The restored context fill (`committed_publication.rs:109-111`) divides
`tokens.input_tokens` by the threshold, while the live request context fill
(`rust/rpc-types/src/request_context.rs:35`, `RequestContextSnapshot::context_fill`)
is inclusive occupancy: input + output of the latest request. Once the stored
value is input-only, the restored fill must add the latest request's output
explicitly to match live.

## Design

- `replace_request_context` stores the request's total input (input +
  cache_read + cache_creation, overflow-checked) in `current_context_tokens`
  and its output (output + reasoning) in a new
  `token_usage.current_context_output_tokens`. The compaction commit sets them
  to the admitted input and 0; `update_token_usage` sets both. One meaning per
  field.
- `TokenUsage::context_occupancy()` = `current_context_tokens +
  current_context_output_tokens`. The restored context fill
  (`committed_publication::restored_snapshot`) uses it with the live formula
  (`occupancy * 100 / threshold`, integer), so it equals
  `RequestContextSnapshot::context_fill` of the last request. Single source:
  the manifest's `token_usage`. (`latest_request_usage` was rejected as the
  source: a compaction commit makes it stale, which would need a second rule.)
- `output_tokens` on resume. Evidence: the live tracker's `output_tokens` is
  the session's cumulative reply output from `StreamingTokenDisplay`
  (`core/src/streaming_display/streaming_token_display.rs:238,295`
  `output.update_from_usage(usage.output_tokens)`), which excludes reasoning;
  reasoning is tracked separately in `reasoning_tokens`
  (`streaming_token_display.rs:246-320`). The manifest's
  `cumulative_billed_output` is `Σ(output + reasoning)`
  (`request_settlement.rs` `settle_request`). Restoring
  `output_tokens = cumulative_billed_output`
  (`sessions/src/session_restore.rs:180`, `committed_publication.rs:78`,
  `cli/src/session/restore_compaction.rs:90`) therefore counted reasoning
  twice (in output and in reasoning). It was wrong; all three paths now use
  `TokenUsage::reply_output_tokens()` = `cumulative_billed_output -
  reasoning_tokens` (an inconsistent manifest where reasoning exceeds billed
  output is an error, not clamped). `cumulative_billed_output` itself is
  unchanged (it is the billed figure and includes reasoning).

## Alternatives rejected

- Subtracting output at restore time: keeps a field with two meanings
  depending on the writer; any other reader stays wrong.
- Storing inclusive occupancy everywhere: the field is read as input by three
  restore paths and the compaction threshold logic; changing all of them is
  larger and the live tracker models input separately.

## Wire / API changes

None on the RPC wire. Persisted manifest: `token_usage.current_context_tokens`
always means the latest request's total input; new
`token_usage.current_context_output_tokens` (u64, `#[serde(default)]` like
every sibling field) holds its output. Restored `TokenTracker.output_tokens`
(and the `TokenUpdate`/`CommittedCompaction` chunks built from it) no longer
include reasoning.
