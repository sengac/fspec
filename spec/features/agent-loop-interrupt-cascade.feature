@wip
@deferred
@session-management
@RPC-088
@rust
@agent-loop
@rpc
@interrupt
Feature: Agent loop honours Esc and emits StreamChunk::Interrupted
  """
  RPC-088 (child of RPC-072 family). Interrupt cascade must consult
  session.is_interrupted and select against
  session.interrupt_notify.notified() inside the stream loop so Esc
  aborts the active provider call and emits StreamChunk::Interrupted.

  Originally scenario "Esc aborts an in-flight provider call and emits
  StreamChunk::Interrupted" from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want pressing Esc during an LLM call to actually cancel the call
    So that I can recover from a runaway response without restarting the session

  Scenario: Esc aborts an in-flight provider call and emits StreamChunk::Interrupted
    Given a Work Agent session backed by a stub provider scripted to stream for 30 seconds
    When the user sends "long task" and presses Esc 200ms later
    Then within 100ms after Esc the scrollback receives a StreamChunk::Interrupted
    And the agent loop returns to status SessionStatus::Idle
    And subsequent input is accepted without restarting the session
