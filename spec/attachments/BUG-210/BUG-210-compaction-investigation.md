# BUG-210: automatic compaction visibility and continuation

Date: 2026-10-21\
Investigated revision: `c65b8fb25abb8bd6f8d822cf08fb1daa09c69afe`\
Scope: read-only investigation of the Rust background agent loop, shared CLI
stream engine, Codex provider, Rust TUI, existing specifications, and persisted
session evidence. No production code was changed and no tests were executed.

## User-visible problem

An agent working through tools appears to compact silently in the background.
A completion notice then appears, but unfinished work does not continue until
the user sends another message. The screenshot also reports misleading
reduction and coverage statistics.

Original screenshot, supplied by the user:
`/Users/rquast/Desktop/Screenshot 2026-10-21 at 8.09.12 am.png`.
This is a local reference, not a repository attachment.

The notice reads:

```text
[compaction] 54.2% reduction (166750 → 90336 tokens, 0 turns summarised)
```

## Persisted evidence matching the screenshot

Session: `4bd2eaa0-0db8-4e54-9c2a-8a8a6457e7b7`\
Provider/model: `codex/gpt-6.1-sol`

The session manifest is stored locally at
`~/.fspec/sessions/4bd2eaa0-0db8-4e54-9c2a-8a8a6457e7b7.json`.
Its committed compaction records generation 1, protocol `full-current-dag`,
196 covered persisted messages, no verified suffix, and an estimated input
basis of 90,336 tokens. Publication is no longer pending.

Timeline on 2026-10-21 in the user's local timezone (UTC+10):

| Time | Evidence |
| --- | --- |
| 08:06:31 | Persisted position 194 contains the BUG-209 commentary and the exact shell command visible in the screenshot. |
| 08:06:32 | Position 195 contains that command's successful tool result. |
| 08:08:50 | The manifest records the committed DAG, covering the prefix through position 195. |
| 08:09:12 | Screenshot shows the completion notice and idle composer. |
| 08:09:18 | Position 196 is a new user message containing `continue`. |

Positions 194–196 were checked using `SessionSearch originals`, not just
conversation previews. The matching tool result and 90,336-token committed
basis tie the persisted evidence to the screenshot. The 196 coverage count
is persisted-message coverage, not necessarily 196 conversational exchanges.

The original trigger is not recoverable from the available diagnostic logs:
the current combined log starts after this incident. We cannot conclusively
identify a backend overflow rejection versus a local request-admission failure,
or prove the exact recovery branch solely from the persisted conversation.

## Confirmed implementation findings

### 1. Shared terminal recovery compacts successfully but ends the turn

- `rust/agent-loop/src/agent_loop.rs:767` registers the compactor builder and
  terminal-overflow ownership before provider dispatch.
- `rust/cli/src/interactive/stream_loop.rs:1743` checks that ownership before
  ordinary in-loop recovery classification. A recognized context overflow
  returns to the outer loop instead of entering the normal retry path.
- `rust/agent-loop/src/terminal_overflow_recovery.rs:97` invokes
  `run_compactor_sub_agent_round` against the session's captured history.
- `rust/agent-loop/src/terminal_overflow_recovery.rs:42` restores Idle and clears
  recovery progress/busy ownership when recovery finishes.
- `rust/agent-loop/src/agent_loop.rs:1432` handles successful recovery by emitting
  Done, explicitly without replaying the oversized payload. It does not schedule
  a new inference turn on the reduced context.
- `rust/agent-loop/src/agent_loop.rs:119` checks synthetic retry input before
  waiting for external input; successful terminal recovery does not set it.

This is the strongest explanation of the observed stop. Avoiding replay of a
rejected oversized request is correct, but it must not imply abandoning the
unfinished task after a successful durable compaction.

### 2. Terminal compaction can be effectively silent

- `rust/cli/src/compactor_sub_agent.rs:100` sets the busy flag and emits progress,
  but does not emit CompactionStarted. Its terminal caller does not emit that
  event either.
- `rust/agent-loop/src/background_output.rs:538` is the event handler that changes
  status to Compacting and publishes the corresponding state change.
- `rust/agent-loop/src/background_output.rs:546` stores progress and returns
  without streaming it, expecting a polling consumer.
- The Rust TUI exposes `get_compaction_progress` in its transport implementations,
  but the production app has no caller polling it. Its progress-store setter has
  no production caller populating that slot.
- `rust/fspec-tui/src/store/agent_view/isolation_state.rs:107` describes
  chunk-driven progress as future work; the footer needs that stored progress
  at `rust/fspec-tui/src/views/agent/footer.rs:67`.
- `rust/agent-loop/src/compaction_builder.rs:67` creates an ephemeral builder ID;
  `rust/agent-loop/src/compaction_builder.rs:177` collects its response internally
  rather than forwarding its reasoning/tool stream to the parent session UI.

Consequently, the parent can show no useful compaction activity until completion.

### 3. Completion statistics identify a concrete emitter defect

- `rust/cli/src/compactor_sub_agent.rs:161` supplies `compacted / original`,
  which is the fraction retained, as the completion ratio.
- `rust/agent-loop/src/background_output.rs:555` multiplies that ratio by 100,
  sets Idle, and hardcodes both coverage counts to zero.
- `rust/fspec-tui/src/app/dispatch_compaction_complete.rs:85` renders that value
  directly as percentage reduction and displays the supplied count.

For this incident, `90336 / 166750 * 100 = 54.1745%`, exactly explaining the
displayed 54.2%. The actual reduction is **45.8%**. The displayed zero is a
placeholder, not evidence that no history was summarized; the durable boundary
records 196 covered messages. Prefer the authoritative committed coverage and
existing removed-token ratio convention rather than duplicating calculations.

### 4. A different recovery path can keep working while falsely showing Idle

`rust/cli/src/interactive/stream_loop.rs:669` runs the registered builder in the
in-loop restart macro. Completion sets Idle, but the macro restarts inference
at line 729 without emitting CompactionContinuing in that branch.

By contrast, pre-prompt recovery emits Continuing at line 335, and
`rust/agent-loop/src/background_output.rs:581` maps it to Running. Thus an idle
composer alone does not establish that inference has stopped. This is a
separate confirmed lifecycle gap, not proof of the screenshot's precise branch.

### 5. Other continuation boundaries require an explicit policy

- `rust/cli/src/interactive/stream_loop.rs:2387` runs a post-loop safety net when
  compaction remains needed. Its success path at line 2412 compacts in place but
  deliberately waits for the next user message rather than restarting.
- A queued self `GenerateCompaction` or `inject_summary` handoff breaks the stream
  at line 1046. The outer loop applies the candidate at
  `rust/agent-loop/src/agent_loop.rs:1347`, then takes the normal Idle success
  path without scheduling a continuation.

Audit these boundaries while fixing the shared lifecycle, but distinguish
automatic recovery of interrupted work from deliberate manual compaction of an
already idle session. Manual idle compaction must not unexpectedly start work.

## Provider scope

The lifecycle defects are shared, not confined to Codex. Terminal recovery
surrounds provider dispatch; built-ins and custom providers use the same stream
engine. Provider-specific builder availability still varies and must retain
explicit failure handling.

The screenshot matches fspec's textual DAG builder completion, not evidence of
native Codex server-side compaction. The current Codex memory policy rejects
native compaction item families and automatic context management in
`rust/providers/src/codex/memory_policy.rs:19`.

`rust/cli/src/interactive/overflow_classification.rs:5` recognizes shared typed
`RequestAdmissionError::BudgetExceeded` failures and structured provider overflow
codes, including Codex rejections. Codex physical request admission can fail on
a tool-result follow-up, but that specific trigger was not established here.

## Existing specifications and coverage gaps

- `spec/features/terminal-overflow-recovery-for-background-sessions.feature:30`
  explicitly requires Idle followed by a new user message. Update this contract
  when introducing automatic continuation; the current behavior is specified,
  not just an accidental missing UI action.
- `spec/features/terminal-recovery-single-owner-locking.feature` protects session
  ownership, unchanged context on failure, and cancellation.
- `spec/features/terminal-compaction-dependency-lifetime.feature` protects builder
  registrations through terminal resolution and their subsequent cleanup.
- `rust/agent-loop/tests/cmpct054_terminal_dependencies.rs` drives actual outer-loop
  terminal recovery with a physical mock HTTP rejection and failing builder.
- `rust/agent-loop/tests/cmpct044_terminal_overflow_test.rs:215` only checks a
  percentage falls within 0–100; this does not distinguish retained from removed.
- `rust/fspec-tui/tests/single_sourced_compaction_notice_rpc421.rs:166` supplies an
  already-correct percentage and tests presentation, not this backend calculation.
- `rust/napi/tests/compaction_status_lifecycle_test.rs:66` manually supplies events,
  so it cannot catch the missing production Continuing emission.

These are source-inspection findings. Test executability and pass/fail status
were not validated during the investigation.

## Proposed acceptance criteria and implementation direction

1. Automatic compaction recovering unfinished work visibly starts and reports
   progress in the originating session, regardless of focus or provider.
2. Successful durable commit and publication resume inference from the reduced
   context without requiring the user to type `continue`. Do not resend the
   rejected request or repeat already-completed tool side effects.
3. Running/Compacting/Idle transitions describe actual activity. Emit terminal
   Done only when the work really ends, is cancelled, or fails terminally.
4. Resume only after commit, publication, and reconciliation succeed. Preserve
   cancellation precedence, inference barriers, and operation-owned cleanup.
5. Bound automatic retries across repeated overflow and compaction attempts;
   surface an actionable terminal error when recovery cannot converge.
6. Report percent removed (45.8% for this fixture) and authoritative coverage;
   do not substitute zero for unavailable coverage or publish duplicate notices.
7. Keep manual compaction of idle sessions idle. Specify self-compaction handoff
   and post-loop recovery continuation separately where their intent differs.

Use ACDD: update the relevant feature contracts first, then write failing
behavioral tests before implementation. Prefer real outer-loop tests with
temporary stores and deterministic HTTP boundaries over source-text assertions.

Suggested regression coverage includes overflow → builder success → committed
generation → resumed ordinary inference with no intervening user input; visible
Started/progress/Continuing ordering; no premature Done; cancellation during
building and before resume; failed publication; repeated overflow budget limits;
completed tool preservation; correct ratio/coverage end to end; and equivalent
shared behavior for Codex and at least one non-Codex provider.

Scope validation to the affected package and test target, for example
`cargo test --profile ci-test -p codelet-agent-loop --test <target>`.
Do not run unscoped workspace tests. No major refactor was proposed or approved
as part of this card creation.
