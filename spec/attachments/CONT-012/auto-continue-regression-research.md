# Auto-continue silent stops: research (2026-10-30)

## TL;DR

Auto-continue is not broken everywhere. The nudge still fires on clean `end_turn`/`None` stops every day (logs Oct 26–30). What users see as "auto-continue stopped working" is one specific, now very frequent shape:

> An armed turn does tool work. The final provider request then ends **thinking-only** (no visible text, no tool call) with `stop_reason = max_tokens`. Neither the PROV-041/BUG-221 thinking recovery nor the CONT-002 nudge runs. The turn ends silently and the session goes Idle. The user (or a supervisor session) has to type `continue`.

On **2026-10-29**, 50 armed turns across 15 sessions ended "stream finished → Idle" with no nudge. **46 of the 50** were thinking-only `max_tokens` finals, and **all 46 had tool calls earlier in the same user turn**. The next user inputs were things like `continue`, `...`, `....`, `ACTION THIS!! STOP OVERTHINKING THINGS`, or `[SUPERVISOR: …]` re-prompts from a supervisor session.

## Original feature (CONT-002) and its early behaviour

- Card: **CONT-002** "Auto-continue engine: done() tool and /continue toggle with nudge budget". Commit `7c95aa84` (2026-07-11): `cli/src/interactive/auto_continue.rs`, `done_early_exit.rs`, `goal.rs`, nudge site in the monolithic `cli/src/interactive/stream_loop.rs`.
- Rule 3: armed + stop_reason in `{stop, end_turn, None}` without accepted `done()` → inject nudge.
- Rule 7: *"max_tokens/truncation recovery (PROV-040/041) takes precedence and is unchanged; only the clean FinalResponse settle point may nudge"*.
- So from day one, a `max_tokens` final was handed to PROV-041. PROV-041 only fired when `reasoning_tokens > 0` **and** `output_tokens < 50` (`recovery_thinking.rs:39`). Local OpenAI-compatible servers (Strata/spark qwen3.8) report no reasoning tokens, and `output_tokens` is aggregated across the whole multi-turn stream. So PROV-041 never matched these turns either. The gap is old. What changed is how often turns hit it (see "Why now").
- CONT-009 (2026-07-10) wired arming into the NAPI twin. Arming now lives in `sessions/src/background_session/completion_contract.rs:94-131` and is called from `agent-loop/src/agent_loop.rs:145-149`. **Verified intact.**

## Current code path (post Oct 25 split of `stream_loop.rs`)

`stream_loop/driver.rs` → `FinalResponse` → `stream_loop/final_response.rs::handle`
1. `usage::final_response` captures `final_stop_reason` (`usage.rs:36`).
2. `final_thinking::recover` (`final_response.rs:75`) runs PROV-041 if either:
   - `is_thinking_exhaustion(stop, reasoning_tokens, output_tokens, 50)`: fails (no reasoning usage; aggregated output in the tens of thousands), or
   - `is_thinking_only_exhaustion(stop, accumulated_reasoning, assistant_text, has_tool_progress)` (BUG-221, added in `2699693b` on 2026-10-25), where
     `has_tool_progress = state.auto_continue_segment_tool_calls > 0` (**`stream_loop/final_thinking.rs:41`**).
     `auto_continue_segment_tool_calls` is incremented for **every** tool call in the turn (`assistant.rs:63`). It is only reset when a nudge is injected (`final_completion.rs:96`). So after any tool use in the user turn, the thinking-only recovery is disabled for the rest of that turn.
3. `final_completion::complete` → `decide_continuation(...)`. `stop_reason = "max_tokens"` is not a clean stop (**`auto_continue.rs:76`**), so the result is `ContinueDecision::Finish` → `emit_done_with_stop_reason` → Idle. No log line is emitted on this path.

### The spec and test lock the failure in

`spec/features/thinking-only-exhaustion-recovery.feature`, scenario **"Tool progress prevents thinking-only recovery"**:

```
Given a length-limited response containing thinking and a completed tool call
When the model stops without an answer after that tool progress
Then no thinking exhaustion retry or auto-continue nudge is sent
```

`rust/cli/tests/bug221_thinking_only_exhaustion.rs:163` (`tool_progress_prevents_thinking_only_recovery`) builds `[tool_response(), response(REASONING, "", Some("length"))]` with auto-continue armed. It asserts `retries == 0` and `continue_nudges_used == 0`. That is exactly the production failure. BUG-221 rule 1 says "no visible answer **or tool progress**". The intent most plausibly meant progress *in the final response*, but the implementation uses turn-wide progress.

A second, smaller path: when PROV-041 does fire and exhausts its 2 retries ("retry budget exhausted", 6× on Oct 29), it returns `Control::Next`. Then `decide_continuation` again sees `max_tokens` → `Finish`, so an armed turn also ends silently there (BUG-221 rule 2: "ordinary length stops never consume auto-continue nudges").

## Evidence

### Logs (`~/.fspec/logs/private/fspec-combined.log.*`)

Nudges are firing (count of `CONT-002: auto-continue nudge`):

| Day | nudges | stop reasons |
|---|---|---|
| 10-26 | 17 | end_turn 16, None 1 |
| 10-27 | 32 | end_turn 19, None 13 |
| 10-28 | 7 | end_turn 5, None 2 |
| 10-29 | 33 | end_turn 33 |
| 10-30 | 10 | end_turn 10 |

On 10-29, 127 `Agent multi-turn stream finished` events occurred. **50** were followed directly by `set_status … to=Idle` with no nudge and no PROV-041 line.

### Persisted envelopes for those 50 (stop_reason from `metadata.message.stop_reason`)

| final message shape | count |
|---|---|
| assistant, `max_tokens`, thinking-only | **46** |
| assistant, `max_tokens`, text + thinking (correctly excluded) | 1 |
| no assistant message persisted after the user input (session 9c8e360f) | 3 |

Every one of the 46 thinking-only finals had ≥1 `tool_use` earlier in the same user turn. Example: the main session `58cff6ed` (Strata qwen3.8-flash, autoContinue 300):
- msg 167 (00:53:54Z): assistant, 4,091 thinking parts, no text, `max_tokens` → Idle → user typed `continue` (msg 168).
- msg 241 (01:16:01Z): assistant, 4,094 thinking parts, `max_tokens` → Idle → user typed `continue` twice (msgs 242, 243).
- Between them, 01:03:59Z on the same session: `end_turn` → nudge fired normally. So arming works.

Sessions affected on 10-29 (provider): 16207d9d, 036d9123, a5e71663, 6b4ec2df, 5a0ef565, b9cf9da2, e94e5edb, 225f2fec, 8221347c, 0c7b9bd6, 58cff6ed, 9c8e360f, cfdb79d5, 072d583b, 37404f8a. All finals were `provider=openai` (spark/Strata qwen3.8 profiles).

### Why now: thinking-only `max_tokens` finals by day (all sessions)

From `~/.fspec/messages/messages.jsonl`, assistant envelopes with `stop_reason=max_tokens`: 1–3/day through mid-October, then 2 (10-20), 7 (10-21), 9 (10-23), 11 (10-26), **56 (10-29)**. From 10-20 on, almost all are thinking-only on `provider=openai`. That matches the move to local qwen3.8 reasoning models on OpenAI-compatible profiles.

Contributing factor: the OpenAI provider puts `max_tokens = self.max_output_tokens` on the wire (`providers/src/openai.rs:415,545`). It comes from `OPENAI_MAX_OUTPUT_TOKENS`, else `DEFAULT_MAX_OUTPUT_TOKENS = 4096` (`openai.rs:31`). `sessions/src/profile_environment.rs` bridges base URL, key, context window and streaming, but **not** max output tokens. The manager's resolved limits (`providers/src/manager.rs:1074-1090`, `override_model_limits`) do not reach the provider request. The ~4,090 streamed thinking parts per failed message line up with that 4096 cap. A reasoning model that thinks more than 4k tokens before acting will therefore end thinking-only at `max_tokens` regularly. (Not proven to be a regression in the last week. It is the condition that makes the gap above frequent.)

## What was ruled out

- **Arming / Done registration / provider builders.** `completion_contract.rs` syncs `set_continue_armed`. Every provider builder checks `is_continue_armed` (claude, openai, gemini, zai, copilot, codex, custom). Nudges fire. This is what CONT-011 hardened; it was not the observed failure.
- **Nudge decision logic.** `final_completion.rs` matches the pre-split `stream_loop.rs:1508-1690` (at `a97b6437`) apart from reusing the shared token state for the nudge hook.
- **BUG-284 admission (staged WIP: `agent_loop/admission.rs`, `cli/src/session/admission.rs`, `driver.rs`).** `detach_admitted_input` only affects the first request. The nudge request uses `prompt_streaming_with_history_and_hook(AUTO_CONTINUE_NUDGE_PROMPT, …)` with no admitted input. Synthetic nudges are not subject to receipts. In a 10-30 probe session (3bc67079) the nudge led to an accepted `done()` correctly.
- **Persistence of nudge prompts.** Nudge text is not persisted as a user-role message today. Earlier hits in the store are Codex provider-native assistant records and tool results quoting the source, not a regression. (Cold-resume fidelity gap only: the persisted transcript merges the pre-nudge answer and the post-nudge `done()` into one assistant record. Out of scope here.)

## Proposed fix direction (for discovery)

1. Scope BUG-221's "tool progress" to the **final response segment**: tool calls since the last provider request started (or since the last tool-result round-trip), not the whole user turn. Reset a per-request counter on `ResponseAttemptStarted` / each rig sub-turn. Keep the turn-wide counter for nudge refund accounting.
2. When armed and a thinking-only `max_tokens` final is not recovered (gate not met, or PROV-041 budget exhausted), do **not** finish silently. Either nudge (charging the zero-progress budget), or emit a visible status line saying why the turn stopped. Decide which with the user (question 1).
3. Replace or extend `bug221_thinking_only_exhaustion.rs:163` with: tool turn → thinking-only `length` final → exactly one recovery retry; plus armed + budget exhausted → visible outcome (nudge or warning).
4. Separately (own card?): bridge a profile max-output setting into `OPENAI_MAX_OUTPUT_TOKENS`, or pass resolved manager limits into the OpenAI request, so reasoning models aren't capped at 4096.

## Open questions

1. Armed + thinking-only `max_tokens` after PROV-041 is exhausted: nudge (consumes budget), or finish with a visible warning?
2. Should "tool progress" mean tool calls in the same provider response only, or since the last tool-result round-trip?
3. Is the 4096 cap for OpenAI-compatible profiles in scope here, or a separate PROV card?
4. Session 9c8e360f had 3 turns that finished with no assistant message persisted at all. Investigate here or separately?
