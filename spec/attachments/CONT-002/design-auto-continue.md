# CONT-002 — Design: Auto-continue engine — done() tool + /continue toggle with nudge budget

**Type:** Story
**Epic:** completion-contract
**Depends on:** CONT-001 (structural loop-exit fix; without it, nudges get burned on a rig-core bug)
**Depended on by:** CONT-003 (/goal layers conditional acceptance on this engine)

---

## 1. Concept

A **Completion Contract**: when armed, the turn-sequence is not finished until the model
explicitly calls the `done()` tool. If the model stops (stop_reason `stop`/`end_turn`) without an
accepted `done()`, we inject a synthetic "nudge" user message and continue the loop — bounded by a
user-settable budget.

Effective mode is **derived, never stored**:

```
mode = Goal          if session.goal.is_some()        (CONT-003, out of scope here)
     = AutoContinue  else if session.continue_enabled
     = Off           otherwise
```

This card implements `Off` / `AutoContinue` only. Design constraint (user decision): completion is
NEVER tied to fspec work-unit status — this works for ad-hoc coding.

## 2. State (on `Session`)

File: `codelet/cli/src/session/mod.rs` — `pub struct Session` (lines 31–62). Add fields following
the precedent of `thinking_exhaustion_cross_turn_count: u32` (line 57):

```rust
/// /continue toggle (auto-continue mode). Persisted across turns.
pub continue_enabled: bool,          // default false
/// Max zero-progress nudges per user-turn. Set via `/continue <n>`. Default 10.
pub continue_budget: u32,            // default 10 (DEFAULT_CONTINUE_BUDGET)
/// Zero-progress nudges consumed this user-turn. Reset on each real user message.
pub continue_nudges_used: u32,       // default 0
```

Initialize in `Session::new` (line ~79) and `from_provider_manager` (line ~100).
NAPI/TUI mode gets this for free: `BackgroundSession.session.inner` is
`Mutex<codelet_cli::session::Session>` (`codelet/sessions/src/background_session.rs:270`,
used in `codelet/agent-loop/src/agent_loop.rs:94,299`).

### Budget semantics (user decision: `/continue 50` = up to 50 retries)
- Single number, user-facing. Counts **only zero-progress nudges**: if the model makes >= 1 tool
  call after a nudge, that nudge is *refunded* (not counted) — `/continue 50` means 50 genuine
  stall recoveries, not 50 loop iterations.
- Reset `continue_nudges_used = 0` on every real user message (start of a new user-turn).
- **No** consecutive-stall fast-path escalation in AutoContinue mode (that is Goal-mode behavior,
  CONT-003). The user set the budget; honor it literally.

### Exhaustion behavior (user decision: option b)
Finish the turn normally and emit a **visible warning line** via the output sink, e.g.
`⚠ auto-continue: model never called done() after N retries`. Session stays interactive; toggle
stays on for the next message. NO HITL pause in this mode.

## 3. `done()` tool

### Definition
```
done(summary: string)         // required, non-empty
```
Optional fields `evidence: string[]` and `goal_assessment: string` may exist in the schema
(forward-compatible with CONT-003) but are NOT required in this card. Acceptance is **Tier 0**:
accept at face value. The summary is surfaced to the user as the turn's closing line.

### Implementation pattern
Copy the session-scoped pattern of `InjectSummaryTool` —
`codelet/tools/src/inject_summary.rs:112–183`: struct holds `session_id: Uuid`; a per-session
handler/state registry (see `INJECT_SUMMARY_HANDLERS` RwLock, lines 90–98) or direct session-state
flag records "done() was called+accepted this turn-sequence" so the stream loop can read it at the
settle point. New file: `codelet/tools/src/done.rs` (or similar).
A minimal stateless reference is `ThinkTool` (`codelet/patches/rig-core/src/tools/think.rs:29–63`)
for `Tool` trait shape (`codelet/patches/rig-core/src/tool/mod.rs:106`).

### Registration — armed only
`done()` is registered in the agent ToolSet **only while armed** (continue_enabled). Rationale:
schemas cost tokens and invite spurious calls in Off mode. Registration sites (the seven
`create_rig_agent` builder chains — add conditionally based on session state):
- `codelet/providers/src/claude.rs:506` (chain 533–555)
- `codelet/providers/src/openai.rs:424` (449–468)
- `codelet/providers/src/gemini.rs:130` (194–214)
- `codelet/providers/src/zai.rs:218` (265–284)
- `codelet/providers/src/codex/mod.rs:331` (400–426)
- `codelet/providers/src/copilot/rig_agent.rs:56` (86–105)
- `codelet/providers/src/custom/custom_provider.rs:110` (261–294)
NOT in the DeepSearch sub-agent toolset (`codelet/agent-loop/src/deep_search_handler.rs:308–337`)
— sub-agents have their own lifecycle.
If conditional registration is architecturally impractical at agent-construction time (verify when
agents are (re)built relative to the toggle), fallback is: always register, and the tool returns an
inert acceptance in Off mode. Prefer conditional registration; document the choice in an
architecture note. If a stale `done()` arrives right after toggling off, accept inertly (never
error).

## 4. `/continue` command surface

| Input | Effect |
|---|---|
| `/continue` | Toggle on/off, default budget (10); print new state |
| `/continue <n>` (n >= 1) | Turn ON with budget = n; if already on, update budget only |
| `/continue on` / `/continue off` | Explicit set, default budget |
| `/continue 0` | Rejected with hint: "use /continue off" |
| invalid arg | Error message, state unchanged |

(`/continue off` refusal while a goal is active is CONT-003 — here there is no goal.)

### TUI wiring (ratatui)
1. Registry: `codelet/fspec-tui/src/views/agent/slash_commands.rs` — add `SlashCommandAction::Continue`
   (enum line 21), `name()` (line 45), entry in `SLASH_COMMANDS` (line 85).
2. Typed args: `codelet/fspec-tui/src/app/slash_parser.rs` — `parse_slash_command` (line 77): add a
   `"/continue"` branch producing e.g. `SlashCommandParse::ContinueToggle(Option<ContinueArg>)`.
   Model the arg-parsing on `loop_parser.rs` (`LoopSubcommand`, line 19; routed at
   `slash_parser.rs:123`).
3. Dispatch: `codelet/fspec-tui/src/app/dispatch_slash_commands.rs` — `handle_slash_command`
   (line 27) for bare palette pick; `handle_input_submitted` (line 175) for typed form. Backend
   round-trip pattern: the `/compact` arm (lines 65–97). The toggle mutates session state via the
   NAPI/backend session binding (follow `/compact` → `backend.compact_session` →
   `codelet/napi/src/session_bindings.rs` pattern for a new session-state setter).
4. Status bar indicator: `⏩ auto-continue (n/N)` while armed (follow existing status-bar patterns).

### CLI repl wiring
`codelet/cli/src/interactive/repl_loop.rs::repl_loop` (line 16): insert the `/continue [arg]`
handler BEFORE the `input.starts_with('/')` provider-switch catch-all (line ~137), following the
`/compact` pattern (line 68): parse, mutate `session`, print state via output, `continue;`.

## 5. Auto-continue decision point (stream loop)

File: `codelet/cli/src/interactive/stream_loop.rs`, `run_agent_stream_internal` (line 269).
The settle point is the `MultiTurnStreamItem::FinalResponse` arm (lines 1039–1431), specifically
just before `output.emit_done_with_stop_reason(final_stop_reason.take()); break;` (lines 1428–1430).

Decision (extract into a testable pure function, e.g. `decide_continuation(...) -> ContinueDecision`
in its own module — keep stream_loop.rs from growing; new files < 300 lines discipline):

| Condition | Decision |
|---|---|
| mode Off | Finish (today's behavior, zero change) |
| `done()` accepted this turn-sequence | Finish; surface summary; reset nudges_used |
| stop_reason in {stop, end_turn} (or None) without done(), nudges_used < budget | Nudge |
| same, budget exhausted | Finish + warning line (`⚠ auto-continue: ...`) |
| interrupted (`is_interrupted`) | Finish — user interrupt ALWAYS wins, never nudge |
| max_tokens/truncation | existing PROV-040/PROV-041 handling first (unchanged, takes precedence) |

### Nudge mechanics — copy the PROV-041 re-prompt recipe (stream_loop.rs lines 1301–1358)
1. Build nudge text (below), as a plain user message (PROV-040/041 style at lines 1332/1701 —
   NOT a persistent typed system-reminder; one-shot).
2. Fresh `TokenState` + `CompactionHook::new(...)`.
3. `stream = agent.prompt_streaming_with_history_and_hook(&msg, &mut session.messages, hook).await`
4. Push `Message::User { content: OneOrMany::one(UserContent::text(&msg)) }` to history.
5. Reset per-turn locals: `assistant_text`, `final_stop_reason = None`, `tool_calls_buffer`,
   `turn_tool_infos`, `tool_execution_in_progress`, `streaming_display`.
6. `continue;` back into the main `loop` (line 745).
7. Increment `continue_nudges_used`; track whether the following segment produces tool activity
   (observe `turn_tool_infos`/tool events) — if it does, refund (decrement) that nudge.

Nudge text (AutoContinue):
```
You stopped without calling done(). If the task is complete, call done(summary);
otherwise continue working.
```

Other `emit_done_with_stop_reason` sites — line 763 (interruption), 813/843/864 (stall timeout),
1903 (error path) — must NOT nudge. Only the clean FinalResponse settle point does.

### done() acceptance signal
The stream loop must know "done() was called and accepted during this turn-sequence". Mechanism:
the tool handler records acceptance in per-session state (registry keyed by session_id, like
inject_summary) or a flag readable at the settle point; cleared at the start of each user-turn.
The model's *text claims* ("I'm done") never count — only the tool call.

## 6. Explicitly Out of Scope
- /goal, verify commands, conditional acceptance tiers, HITL escalation, system-reminder
  persistence of contract state → CONT-003.
- Any coupling to fspec work-unit status. Forbidden by design.
- Empty-response/thinking-only facade unification (Gemini facade continues to work as-is; its
  reprompts are independent of this budget in this card).

## 7. Acceptance Rules (seed for Example Mapping)
1. Off mode: behavior is byte-for-byte today's behavior; done() not registered (or inert).
2. `/continue` toggles; `/continue 50` arms with budget 50; `/continue 0` rejected; state printed.
3. Armed + model stops without done() → nudge injected, loop continues, nudge counted.
4. Armed + model calls done(summary) → turn finishes, summary surfaced, no nudge.
5. Nudge followed by tool activity → refunded (doesn't consume budget).
6. Budget exhausted → finish with visible warning; session interactive; toggle still on.
7. New user message resets nudges_used.
8. User interrupt during armed session finishes immediately (no nudge).
9. Works in both CLI repl and TUI (slash command + status indicator).
10. Truncation (max_tokens) recovery still takes precedence and is unchanged.

## 8. Testing Guidance
- Rust: unit-test the pure decision function exhaustively (all table rows). Integration-test the
  stream loop with a scripted/mock stream where feasible (mirror existing PROV-040/041 test
  harnesses — find via `rg "truncation_retry" codelet/ -l`; also reuse the `ScriptedModel` harness
  from CONT-001's `codelet/patches/rig-core/tests/multi_turn_tool_continuation.rs`).
- TUI: slash parser unit tests (mirror `loop_parser` tests), dispatch tests per existing patterns
  in `fspec-tui/src/app/` tests.
- Every Gherkin scenario → one test with `// @step` comments matching step text exactly.

## 9. Definition of Done
- Feature file(s) tagged `@CONT-002`, capability-named (e.g. `auto-continue-engine.feature`).
- Tests first (red) → implementation (green); `cargo build`, `cargo clippy`, full `cargo test`
  workspace-green; TUI crate tests green.
- Coverage fully linked; `fspec validate`, `validate-tags` clean.
- No `unwrap()` in production paths, no `todo!()`/`unimplemented!()`, files < 300 lines where the
  project convention applies.
