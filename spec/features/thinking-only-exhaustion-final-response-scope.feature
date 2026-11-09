@done
@CONT-012
@cli
@streaming
@completion
@thinking-exhaustion
Feature: Thinking-only exhaustion is judged on the final response and never stops an armed turn silently
  """
  Thinking-only detection (BUG-221) is scoped to the provider response attempt that
  produced the FinalResponse: reasoning, visible text and tool calls are tracked from the
  latest ResponseAttemptStarted. Tool calls completed by earlier requests in the same
  user turn are not progress for the final response and never suppress recovery.
  The recovery prompt preserves only the final response's captured reasoning.

  When auto-continue or goal mode is armed and a thinking-only length final remains
  unrecovered after the bounded PROV-041 retries, the FinalResponse settle point treats it
  as a zero-progress stop: nudge within budget, then the existing exhaustion warning or
  goal escalation. The emitted done stop_reason stays the provider's real stop reason.
  Unarmed sessions, useful or plain-text truncation, non-length stops, terminal errors
  and user interrupts keep their existing behaviour.

  Fix sites: cli/src/interactive/stream_loop/assistant.rs (per-response tracking),
  final_thinking.rs (detection and exhausted flag), final_completion.rs (settle stop).
  """

  Background: User Story
    As a developer running an armed auto-continue session on a reasoning model
    I want to have turns that end thinking-only at the output limit recover or continue instead of stopping silently
    So that long agentic work keeps going without me typing continue

  Scenario: Thinking-only length final after earlier tool progress is recovered in an armed session
    Given an armed session whose first request completes a tool call
    And the next response contains only thinking and stops for length
    When the turn runs to completion
    Then exactly one thinking exhaustion retry is sent
    And the retry prompt preserves only the final response's reasoning
    And the retry's answer finishes the turn without consuming an auto-continue nudge

  Scenario: Thinking-only length final after earlier tool progress is recovered with auto-continue disabled
    Given an unarmed session whose first request completes a tool call
    And the next response contains only thinking and stops for length
    When the turn runs to completion
    Then exactly one thinking exhaustion retry is sent

  Scenario: Armed session nudges after thinking exhaustion recovery is exhausted
    Given an armed session with an auto-continue budget of 1
    And every response contains only thinking and stops for length
    When the turn runs to completion
    Then two thinking exhaustion retries are followed by one auto-continue nudge
    And the auto-continue budget exhaustion warning is shown
    And the turn's done event reports the normalized provider stop reason max_tokens

  Scenario: Unarmed session finishes after thinking exhaustion recovery is exhausted
    Given an unarmed session where every response contains only thinking and stops for length
    When the turn runs to completion
    Then exactly two thinking exhaustion retries occur before completion
    And the thinking exhaustion warning is visible and no auto-continue nudge is consumed

  Scenario: Goal mode escalates a stalled thinking-only turn instead of finishing silently
    Given a session with an active goal and auto-continue disabled
    And every response contains only thinking and stops for length
    When the turn runs to completion
    Then two auto-continue nudges are sent after the thinking exhaustion retries
    And the goal stall escalation message is shown
