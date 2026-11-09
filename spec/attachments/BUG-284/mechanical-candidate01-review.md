# BUG-284 approved mechanical extraction: semantic audit

Status: candidate-01 static comparison completed against the preserved baseline;
no functional mismatch was found in the 22 reviewed files. Final compiled stable
hashes and focused behavioral test receipts remain pending. This is not a GREEN
or live-repair claim. Native session
`22032f8b-b6b7-45ea-a47f-5b25017b31f4` exclusively owns edits and builds.

## Bound baseline and scope

The original `rust/cli/src/interactive/stream_loop.rs` was captured at
2026-10-28T22:08:29.451112+00:00, before extraction: 2,564 lines, SHA-256
`3cbfd1d44f21d7b54515d8b4f6b68e320a150584a1bd8083d2c96474abe7790e`.
It was byte-equal to the same path at HEAD. Copy and capture metadata:
`/tmp/compaction-investigation/mechanical-review/stream_loop.before.rs` and
`before.json`. `control-sites.before.json` is a search index, not a parser or
an equivalence proof. The reviewed proposal is
`spec/attachments/BUG-284/cli-split-proposal.md`.

The user approved the focused extraction preserving cancellation, retry,
rollback and settlement ordering on 2026-10-28 22:01:24 UTC. The proposal still
contains pre-approval wording. No permission for install, restart, history
migration, commit or push is inferred. Review performs no repo edits/builds.

## High-risk original ordering contracts

Coordinates below refer to the immutable baseline, not a moving working tree.

| Baseline | Required ordering and branch meaning |
| --- | --- |
| 232–265 | Configure tool attribution (fallible await), require inference readiness, create cancellation token, configure settlement agent, wrap progressive output, generate request identity/time, then TurnStart. Moving fallible preflight across TurnStart changes emitted lifecycle. |
| 266–369 | Build/install selected preparer, resolve threshold, pre-prompt compaction before prompt append, emit continuing only on compaction success, then ensure thought signatures. Preflight failures currently propagate directly; extraction must not route them into network/overflow recovery. |
| 371–473 | Seed token state with cache-inclusive input and zero cache split, install hook, record request, register progress callback under real session ID and shared active tool ID before stream creation. Callback cleanup currently occurs only on normal post-loop path, not every error exit. |
| 474–494 | Select effective prompt (`Continue` only after pre-prompt compaction), capture `request_history_len`, await rig cloning history/constructing stream, then append exactly one user message with supplied images to session history. Mechanical stage must preserve this order and length origin; later receipt semantics must be reviewed separately. |
| 525–617 | All counters/buffers and response-attempt snapshot start once per user turn; previous reasoning/output bases and finalized totals are separate values. Do not conflate per-stream, per-API, per-turn and cross-turn state. |
| 619–777 | Compaction restart increments budget before doing work; selected policy determines retry text. Sub-agent path installs mapped preparer with retry prompt before compaction. Non-sub-agent path explicitly resets the existing token-state Arc; not a new Arc. Both create new hook, replace primary stream, clear only listed buffers, and reseed display with zero output/cache and original reasoning base. This macro has nonlocal error exits and requires caller `continue`. |
| 779–828 | Interruption is checked before the next stream poll: drop stream, acknowledge, await tool cancellations, dequeue input, await Interrupted settlement, preserve partial assistant text, await Done settlement, then break. Helpers must not keep an owned stream alive during settlement. |
| 830–939 | Poll selection has CLI event/status, Notify and fallback modes. Tool-in-progress uses 24-hour stall limit. Escape stores interrupt but returns no chunk so the next loop iteration handles settlement. Stall exits preserve partial text, emit stall Done and return error directly, bypassing generic classifiers and post-loop cleanup. |
| 944–965 | Attempt start clears input observation and snapshots text length, reasoning length and full display. Abort clears observation, takes snapshot, truncates both buffers, restores display, emits abort. Do not broaden rollback to committed tool/history state or omit display rollback. |
| 966–1043 | Text/tool/reasoning each reset network count and reconnect flag before content handling. ToolCall handling precedes annotation, activity accounting, active-ID update and tool-in-progress=true. Reasoning is accumulated separately from assistant text. |
| 1044–1165 | Persist tool pair first, update annotation, clear active-ID/in-progress, then check compaction handoff before done acceptance. Successful handoff blocks inference and breaks without early-done path. Failed handoff blocks inference and returns error. Accepted done drops stream before awaited cancellations, flushes text, rotates annotations, applies shared summary teardown, emits Done, breaks. |
| 1166–1312 | Usage handling resets network retry, records physical input and separate display segments; final response uses physical request context for current input but aggregated completion usage for billing/output. Finalized output and reasoning bases are updated before continuation. |
| 1313–1395 | Gemini continuation precedes normal assistant append. Completed adopts returned text/display/usage/stop/activity; Finished only adopts display then breaks; CompactionNeeded clears outer text, restarts same primary loop and continues. Normal assistant text is appended before thinking recovery. |
| 1396–1578 | Thinking retry requires no interruption, increments per-turn count and increments cross-turn count only on first detection. Downgrade affects subsequent turns. A second interruption check continues to top-level settlement. Retry clones current history, then appends recovery prompt, resets exact local buffers/display, continues. Exhaustion warns and falls through, rather than returning error. |
| 1579–1626 | Rotate annotations before clean-FinalResponse compaction guard. If flag set and no interruption: clear already-appended text, begin recovery with pop_user_prompt=false, restart and continue before any auto-continue or done emission. |
| 1627–1819 | Take done acceptance, settle pending refund, update zero-activity count, then choose goal/ordinary decision. Nudge increments used, sets refund flag, clears segment activity, emits state, clones history into retry then appends nudge, resets exact buffers/display, continues. Other decisions fall through to one Done and break; only this clean completion branch may auto-nudge. |
| 1820–2085 | Error cascade order is terminal overflow owner, terminal request-failure guard, typed PromptCancelled, stall terminal guard, then two sequential structural-overflow checks. Do not merge/reorder apparently redundant checks in the mechanical stage. Typed cancellation reconciles Rig history, drains buffered calls, inserts orphan outcomes, then invokes recovery and restarts. |
| 2086–2311 | Image sanitation pops trailing user before sanitizing and can break successfully. Truncation increments its own budget, retains accumulated reasoning, and returns Err on exhaustion. Network retries use provider-aware policy, sleep before saving partial text, and break directly if interrupted after sleep; they clear reasoning and fall through to terminal handling on budget exhaustion. |
| 2312–2427 | Terminal error capture precedes legacy tail recovery; Codex never enters that removal path. Stream end saves text, rotates annotations, emits Done, breaks. Chunk flush occurs only after a matched event completes normally; all break/continue/return/? and no-chunk paths skip it. |
| 2429–2564 | Normal-break finish clears actual-session callback. Post-loop compaction is suppressed for summary handoff/interruption, uses empty text and no user pop, never restarts inference or increments budget, and logs/swallows compaction errors. Tracker publication separately skips only interruption, computes output delta before display mutation and reloads billed totals from durable state. Its durable-load error still propagates after Done. |

The independently checked full recovery inventory is
`/tmp/compaction-investigation/mechanical-review/recovery-review.md`
(current SHA-256 `0e6f4c7318a553cd59fb762559bc520fad9396aac5f829ab218c7ccb0cd46c15`,
including its candidate-01 follow-up).
It records exact retry reset differences and each terminal branch.

## Equivalence risks that file splitting alone cannot address

- `break` from the driver means run shared finish; `return Err` and `?` currently
  skip it. A single Stop/Error enum must preserve that distinction, including
  whether per-chunk `progressive_output.flush()` runs.
- Direct `drop(stream)` exists at interruption and accepted-done settlement.
  Assignments on retry construct/await the replacement before dropping the old
  binding. Introducing blanket drop-before-retry changes this lifetime; if
  intentionally required it is a semantic change needing separate evidence.
- Retry branches deliberately have different reset sets. Shared helpers must
  not casually reset network/truncation/thinking budgets, prior reasoning,
  finalized usage, response-attempt snapshots, nudge refunds or active IDs.
- The generic stream/final-response ownership boundary must not introduce a
  second inference loop or change which turn owns an error/retry.
- Existing causal REDs are necessary but cover only a subset of this state
  machine. Focused interruption/tool settlement, rollback, retry/compaction and
  completion tests plus manual branch comparison are still required.

## After-state review and verification

Candidate-01 was captured at 2026-10-28T22:24:22.083911+00:00. Every one of its 22
files exactly matched `mechanical-review/candidate-01-hashes.json` (manifest
SHA-256 `a7659336c4778d6d85b77331d186f4f89bcbd640e7e1efde914239aeb30ea31f`).
Immutable copies are in `mechanical-review/candidate-01-review/`, with the full
observed hash map and zero mismatches in `candidate-01-review-hashes.json`.

The parent auditor checked public entry points, driver, state, preflight,
telemetry, poll/interrupted, assistant, annotations, tool_result, usage,
final_response, final_thinking and final_completion. A second read-only auditor
independently compared recovery, recovery_compaction, recovery_network,
recovery_truncation, restart, terminal_error, stream_end and finish, including
their driver caller outcomes. These are semantic comparisons to executable
branches in the original, not a file-length or string-inclusion assessment.

| Contract checked | Candidate-01 result |
| --- | --- |
| Request handoff | Driver lines 100–112 retain effective prompt, original history length, awaited stream construction then user append. Images still enter the appended user content. No receipt/admission change is present in this block. |
| Stream ownership | BoxStream replaces the concrete local binding. Explicit drop remains immediately before interruption or accepted-done settlement; retry helpers assign `*stream = ...await.boxed()` so the old stream survives RHS construction as before. No alternate retry inference loop was introduced. |
| Exit and flush meaning | `Next` reaches the sole per-chunk flush, `Continue` skips flush and repolls, `Stop` skips flush and enters finish, and propagated errors skip finish. Tool handoff branches directly stop/error before checking done acceptance. |
| Polling and cancellation | CLI, Notify and fallback polling preserve no-chunk wakeups and stall limit. Stall emits partial text/error/Done and returns error directly. Interrupted settlement retains acknowledgement, cancellation, queue drain, Interrupted, partial text, Done order. |
| Attempt rollback and accounting | Snapshot contains the same text/reasoning lengths and display; abort takes/restores it and resets input observation. Only the five successful chunk categories reset network retry. Usage distinguishes physical input from aggregated output and tracks finalized reasoning/output separately. |
| Final completion | Gemini handling precedes assistant append, then thinking recovery, annotation rotation, clean-exit compaction, continuation/refund, and Done. Each continuation/retry keeps its original counter and buffer reset set. |
| Recovery guards | Terminal owner and request-failure guards stay first. `recovery_compaction` handles typed cancellation then stall and overflow. Its stall return false is followed by the caller's terminal stall guard before image/truncation/network handling. Thus extraction has not let a stall enter a retry branch. |
| Recovery side effects | PromptCancelled settles Rig/buffered/orphan tool outcomes before compaction. Truncation retains reasoning; network and compaction clear it. Interrupted network backoff remains Stop without new top-loop settlement. Images and legacy tail recovery retain their original successful-stop versus terminal-error behavior. |
| Finish | Callback cleanup is still normal-break only. Post-loop recovery is suppressed for handoff/interruption, does not restart, and still logs/swallows failures. Tracker updates preserve delta-before-display and durable billing reload; interrupted turns skip them. |

Actionable remaining verification: native must bind the final compiled candidate
to a fresh hash manifest, demonstrate the same causal BUG-284 RED at the
mechanical stage, and provide focused interruption/tool-pair, response rollback,
thinking/continuation and compaction/recovery results. Re-review any subsequent
source changes against the candidate snapshot. Admission behavior must remain
separately identified; this static comparison does not establish receipt
correctness, historical recovery, or live runtime repair.

Test migration inventory: `mechanical-review/source-test-references.json`
records loop-path/macro references (including comments, so not a count of
executable assertions). Source checks that actually bind the old layout include
`cmpct044_cascade_ordering_test`, `compaction_trigger_reliability_test`,
`cont005_done_immediate_termination`, `cont006_goal_immediate_termination`,
`auto_continue_engine_test`, `rpc087_error_recovery_wiring_shape`, token 001/002,
CMPCT-041 display seeds, BUG-149 active progress IDs and RPC398 callback keys.
Moving these checks to new owners must retain branch-local ordering, counts,
negative assertions and actual driver wiring; concatenating arbitrary sources
and finding matching strings would not establish the same contracts.
