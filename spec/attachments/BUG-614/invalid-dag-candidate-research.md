# "summary submission blocked ordinary inference: invalid DAG candidate" — research

Session 21b391d2-21e5-40e2-bbe3-dac3612c099a, project /home/rquast/projects/finance,
provider codex/gpt-6.1-sol, created 2026-11-07T04:03:37Z. Read-only investigation.
Repo HEAD 617ce182a. The running daemon was built from b7e6d5ea5 or a little earlier.

## TL;DR

* **This was not an output-limit truncation.** The **main model** called `inject_summary`
  on its own initiative. The candidate was a **complete markdown handoff note with zero
  `<dag-node>` elements** (19,885 chars, 97 lines, about 4.6k output tokens). The model's
  output limit is 128k tokens.
* No compaction had been requested. Context was about 105k tokens and no compaction
  instruction appears anywhere in the session. `inject_summary` is in every ordinary
  turn's toolset. Its description says "hierarchical DAG summary" but never gives the
  syntax.
* The validator rejected the candidate with `nodes.is_empty()`. `requires_handoff`
  treats any result other than "queued" as fatal. The turn was killed, and the error never
  went back to the model. The session was then blocked: nothing in this flow clears the
  block except a successful commit or a cold restore.
* CMPCT-084 (chunked continuation), CMPCT-087 and BUG-232 (whole-candidate correction,
  3 attempts) **exist only inside the compactor sub-agent builder** (`build_validated_dag`).
  The main model's `inject_summary` path has neither.
* **HEAD behaves identically.** None of the 41 commits in b7e6d5ea5..HEAD touch
  compaction, handoff, inject_summary or the validator.

## 1. What happened

### Sources
- Transcript: `~/.fspec/transcripts/21b391d2-.../` (988 events, all but one in request
  `1a1daafc`, a single 60-minute turn).
- Session manifest: `~/.fspec/sessions/21b391d2-....json` (`compaction: null`,
  `compaction_commit: null`, 296 messages).
- Messages: `~/.fspec/messages/messages.jsonl`.
- Input receipts: `~/.fspec/input-receipts/21b391d2-.../`.
- History: `~/.fspec/history.jsonl` (4 user prompts).
- Log: `~/.fspec/logs/private/fspec-combined.log.2026-11-07:2021-2033`.

### Timeline (UTC)
| time | event |
|---|---|
| 04:03:59 | User: "please continue where you left off last session". One turn (request 1a1daafc) runs as an orchestrator for the whole hour. |
| 04:15:42, 04:27:51 | Steered user inputs (40 GB cargo guard rule; "get rid of flutter and egui"). |
| 04:56:18 / 04:56:23 | User: "I hope you're using rust to build all of this - DO NOT USE OTHER LANGUAGES". Delivered as a steer (RIG-016). |
| 04:56 to 05:02:06 | The model relays the Rust-only rule to sub-agents through AgentManager and does SessionSearch and other work. |
| 05:02:06 | Last attempt starts. No thinking or text events, then 95 s of generation. |
| 05:03:41.885 | `tool_requested name=inject_summary` (seq 983) with args `{content, work_unit_id:"MP-086"}`. |
| 05:03:41.903 | `tool_completed`, content `invalid DAG candidate: missing or unclosed dag-node structure` (seq 985, `is_error:false`). Persisted as message d21bc329. |
| 05:03:42.017 | `terminal outcome=provider_error`. Log: `summary submission blocked ordinary inference: invalid DAG candidate: ...`. The first input receipt's `execution.state` is `failed` with the same reason. |
| 05:06:25.46 | Next input. Its receipt 5f9b958b has `origin.kind = "supervisor"`, `source_session = e810a6f4...`: a **sub-agent's AgentManager report**, not a user prompt. |
| 05:06:25.74 | Fails 0.3 s later with `execution.state = failedBeforeDispatch` and the same reason (driver.rs:67 gate). |

### Who submitted it
`inject_summary`, called by the **main model** (codex gpt-6.1-sol) in an ordinary turn. It
was not GenerateCompaction and not the compactor sub-agent. No compaction instruction
(`COMPACTION_INSTRUCTION_FRESH` or `_INCREMENTAL`) was ever delivered: no session message
contains "compaction", "dag-node" or "GenerateCompaction". No transcript event explains
why the model chose to call it, because the final attempt has no thinking. The title
("MarketPulse continuation orchestration") and section 1 ("User requests / hard
constraints (most recent highest priority)", which includes the 04:56 Rust-only rule)
suggest the model was trying to pin a handoff note.

### The submitted text (shape only)
- 19,885 chars, 19,901 bytes, 97 lines, 2,151 words. Markdown headings: `# MarketPulse
  continuation orchestration`, `## User requests / hard constraints`, `## Developer / repo
  rules`, `## Current active subordinate agents` (6 `###` entries for sub-agent sessions),
  `## Completed workers`, `## Actual legacy retirement`, `## Repo baseline / environment`,
  `## Next orchestration steps`.
- `<dag-node` count is **0**, `</dag-node` is 0 and `<dag-files>` is 0. Only 2 `<`
  characters appear anywhere.
- **It ends cleanly**: the final numbered step "7. Final concise user update ... Do not
  imply full UI migration finished." is followed by `\n`. The tool-call JSON parsed and
  the handler ran, so the arguments were not cut off.
- Many words are run together in the later sections ("nativeRust review
  preservingproof"). The model was compressing on purpose; this is not a transport
  artefact.

### Output limit
- The transcript records no stop or finish reason. This journal only stores text,
  thinking, tool and terminal events.
- In `request_settlements`, the only settlement with large output is `d9e8ae92`:
  output_tokens 4,609, reasoning 61, total 110,553. The next largest is 450. This is
  almost certainly the inject_summary generation, since about 19.9k chars of compressed
  text comes to roughly 4.6k tokens.
- Catalog limits for gpt-6.1-sol (`~/.fspec/cache/models.json`, every provider entry):
  context 1,050,000, input 922,000, output 128,000. Codex's live /models value was not
  captured in the logs. Either way, 4.6k is far below any output limit.
- **Conclusion: no truncation.** The candidate failed because it was prose, not DAG.

### What the tool result said
Exactly `invalid DAG candidate: missing or unclosed dag-node structure`, with no syntax
help, no attempt count and no "fix and resubmit". It was stored as a tool result, but the
model never got another request in which to read it.

### Does the 04:56 steer matter?
Not mechanically. Steering only injects a user message (BUG-546 makes it wait for tool
calls) and has no interaction with the handoff code. Its likely effect was on content:
the new hard constraint gave the model a reason to write a "continuation constraints"
note. The 05:06 failure was not the user retrying. It was a background supervisor
notification, which BUG-211 rules say can never authorize a builder retry anyway.

## 2. Why CMPCT-084 and the "actionable whole candidate correction" didn't save it

Both features exist only inside the compactor **sub-agent** builder:

- `rust/cli/src/compaction_operation.rs:161-202` `build_validated_dag`: 3 attempts.
  After each full candidate it calls `validate_dag_candidate`. On error it rebuilds the
  task as prompt + "Validation error: {error}" + DAG_NODE_SYNTAX + DAG_RANGE_RULES +
  the rejected candidate (194-197). After 3 attempts it returns Err (183-186).
  `InvalidSource` returns immediately (182).
- `rust/agent-loop/src/compaction_chunking.rs:119-165` `synthesize_in_chunks`
  (CMPCT-084). It applies only when the provider stop reason is
  `length|max_tokens|max_output_tokens` (28-35). It keeps the complete-node prefix and
  asks for the rest, up to 8 chunks with a 2-stall limit (17-18).
- Wired up at `rust/agent-loop/src/compaction_builder.rs:107-148`. The prompt comes from
  `rust/cli/src/compaction_dag.rs:183-245`.

The main model's path has none of this:

1. `providers/src/codex/agent.rs:91` registers `InjectSummaryTool` for **every ordinary
   turn**. The same is true in claude.rs:562, openai/agent.rs:121, gemini.rs:230,
   zai.rs:288, custom/custom_provider.rs:304,330 and copilot/rig_agent.rs:97.
2. Its description (`rust/tools/src/inject_summary.rs:136-142`) and schema ("The DAG
   summary content to inject") contain **no DAG syntax**.
3. `rust/agent-loop/src/generate_compaction_handler.rs:136-145` sends the content to
   `TurnCompaction::submit`.
4. `rust/agent-loop/src/turn_compaction.rs:86-95` runs `validate_dag_candidate` once and
   returns the bare error string. `submitted` is only set on success (97-98), so a second
   submission would technically be allowed.
5. `inject_summary.rs:171-176` turns the error into a tool error with the bare message.
6. `rust/cli/src/interactive/compaction_handoff.rs:45-76` `requires_handoff`: tool
   `inject_summary` with status not `queued` gives `Err(CompactionHandoffError)` (72-75).
7. `rust/cli/src/interactive/stream_loop/tool_result.rs:62-66` calls
   `block_compaction_inference(reason)` and `return Err`, which **ends the turn at the
   tool boundary**. The model never sees its own tool result.
8. `compaction_handoff.rs:10-15` `block_compaction_inference` also calls
   `prevent_fresh_retry()` on any existing CompactionOperation.

So validation happens only after submission, and only once. There is no loop back to
the model and no syntax guidance. Chunked continuation does not apply: it is a different
code path, and nothing was truncated anyway.

### Why the session stays blocked
- `stream_loop/driver.rs:67` `ensure_compaction_inference_ready()?` fails every later
  turn.
- `agent-loop/src/agent_loop.rs:228-241` retries a failed builder only when
  `pending.is_fresh_user_input` is true. The 05:06 input was a supervisor notification.
- Even a fresh user input would not help. `restart_failed_compaction_builder`
  (`compaction_handoff.rs:17-24`) needs a `compaction_operation` with
  `retryable_builder_failure`. Here there was either no operation, so it returns false,
  or `prevent_fresh_retry()` had cleared the flag. **Result: blocked until a restart.**
- The only places the block is cleared are a successful commit
  (`cli/src/session/compaction.rs:192`), a restore (`cli/src/session/restore_compaction.rs:80-82`,
  reached from `sessions/src/session_restore.rs:71,102` and from
  `agent-loop/src/agent_loop/prompt.rs:101-110` only after 3 automatic recoveries), and
  discarding an in-view working view (`session/recovery_view.rs:69-75`).
- The block lives only in memory and the manifest has no pending commit, so a daemon
  restart and cold restore should clear it. This has not been tested.

Earlier cards: BUG-232 (same error string, 2026-10-21) wrote down "likely initiating
causes: model returned prose/no DAG, unmatched tags or truncated output", but its fix
went into the builder. CMPCT-084's evidence was qwen compactor runs at 4096 max_tokens,
also the builder. None of the cards covers the main-model `inject_summary` route.

## 3. HEAD

`git log b7e6d5ea5..HEAD` has 41 commits: graph, WT-015 path refactors, TUI, RPC, audit
and spec chores. None of them touches `compaction_handoff.rs`, `stream_loop/`,
`core/src/compaction/`, `session/`, `tools/src/inject_summary.rs`, `turn_compaction.rs`
or `compaction_operation.rs`. The two "compact" files in the diffstat
(`fspec-core/src/commands/compact_work_unit.rs`, `generate_summary_report.rs`) are
unrelated work-unit commands. **HEAD reproduces the same failure.** The file:line
references above are all at HEAD 617ce182a.

## 4. Every path that produces a DAG candidate

| Path | Who generates | Can the output limit truncate it? | Validated before final? | Exact error back to the generator to fix? | End state on failure |
|---|---|---|---|---|---|
| **inject_summary** in an ordinary turn (this incident) | main model | Yes (tool args). PROV-040 truncated-tool-call recovery (`stream_loop/recovery.rs:84`) is triggered only by the Anthropic transport string (`patches/rig-core/.../anthropic/streaming/transport.rs:192`). For Codex/Responses this was not verified. | Only once, at submit (`turn_compaction.rs:86-95`) | **No.** The turn is killed (`tool_result.rs:62-66`) | Blocked; fresh retry suppressed. **Permanent until restore.** |
| **inject_summary in the in-view builder view** (`session/recovery_view.rs:16-56`, via `interactive_helpers.rs:596-605`; used when no compactor handler is registered, `stream_loop/restart.rs:30-31,72`, and always by the CMPCT-032 post-loop safety net, `stream_loop/finish.rs:42`) | main model, given FRESH/INCREMENTAL + DAG_NODE_SYNTAX | Same as above | Once, at submit | **No** | Same as above |
| **GenerateCompaction, self** (`generate_compaction_handler.rs:154-161`) | compactor sub-agent | Yes; handled by CMPCT-084 chunks (if a stop reason is reported) | Yes: `build_validated_dag` after each candidate, then again at `submit_with_deadline` | Yes, up to 3 whole-candidate attempts inside the sub-agent | After exhaustion the tool error is not "queued", so `requires_handoff` gives Err: blocked + `prevent_fresh_retry`. **Permanent.** |
| **GenerateCompaction, other target** (`generate_compaction_handler.rs:162-185`) | compactor sub-agent | Same as above | Same, plus apply on the target | Yes (3). Exhaustion comes back to the *calling* model as an ordinary tool error | Caller continues (`requires_handoff` returns Ok(false) for non-self). Target is untouched unless application is cancelled (`turn_compaction.rs:124-127`). |
| **Threshold / pre-prompt / in-stream hook / terminal overflow** (`compactor_sub_agent.rs:125-270`, from `stream_loop/preflight.rs:94`, `restart.rs:61`, `agent-loop/src/terminal_overflow_recovery.rs:128`) | compactor sub-agent | Same as above | Yes: builder 3x, then `validate_dag_structure` (194-195), then commit validation | Yes (3) | Blocked with `retryable_builder_failure`. A **fresh user input** retries with a new budget (`compactor_sub_agent.rs:272-300`, `agent_loop.rs:228`). Background notifications do not (BUG-211). |
| **Manual /compact** (BUG-516, `stream_loop/manual_compaction.rs:17-71`) | compactor sub-agent (same round) | Same as above | Same | Yes (3) | Emits "Compaction failed: ..." and Done. Calls `restart_failed_compaction_builder` first (49), so repeating /compact retries a retryable failure. It does not recover an inject_summary block, because that path set `prevent_fresh_retry`. |

## 5. Proposed fix (in line with the user's words)

Principle: **the generating model gets the exact validator error and must fix the DAG
before the operation finishes. Rejection never installs anything and never leaves the
session permanently blocked.** Validation stays strict: no salvage, clamping or invented
coverage, as BUG-232's rule requires.

### A. inject_summary (main model): turn rejection into a correction loop
1. In `TurnCompaction` (turn_compaction.rs), count rejected submissions per operation.
   On a validation error (`InvalidCandidate`), return a structured, non-fatal tool
   result such as:
   `{"status":"rejected","attempt":n,"max_attempts":3,"error":"<exact validator error>",
   "syntax":DAG_NODE_SYNTAX+DAG_RANGE_RULES+FILE_METADATA_INSTRUCTION,"installed":false,
   "instruction":"Call inject_summary again with a complete corrected DAG (not a patch)"}`.
   Also return the persisted turn coordinates the DAG must cover (`source.prefix.len()`).
2. In `compaction_handoff.rs:45-76`, add an `Ok(false)` outcome for `status:"rejected"`
   (or a typed `HandoffOutcome::Rejected`). `tool_result.rs:54-69` then lets the loop
   continue, so the model sees the error in the same turn and resubmits. No block is set,
   because nothing was cleared: the production TurnCompaction never touches
   `session.messages` before commit, and the "preserves memory" half of the scenario
   still holds.
3. Truncation: if the inject_summary tool call is cut off by the output limit (Responses
   `incomplete`/`max_output_tokens`, Anthropic `max_tokens`), give the same feedback:
   "your DAG was truncated after N complete nodes; resubmit with fewer, shorter nodes".
   Reuse `complete_prefix` and `instruction()` from `compaction_chunking.rs`.
   Optionally allow `inject_summary(content, continuation: true)` so it appends to the
   kept prefix, mirroring CMPCT-084 (see question Q5).
4. Put DAG_NODE_SYNTAX into the `inject_summary` tool description
   (`tools/src/inject_summary.rs:136-142`). This incident's model had never been shown
   the format.
5. After 3 rejections (the fixed limit): the tool returns
   `{"status":"rejected_final","installed":false,"error":...}`. The turn continues on the
   unchanged context, the TUI shows a warning "Summary not installed: <error>", and
   **inference is not blocked**. If the session really is over the compaction threshold,
   the normal preflight/hook compaction on the next request runs the validated
   sub-agent builder.

### B. Compactor sub-agent (GenerateCompaction, threshold, manual)
1. Keep `build_validated_dag` with its 3 attempts. Validate each chunk's completed nodes
   as soon as the chunk ends, not only the final assembly, so a malformed node triggers
   correction right away.
2. When a candidate fails validation and ends mid-node (the `closed=false` signal already
   logged at compaction_operation.rs:190-192) but the provider gave no output-limit stop
   reason, treat it as truncated and *continue*, as `synthesize_in_chunks` does, instead
   of asking for a whole new candidate that will be cut off again.
3. GenerateCompaction self, after exhaustion: return a non-fatal
   `{"status":"rejected_final", ...}` result to the caller. Do not block and do not call
   `prevent_fresh_retry`. Same end state as A5.
4. Threshold, pre-prompt and terminal-overflow exhaustion: committed memory stays
   unchanged and the operation stays retryable. **Any** next input (or /compact) starts a
   fresh operation with a fresh 3-attempt budget, so the session is never permanently
   stuck. Whether ordinary inference may proceed uncompacted while the context still fits
   is question Q3.
5. Only a `prevent_fresh_retry`-style permanent block should remain, and only for real
   durable-authority failures (publication or identity loss, pending commit). It should
   never follow a candidate rejection. Even then, a restore path must stay reachable from
   the UI.

### End state when attempts run out
Nothing is installed. Committed messages, counters and originals are unchanged. A visible
error carries the exact validator message, operation id and attempt count. The session is
**usable**: either the current turn continues (inject_summary, GenerateCompaction self),
or the session goes idle and the next input or /compact retries with a fresh budget
(threshold compaction). The block only survives where durable state is ambiguous, which
is not the case for a rejected candidate.

### Open product decisions
- **Q1.** Should ordinary turns keep `inject_summary` (with syntax and the correction
  loop), **or** should it be removed outside an in-view compaction view, leaving
  self-compaction to GenerateCompaction's validated builder?
- **Q2.** When a spontaneous inject_summary exhausts its attempts, should the current
  turn **continue normally**, **or** end with a visible error (unblocked) so the user
  decides?
- **Q3.** When threshold compaction exhausts its attempts and the context still fits the
  model window, should ordinary inference **proceed uncompacted with a warning**, **or**
  stay held until a compaction succeeds, with every next input retrying?
- **Q4.** Should background or supervisor notifications (AgentManager reports) be allowed
  to start a fresh builder retry, **or** only fresh user input and /compact (today's
  BUG-211 rule)?
- **Q5.** For a truncated inject_summary: should the model **resubmit a whole shorter
  DAG**, **or** should inject_summary gain a continuation mode that appends to the kept
  complete-node prefix (as CMPCT-084 does)?
- **Q6.** Attempt budget: should there be **one shared limit** (for example 3) across
  continuation and correction, **or** separate limits (today: 8 chunks + 2 stalls for
  continuation, 3 for correction)?
- **Q7.** Should a correction **continue the same generating conversation** (append the
  assistant candidate plus a user error message), **or** start a fresh synthesis request
  that embeds the rejected candidate (today's CMPCT-087 behaviour)?
- **Q8.** Should validation run **incrementally while streaming** and abort at the first
  malformed closed node, **or** only at the end of each chunk or candidate?

## 6. Scenarios that conflict with the fix

Direct conflicts (they require fail-closed blocking after a candidate rejection):
- `spec/features/durable-summary-submission-handoff.feature:27-31` "Production candidate
  rejection preserves memory and blocks inference". Keep "preserves memory"; replace
  "ordinary inference remains blocked" with "the exact error returns to the model for
  bounded correction".
- `spec/features/compaction-inference-handoff.feature:27-32` "Rejected summary
  submission blocks subsequent inference".
- `spec/features/compaction-inference-handoff.feature:41-46` "Rejected self-target
  generation blocks ordinary inference".
- `spec/features/compaction-malformed-generation-recovery.feature:29-33` "Exhausted
  malformed generation keeps inference blocked". The end state changes.
- `spec/features/compaction-peer-range-recovery.feature:29-33` "Exhausted peer conflicts
  never promote partial memory". The clause "with ordinary inference blocked" changes.

Conflicts depending on Q3 and Q4:
- `spec/features/runtime-compaction-no-substitute-success.feature:27-32` "Invalid builder
  output cannot become a replacement DAG" ("inference remains blocked").
- `spec/features/stream-recovery-compaction-builder.feature:29-34` "Builder failure stops
  hook-triggered recovery..." ("inference remains blocked").
- `spec/features/compaction-builder-budget-recovery.feature:45-49` "Repeated builder
  failures wait for fresh user input" and `:57-61` "Background notifications cannot
  authorize a failed builder retry".
- `:51-55` "Unrelated inference blocks cannot restart compaction" can stay, provided
  candidate rejections stop producing "unrelated" blocks.

Need extending, not reversing:
- `compaction-malformed-generation-recovery.feature:23-27` "Malformed generation receives
  actionable whole candidate correction". Its scope today is "the production builder";
  add the inject_summary and GenerateCompaction-self routes.
- `inject-summary-tool.feature:42` (description content) and `:50` (result shape:
  "queued" plus a new "rejected" result).

Not affected: the publication, identity and restore fail-closed scenarios (cold
publication, codex checkpoint, selected-provider identity and similar). Durable state is
genuinely ambiguous there, so blocking is correct.
