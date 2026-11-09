@RIG-016
@session
@multi-session
@BUG-546
Feature: Interrupt, steer and queue input to a busy agent
  """
  Every input submitted to a busy session carries a mode. Steer (the default for Enter) is held in the session's
  in-process inbox (codelet_common::input_inbox) and delivered at the next safe boundary of the running turn: after the current model step, or once
  every pending tool result has settled, and before the next provider request, so tool-call history stays valid and no
  in-flight generation is altered. Queue (Alt+Enter) is held in the same inbox and starts as a new turn when the running
  turn completes, in submission order; a queued submit to an idle session resumes any parked inputs ahead of it. Interrupt (Esc) cancels the running turn; queued inputs stay parked, are listed in
  the Interrupted chunk's queued_inputs, and are only sent when the user resumes. Inputs keep their session; nothing is
  silently dropped. Research: spec/attachments/RIG-016/fspec-interruption-modes-research.md.
  BUG-546: rig runs each tool call as soon as it streams, so a tool result can arrive while the same response still has
  calls to stream. A steer is therefore delivered only when the response has finished and every call it requested has its
  real result, i.e. at the next request's ResponseAttemptStarted before the request is sent. Delivering after the first
  result failed Codex turns ('Codex replay has no recorded outcome for call ...') and silently dropped the remaining calls
  for the other providers.
  """

  Background: User Story
    As a user directing a busy agent
    I want to steer it mid-turn, queue a follow-up, or interrupt it
    So that I can correct course without losing work or my pending instructions

  Scenario: A steered input reaches the next provider request after the tool result
    Given a session whose turn is running a tool call
    When the user steers "use the staging database"
    Then the tool result is recorded before the steered input
    And the next provider request of the same turn carries "use the staging database"

  Scenario: A steer waits for every tool call of the response before it is delivered
    Given a session whose model response requested two tool calls and the first is still running
    When the user steers "use the staging database"
    Then both tool calls run and both real results are recorded
    And the next provider request carries both tool calls and both results before "use the staging database"
    And the turn continues in that request

  Scenario: Codex replays a steered request only when every call has its outcome
    Given a Codex checkpoint whose response requested the calls "call_1" and "call_2"
    When a steered request carries only the result of "call_1"
    Then the request is refused because "call_2" has no recorded outcome
    When the steered request carries the results of "call_1" and "call_2" before the steer
    Then the request is accepted with an output for every call

  Scenario: A queued input starts a new turn after the running turn completes
    Given a session whose turn is running
    When the user queues "then update the changelog"
    Then the running turn does not receive the queued input
    And when the turn completes a new turn starts with "then update the changelog"

  Scenario: Queued inputs start in submission order
    Given a session whose turn is running
    When the user queues "first follow-up" and then "second follow-up"
    Then the follow-ups start as separate turns in that order

  Scenario: Interrupting keeps queued inputs parked
    Given a session whose turn is running with two queued inputs
    When the user interrupts the turn
    Then the turn stops and the interrupted notice lists both queued inputs
    And neither queued input is sent

  Scenario: Resuming sends the parked inputs
    Given an interrupted session with a parked queued input
    When the user resumes the session
    Then the parked input starts as a new turn
