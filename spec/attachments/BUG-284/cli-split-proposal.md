# BUG-284 CLI extraction approval proposal

No extraction or production edit has been performed. Explicit human approval is
pending. The requested scope is limited to the existing
`rust/cli/src/interactive/stream_loop.rs` and new sibling-directory modules under
`rust/cli/src/interactive/stream_loop/`. Narrow session admission wiring remains
under the previously authorized `rust/cli/src/session/` scope.

## Mechanical stage, before admission behavior changes

Snapshot the current file and its hash (included in the RED source manifest).
Preserve public exports and signatures. Replace the giant local-state function
with typed orchestration and helper functions, not source-text composition,
fragments, an oversized moved function, or an alternate stream-loop bypass.
Every result file must be fewer than 300 lines including imports and comments.

The following source coordinates identify the current semantic regions. These
are reviewable extraction boundaries, not proposed textual `include!` fragments.
Where a region crosses Rust scopes, use explicit state/return types to preserve
the same branch order and execution effects. Minor punctuation-only boundary
adjustment is expected; moving behavior between branches is not.

| Current lines | Proposed module(s) | Preserved responsibility |
| --- | --- | --- |
| 1-128 | `stream_loop.rs`, `annotations.rs` | imports, documentation, annotation helper |
| 129-207 | `stream_loop.rs` | two existing public async entry functions |
| 208-265 | `driver.rs`, `preflight.rs` | internal signature, attribution, setup, TurnStart |
| 266-369 | `preflight.rs` | selected preparer, threshold, pre-prompt compaction, thought signatures |
| 371-473 | `telemetry.rs` | token state, capture, progress callback registration |
| 474-494 | `request.rs` | effective prompt and clone-before-append request handoff |
| 496-524 | `telemetry.rs`, `poll.rs` | response-start capture, optional CLI status |
| 525-617 | `state.rs`, `telemetry.rs` | buffers/counters, previous usage, seeded display/context projection |
| 619-777 | `restart.rs` | typed replacement for in-loop compaction restart macro |
| 779-939 | `poll.rs`, `interrupted.rs` | interruption priority, settlement, CLI/notify/stall selection |
| 941-965 | `attempt.rs`, `driver.rs` | event match and attempt snapshot/rollback |
| 966-1043 | `assistant.rs` | text, tool-call, reasoning-delta handling |
| 1044-1165 | `tool_result.rs` | tool result, handoff, early done settlement |
| 1166-1196 | `usage.rs` | per-API usage observation and display |
| 1197-1312 | `final_usage.rs` | terminal-response usage, token projection, capture |
| 1313-1395 | `final_continuation.rs` | Gemini continuation and normal final response append |
| 1396-1578 | `final_thinking.rs` | thinking-exhaustion retry before clean finalization |
| 1579-1626 | `final_compaction.rs` | FinalResponse clean-exit compaction check |
| 1627-1819 | `final_completion.rs` | auto-continue/goal decisions and final done emission |
| 1820-1861 | `recovery.rs` | terminal owner/request guards, authoritative error classification |
| 1862-1961 | `recovery_hook.rs` | typed PromptCancelled reconciliation and compaction policy |
| 1962-2085 | `recovery_context.rs` | stall terminal guard, overflow compaction branches |
| 2086-2116 | `recovery_image.rs` | image sanitization retry |
| 2117-2206 | `recovery_truncation.rs` | truncated-tool retry and exhausted budget |
| 2207-2311 | `recovery_network.rs` | network retry and exhausted budget |
| 2312-2383 | `terminal_error.rs` | terminal error capture, tool tail policy, propagation |
| 2384-2427 | `stream_end.rs`, `driver.rs` | stream-end safety, output flush, loop closure |
| 2429-2523 | `finish.rs` | callback cleanup and post-loop compaction safety net |
| 2524-2564 | `finish.rs` | token-tracker publication and final result |

`driver.rs` must retain one control loop and explicit typed outcomes such as
PollAgain, Restart, Stop and Error. Retry helpers must not independently spawn a
second inference loop. Keep retry stream construction where the prior stream
is explicitly dropped; preserve all statement-order constraints. State may be
factored into small nested typed structures if imports/methods approach 300
lines. Do not hide a larger file behind module aliases or lint suppressions.

## Mechanical-stage verification before semantic edits

- Record all original match arms, guard order, return/break/continue sites and
  stream drop sites against the source hash and coordinate inventory above.
- Preserve effective_prompt selection, request_history_len, clone-before-append,
  interruption-before-next-poll, response-attempt rollback, and exact per-segment
  reset values.
- Preserve tool-call/result settlement ordering, early-done and queued-summary
  handoff, auto-continue/goal counters, network/truncation/thinking budgets,
  compaction cascade policy and final tracker updates.
- Run the new BUG-284 harness and focused existing BUG-276, BUG-278/282,
  interruption/tool-pair, rollback and compaction tests. BUG-284's known failures
  must remain the same causal RED at this purely mechanical stage.
- Review the extraction diff and file lengths before adding admission semantics.
  Native coverage bindings affected by extraction must be migrated without
  weakening their assertions or changing unrelated specifications.

## Semantic stage after mechanical verification

Only then connect the new durable receipt/admission API to `request.rs` and the
session admission helper. Pass a typed identity/receipt so a genuine admitted
input occurs once in runtime and once in the effective request. Do not dedupe by
text. Pre-prompt compaction and construction errors must not strand the durable
original outside runtime. No strict provider/projection/generation/DAG guard is
relaxed. No TUI/RPC/Cargo/provider production change is proposed.

This proposal is intentionally separate from the completed investigation. It
does not authorize implementation, installation, restart, history migration,
commit or push. It exists so the exact remaining approval boundary is explicit.
