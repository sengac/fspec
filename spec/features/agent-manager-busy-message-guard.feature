@agent-core
@agent-manager
@phase-1
Feature: Reject AgentManager messages while the recipient is busy
  """
  Both AgentManager handlers use the same BackgroundSession admission method.
  Admission atomically claims an idle recipient as running before publishing its message.
  Status changes use the existing session broadcasts so await_idle observes accepted work.
  Other incoming-message sources retain their existing delivery behavior.
  """

  Background: User Story
    As an agent coordinating other agents
    I want busy recipients to reject messages instead of queueing them
    So that redundant or outdated messages cannot accumulate while work is running

  Scenario: Reject messages to non-idle recipients
    Given a recipient that is running, paused, compacting, interrupted, or cleared
    When an agent sends the recipient a message with optional quoted context
    Then the response has error code "session_busy"
    And the response identifies the recipient and instructs the caller to use "await_idle" before sending again
    And the response advises returning results instead of waiting on a recipient that is waiting for the caller
    And no incoming message is queued and the recipient status is unchanged

  Scenario: Accept one message and immediately mark the recipient running
    Given an idle recipient
    When an agent sends the recipient a message
    Then the message is delivered with its sender identity intact
    And the recipient immediately reports running through its status and broadcast channels
    And await_idle does not report the recipient idle before it finishes

  Scenario: Reject a burst after the first accepted message
    Given an idle recipient
    When an agent sends one task followed by hundreds of redundant messages
    Then only the first task is queued
    And all subsequent messages return "session_busy"

  Scenario: Concurrent senders cannot queue multiple tasks to an idle recipient
    Given an idle recipient
    When multiple agents send messages concurrently
    Then exactly one message is accepted
    And all other sends return "session_busy"

  Scenario: Await completion before sending a fresh message
    Given a recipient processing an accepted task
    And a follow-up message has been rejected as busy
    When the caller awaits the recipient completing its task
    And sends a fresh follow-up message after the recipient becomes idle
    Then only the fresh follow-up message is delivered

  Scenario: Failed delivery leaves an idle recipient idle
    Given an idle recipient whose incoming message channel is full or closed
    When an agent sends the recipient a message
    Then the response has error code "delivery_failed"
    And the recipient remains idle with its pending message count unchanged

  Scenario: Active agents cannot send messages to themselves
    Given an actively running agent
    When it sends a message to its own session
    Then the response has error code "session_busy"
    And no self-addressed message is queued

  Scenario: Tool guidance explains how to message busy recipients
    Given an agent has access to the AgentManager tool description
    When it reads the message action guidance
    Then the guidance states that messages require idle recipients
    And it instructs the caller to use "await_idle" before sending to a busy recipient
