@done
@cli
@codelet
@completion
@session
@CONT-006
Feature: /goal immediate termination and atomic goal teardown on accepted done()
  """
  VERIFIED teardown inventory an early-exit path must replicate atomically (all currently at the FinalResponse settle point): announce via apply_goal_acceptance (goal.rs:81-84, called at stream_loop.rs:1517); Session::clear_goal (session/mod.rs:235-242) clears goal + done_rejections + removes the CompletionContract reminder (system_reminders.rs:366); registry clear+rejection reset via set_session_goal(None) (stream_loop.rs:1519 → done.rs:110-116); nudge reset (:1525). DESIGN: factor :1511-1525 into a shared helper used by both the ToolResult-arm early exit (CONT-005 mechanism) and the FinalResponse fallback so paths can never diverge.
  GAP HAZARDS immediate termination eliminates (verified): (1) Tier-2 verify passed at tool time can be invalidated by post-acceptance work; (2) second done() before settle re-runs the verify command (side effects, 300s timeout) and DONE_ACCEPTANCE.insert (done.rs:360) silently overwrites the first summary (last-writer-wins); (3) pending acceptance masks a due escalation — decide_goal_continuation checks done_summary first (auto_continue.rs:113-117) before rejections>=4 (:124-126) and the stall fast-path (:129-134); (4) the CompletionContract reminder stays in context during the gap (session/mod.rs:252-257), encouraging redundant work. Escalation is also settle-only (stream_loop.rs:1533-1545 → goal.rs:66-76 → tool_pause.rs:81-86).
  Implementation shape: (1) done_early_exit.rs — decide_tool_result_early_exit loses the goal_active parameter (signature: (take_acceptance: impl FnOnce() -> Option<String>) -> Option<String>); apply_finish_with_summary is UNCHANGED — its goal branch (apply_goal_acceptance → clear_goal + '🎯 goal satisfied'; set_session_goal(None); nudge reset) already implements the full CONT-006 atomic teardown; (2) stream_loop.rs ToolResult-arm call site drops the session.goal.is_some() argument and updates the CONT-005 comments; the FinalResponse FinishWithSummary fallback is untouched. CONT-005 test/feature updated for the lifted gate.
  """

  Background: User Story
    As a TUI/CLI user driving an agent under a /goal completion contract
    I want to have the agent loop terminate immediately with a full atomic goal teardown when the model's done() call is accepted
    So that the verified completion is the turn's true closing state — no post-acceptance work can invalidate the passed verify, re-run it, or mask a due escalation

  Scenario: Accepted goal-mode done() terminates the turn at the ToolResult arm with atomic teardown
    Given a goal "make all tests pass" is active for an armed session
    And the model's done call with summary "All tests green" was accepted into the registry
    When the early-exit decision is consulted after the done tool result is processed
    Then the decision consumes the acceptance and returns the summary "All tests green"
    And the shared teardown announces "🎯 goal satisfied: All tests green" and never "✓ done:"
    And the session goal is cleared and the CompletionContract reminder is removed from the conversation
    And the registry goal is cleared and both rejection counters reset to zero
    And the nudge counter resets and the turn terminates with the literal stop_reason "done"
    And session messages contain the paired done tool_use and tool_result

  Scenario: Immediate termination prevents repeat verify runs and acceptance overwrite
    Given a goal with a verify command that appends a line to a counter file is active for an armed session
    When the model's done call with evidence and a goal assessment passes the verify command
    Then the counter file records exactly one verify run
    And the early-exit decision consumes the acceptance so a second take finds nothing

  Scenario: Tier 1 rejected done() records a rejection and never exits early
    Given a goal is active for an armed session
    When the model calls done without evidence or a goal assessment
    Then the done call is rejected as a tool error and the rejection count becomes 1
    And the early-exit decision finds no acceptance and the loop continues
    And the session goal and CompletionContract reminder stay intact

  Scenario: Failing Tier 2 verify rejects done() without early exit
    Given a goal with the failing verify command "exit 1" is active for an armed session
    When the model calls done with evidence and a goal assessment
    Then the done call is rejected with the verify failure and the rejection is counted
    And the early-exit decision finds no acceptance and the loop continues

  Scenario: Settle-point escalation semantics are unchanged
    Given no acceptance is pending at the FinalResponse settle point
    When the goal continuation decision runs with four rejections
    Then the decision escalates for human review
    And the stall fast-path and budget exhaustion still escalate
    And a pending acceptance at the fallback still finishes with the summary before escalation is evaluated

  Scenario: Stream-loop wiring pins goal-mode early exit and single-teardown invariants
    Given the source file rust/cli/src/interactive/stream_loop.rs
    Then the ToolResult arm consults the early-exit decision without any goal gate
    And the FinalResponse fallback routes through the same shared teardown helper as the early exit
    And the goal announcement formatting lives only in the goal acceptance helper
    And CompletionContract reminder removal lives only in the session goal clearing method

  Scenario: Verify command exceeding the timeout rejects done() without early exit
    Given a goal with a verify command that sleeps beyond the configured test timeout is active for an armed session
    When the model calls done with evidence and a goal assessment
    Then the done call is rejected with a verify timeout and the rejection is counted
    And the early-exit decision finds no acceptance and the loop continues
