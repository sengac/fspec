@done
@cli
@codelet
@completion
@CONT-005
Feature: done() immediate termination
  """
  VERIFIED (2026-07-10 deep-search): why the loop cannot end at done() today — rig-core re-prompt condition at codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:810-831 exits only when a turn produced NO tool calls (CONT-001); done() is a tool call so rig always re-prompts after it. Acceptance write at done.rs:359-361 (returns 'Completion recorded. The turn will finish with your summary.'); single consumption at stream_loop.rs:1458 in the FinalResponse arm (:1054-1615); FinishWithSummary at :1510-1526; break at :1613-1614.
  CHOSEN MECHANISM (Option D, verified): the outer loop DOES see the done() ToolResult before rig re-prompts (StreamUserItem(ToolResult) arm, stream_loop.rs:965-1011; result string at :982-994). Caveat: rig's ToolResult carries only id/call_id/content — NO tool name (rig streaming.rs:685-686) — so identify done() via the DONE_ACCEPTANCE registry (or last_tool_name, stream_handlers.rs:143), not by name matching. Do NOT reuse rig CancelSignal (rig streaming.rs:461): it exits via Err(PromptCancelled) which stream_loop.rs:1634 classify_compaction_branch routes into the compaction recovery cascade.
  HISTORY INVARIANT (verified safe): handle_tool_call only buffers (stream_handlers.rs:106-154); handle_tool_result flushes the assistant tool_use message AND pushes the tool_result user message into session.messages (stream_handlers.rs:157-242, :168-177) BEFORE any early-break point in the ToolResult arm — so breaking after done()'s ToolResult preserves the tool_use/tool_result pairing. rig's own chat_history copy is discarded harmlessly (only read on PromptCancelled recovery via reconcile_session_messages, stream_loop.rs:1657-1667). Early-exit must replicate the FinishWithSummary teardown: emit '✓ done: <summary>' (:1523), reset continue_nudges_used (:1525), flush pending assistant_text (interrupt-path pattern :770), emit_done_with_stop_reason (:1613), break (:1614); goal-mode teardown lives in CONT-006 via a SHARED helper. Open questions: drain remaining batch tool results before break (recommended) vs break instantly; stop_reason value ('stop' vs new 'done').
  Implementation shape (as built; goal gate SUPERSEDED BY CONT-006): module codelet/cli/src/interactive/done_early_exit.rs (pub, precedent: auto_continue/goal modules) with (1) decide_tool_result_early_exit(take_acceptance: FnOnce() -> Option<String>) -> Option<String> — pure decision, filters empty summaries; CONT-005 originally shipped a goal_active first parameter that never invoked the take closure in goal mode, and CONT-006 dropped that gate/parameter so the decision no longer consults goal state; (2) apply_finish_with_summary(session, session_id, summary, output) — the ONE shared teardown used by both the ToolResult-arm early exit and the FinalResponse FinishWithSummary arm (goal branch: apply_goal_acceptance + set_session_goal(None); non-goal: '✓ done:' status; both: continue_nudges_used = 0 + CONT-007 ContinueState snapshot with the CONT-008 GoalSatisfied/DoneAccepted reason); (3) DONE_EARLY_EXIT_STOP_REASON = "done". stream_loop.rs ToolResult arm runs the consult+teardown+break after tool_execution_in_progress = false; the FinalResponse FinishWithSummary arm calls the same helper. One fix covers CLI, NAPI, and agent-loop surfaces (all delegate to run_agent_stream_internal). Post-loop CMPCT-032 safety net already covers any new break path.
  """

  Background: User Story
    As a TUI/CLI user running an agent with auto-continue armed
    I want to have the agent loop terminate immediately when the model's done(summary) call is accepted
    So that the recorded summary is the turn's true closing state and no further model segments can run after completion

  Scenario: Accepted done() terminates the turn at the ToolResult arm
    Given auto-continue is armed for a session with no active goal
    And the model's done call with summary "Task complete" was accepted into the registry
    When the early-exit decision is consulted after the done tool result is processed
    Then the decision consumes the acceptance and returns the summary "Task complete"
    And the shared teardown surfaces "✓ done: Task complete" and resets the nudge counter
    And the early exit terminates the turn with the literal stop_reason "done"

  Scenario: Goal-mode acceptance exits at the ToolResult arm (deferral superseded by CONT-006)
    Given a goal is active for the session
    And the model's done call was accepted into the registry
    When the early-exit decision is consulted at the ToolResult arm
    Then the decision consumes the acceptance and returns the summary
    And the goal-mode teardown is delegated to the shared helper owned by CONT-006

  Scenario: Rejected done() records no acceptance and never exits early
    Given a goal is active for an armed session
    When the model calls done without evidence or a goal assessment
    Then the done call is rejected as a tool error
    And the early-exit decision finds no acceptance and the loop continues

  Scenario: Stale done() while auto-continue is off never exits early
    Given auto-continue has been toggled off for the session
    When the model calls done with summary "late but harmless"
    Then the call is acknowledged inertly without recording an acceptance
    And the early-exit decision finds no acceptance and the loop continues

  Scenario: FinalResponse fallback runs the identical shared teardown
    Given an acceptance that survives to the FinalResponse settle point
    When the settle point decision is FinishWithSummary
    Then the FinishWithSummary arm routes through the same shared teardown helper as the early exit
    And in goal mode the teardown announces the satisfied goal, clears the session goal, and clears the registry goal
    And in non-goal mode the teardown surfaces "✓ done: <summary>" and resets the nudge counter

  Scenario: Pending assistant text is preserved in history before the early break
    Given the model streamed explanation text before calling done
    When the early exit fires at the ToolResult arm
    Then the early-exit block flushes pending assistant text into message history before breaking
    And the early-exit block processes turn annotations like the other clean-exit paths

  Scenario: Stream-loop wiring pins early-exit ordering and single-teardown invariants
    Given the source file codelet/cli/src/interactive/stream_loop.rs
    Then the ToolResult arm consults the early-exit decision only after handle_tool_result has paired the tool messages
    And the loop-top interrupt check precedes the early-exit consultation
    And the early-exit consultation precedes the FinalResponse settle point decision
    And the early exit emits done with the shared stop_reason constant and breaks
    And the done summary status formatting lives only in the shared teardown helper
    And the stream loop never reuses CancelSignal for done() termination
