@done
@BUG-272
@cli
@providers
@provider-abstraction
@error-handling
@integration
@phase-1
Feature: Preserve Codex history after terminal provider errors
  """
  The shared CLI/NAPI streaming consumer must preserve Codex checkpoint ancestry
  and genuine tool outcomes when terminal inference fails. BUG-170/BUG-185 tail
  deletion remains a legacy non-Codex policy, not a Codex recovery mechanism.
  Keep earlier context-overflow recovery and the final explicit error return.
  Never fabricate cancellation, weaken replay validation or reset checkpoints.
  Integration tests use the real streaming consumer, scripted provider transport,
  real tool side effects and temporary filesystem persistence.
  """

  Background: User Story
    As a Codex session owner
    I want to continue safely after a terminal provider error
    So that completed tool work is not lost or executed again

  Scenario: A later prompt preserves completed work after a provider rejection
    Given a Codex tool has completed a real filesystem change
    When the following inference receives a terminal provider rejection
    Then the original tool call and genuine outcome remain unchanged
    And the provider error is surfaced without claiming context was removed
    And a later prompt replays the outcome exactly once without repeating the tool

  Scenario: Cold resume preserves completed work after overload
    Given a Codex tool has completed a real filesystem change
    When the following inference receives a terminal provider rejection
    And the conversation and checkpoint are saved and cold reloaded
    Then the original tool call and genuine outcome remain unchanged
    And a later prompt replays the outcome exactly once without repeating the tool

  Scenario: A rejected continuation preserves every completed batch outcome
    Given a Codex batch contains successful work and a genuine tool error
    When the following inference receives a terminal provider rejection
    Then all invocation identities and genuine outcomes remain unchanged
    And a later prompt replays each outcome exactly once without repeating the tools

  Scenario: A rejected first request preserves the user message
    Given a new Codex conversation without tool calls
    When inference receives a terminal provider rejection
    Then the original user message remains in conversation history
    And the provider error is surfaced without claiming context was removed
    And a later prompt reaches the provider with both user messages

  Scenario: An unresolved committed call still fails replay safely
    Given a Codex checkpoint contains a call without a genuine outcome
    When I try to continue through the streaming consumer
    Then the original call and checkpoint remain unchanged
    And replay fails explicitly before sending a request
    And no successful completion or invented tool outcome is published

  Scenario: Network setup failure preserves completed work across cold resume
    Given a Codex batch has completed a filesystem change and a genuine tool error
    When the continuation connection closes before response headers arrive
    Then the setup failure is explicit and every original call and outcome remains unchanged
    And cold resume replays each real outcome exactly once without repeating side effects
