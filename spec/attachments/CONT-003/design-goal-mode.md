# CONT-003 — Design: /goal mode — conditional done() acceptance against a user-set goal

**Type:** Story
**Epic:** completion-contract
**Depends on:** CONT-002 (auto-continue engine: done() tool, nudge budget, decision point)

---

## 1. Concept

**`/goal` = `/continue` + an acceptance condition on `done()`.**

While a goal is active, effective mode is `Goal` (implies auto-continue). `done()` is no longer
accepted at face value: it must carry evidence and a goal assessment (Tier 1), and when a verify
command is configured, that command must exit 0 (Tier 2). A **rejected `done()` is just a failed
tool result** — the multi-turn loop continues naturally with zero special machinery. Only
"stopped without calling done()" uses the CONT-002 nudge path (shared budget engine).

Derived mode (from CONT-002, extended):
```
mode = Goal          if session.goal.is_some()
     = AutoContinue  else if session.continue_enabled
     = Off           otherwise
```
`continue_enabled` and the goal are independent inputs; mode is derived, never stored. Therefore
`/goal clear` after `/continue on` falls back to AutoContinue (NOT Off).

Design constraint (user decision): NEVER coupled to fspec work-unit status.

## 2. State (on `Session`, extends CONT-002 fields)

```rust
pub struct SessionGoal {
    pub text: String,
    pub verify: Option<String>,   // shell command; exit 0 = verified
    pub set_at: /* timestamp */,
}
pub goal: Option<SessionGoal>,        // Some ⟺ effective mode Goal
pub done_rejections: u32,
```
`done_rejections` resets when the goal is set/replaced/cleared. It does NOT reset on nudges.

### Budget interplay
- Goal-mode default budget: **15** zero-progress nudges.
- If the user explicitly set `/continue <n>`, the **larger explicit value wins** over the Goal
  default.
- Progress-refund semantics identical to CONT-002.
- Goal mode KEEPS the stall fast-path: two consecutive zero-activity nudges (no tool calls, no
  done()) → escalate immediately (see §5) without burning remaining budget.

## 3. `/goal` command surface

| Input | Effect |
|---|---|
| `/goal <text>` | Set/replace goal → mode Goal; reset done_rejections and nudges_used; print state |
| `/goal` | Show contract state: goal text, verify cmd, effective budget, nudges used, rejections |
| `/goal verify <cmd>` | Attach/replace verify command (requires an active goal; else error) |
| `/goal clear` | Drop goal → fall back to continue_enabled (AutoContinue or Off); print state |
| `/continue off` while goal active | **Refused** with message "clear the goal first (/goal clear)" |

Wiring: identical surfaces to CONT-002 —
- TUI: `slash_commands.rs` registry (+`SlashCommandAction::Goal`), `slash_parser.rs` subcommand
  parsing (model on `loop_parser.rs` `LoopSubcommand`), `dispatch_slash_commands.rs` dispatch,
  session-state mutation via backend binding.
- CLI: `repl_loop.rs` handler before the provider-switch catch-all.
- Status bar: `🎯 goal (nudges u/N)` replaces the `⏩ auto-continue` indicator while goal active.

## 4. done() acceptance pipeline (Goal mode)

Schema (extends CONT-002's tool):
```
done(summary: string, evidence?: string[], goal_assessment?: string)
```

| Tier | Check | On failure (tool returns error result, loop continues) |
|---|---|---|
| 1 — schema | `goal_assessment` present and non-trivial (non-empty, not boilerplate-length < ~20 chars); `evidence` non-empty | `done() rejected: you must provide evidence and a goal_assessment for the active goal: <goal text>` |
| 2 — verify (only if `verify` configured) | Run the command (cwd = project root, bounded timeout, capture output) | `done() rejected: verification command failed (exit <code>):\n<stderr/stdout tail (bounded, e.g. last 4KB)>` |

- Tier 2 runs ONLY after Tier 1 passes.
- Verify command execution: use existing tool/bash execution infrastructure conventions (timeout,
  output capture) — do NOT shell out unboundedly. Timeout default e.g. 300s; on timeout →
  rejection with a timeout message.
- Accepted done(): mark goal satisfied, announce to user (`🎯 goal satisfied: <summary>`), and
  **auto-clear the goal** (falls back to the toggle). Reset done_rejections.
- On each rejection: `done_rejections += 1`.

### Dynamic tool description
While a goal is active, `done()`'s tool description (the `definition()` output) appends:
`The current goal is: <text>. You must not call done() unless this goal is met; provide evidence
and goal_assessment.` The definition must therefore be built per-prompt from session state
(verify when `Tool::definition()` is invoked relative to agent (re)construction; if agents are
rebuilt per user-turn this is straightforward, otherwise read goal via the session-scoped
registry at definition time).

## 5. Escalation (differs from CONT-002's option-b)

Goal mode justifies stopping the world:
- `done_rejections >= 4` → escalate: HITL pause (reuse the existing HITL pause mechanism in
  `codelet/agent-loop/src/agent_loop.rs` — same path used by the compaction watchdog/HITL work)
  or, in plain CLI repl mode where no HITL pause construct exists, finish the turn with a
  prominent blocked message: `🎯 goal: model repeatedly claims completion but verification fails
  — human review needed`. The goal remains active.
- Budget exhausted (zero-progress nudges) → same escalation (NOT the silent-warning finish of
  AutoContinue).
- Two consecutive zero-activity nudges → immediate escalation (stall fast-path).

## 6. Compaction-proof persistence

The goal must survive compaction. Use the existing typed system-reminder infrastructure:
- `codelet/cli/src/session/system_reminders.rs` — add `SystemReminderType::CompletionContract`
  (enum at line 23; tags at 67–68). Reminders are deduped via `add_system_reminder()` and
  preserved through compaction by the partition helpers (~line 192).
- Inject/refresh on goal set (`Session::add_system_reminder`, session/mod.rs:169–172) and ensure
  per-turn presence following the pattern at `codelet/agent-loop/src/agent_loop.rs:301`
  (user_prompt_submit additional_context) / `codelet/napi/src/agent_loop.rs` (~199).
- Content: `<system-reminder>` with `<!-- type:completion-contract -->`, goal text, verify command
  (if any), and the instruction that done() requires evidence.
- On `/goal clear` or acceptance: remove/replace the reminder (retain-based removal in
  `add_system_reminder` dedup logic supports replacement; verify a removal API exists or add one).

## 7. Explicitly Out of Scope
- Tier 3 LLM-judge acceptance (future work; not this card).
- `/goal from <WORK-UNIT>` fspec sugar (future; mechanism must stay fspec-agnostic).
- Changing CONT-002 AutoContinue exhaustion behavior (stays option-b warning finish).

## 8. Acceptance Rules (seed for Example Mapping)
1. `/goal <text>` arms Goal mode even when /continue is off; state shown; reminder injected.
2. `/goal clear` with continue_enabled=true → AutoContinue; with false → Off.
3. `/continue off` while goal active is refused with actionable message; state unchanged.
4. done() without evidence/goal_assessment in Goal mode → rejected (Tier 1), loop continues.
5. With verify configured: done() with evidence but failing verify → rejected with command output
   tail (Tier 2), loop continues.
6. Verify passes → done() accepted, goal announced satisfied and auto-cleared, falls back to
   toggle mode.
7. 4th rejection → escalation (HITL pause / prominent blocked message), goal stays active.
8. Budget exhaustion in Goal mode → escalation, NOT silent warning.
9. Two consecutive zero-activity nudges → immediate escalation.
10. Goal text survives compaction (system-reminder re-injection) and appears in done()'s tool
    description while active.
11. Larger explicit /continue budget overrides Goal default 15.
12. `/goal verify <cmd>` without active goal → error.

## 9. Testing Guidance
- Pure-function tests for the acceptance pipeline (Tier 1 validation, Tier 2 result mapping,
  rejection counting, escalation thresholds) — exhaustive.
- Verify-command execution tests with real short-lived commands (`true`/`false`/`sh -c 'exit 3'`),
  timeout behavior with a bounded sleep.
- System-reminder persistence tests following existing `system_reminders.rs` test patterns.
- Slash parser tests for all /goal forms + the /continue-off refusal.
- Every Gherkin scenario → one test with `// @step` comments.

## 10. Definition of Done
- Feature file(s) tagged `@CONT-003`, capability-named (e.g. `goal-enforcement.feature`).
- Tests first (red) → implementation (green); workspace `cargo build`/`clippy`/`test` green;
  TUI tests green.
- Coverage fully linked; `fspec validate`, `validate-tags` clean.
- No unwrap()/todo!() in production paths; new modules < 300 lines discipline.
