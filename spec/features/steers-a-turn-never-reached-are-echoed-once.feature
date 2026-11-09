@done
@BUG-348
@session
@agent-loop
@input
@agent-interaction
@bug
@regression
Feature: Steers a turn never reached are echoed once
  """
  BUG-348 (last gap from TUI-115): BackgroundSession::submit_input echoes a steer as soon as it is submitted (input_modes.rs, the (true, Steer) arm) and parks it with input_inbox::push_steer. If the running turn ends before a safe boundary drains it, continue_with_inbox joins the parked steers with "\n\n" and starts them as a new turn through send_input. send_input echoed StreamChunk::user_input(joined) again, so the steer showed twice ('You (steer): a' and 'You: a'). Fix: continue_with_inbox starts the new turn through the same submission path without the echo, because each steer already echoed.
  Test file: rust/agent-loop/tests/bug348_steer_echoed_once.rs (reuses the RIG-016 harness).
  """

  Background: User Story
    As a user steering a running agent turn
    I want each steer to appear once in the transcript
    So that a steer the turn never reached does not look like a second, separate message

  Scenario: A steer the running turn never reached is echoed once when it starts the next turn
    Given a session whose turn is streaming a text reply with no tool call
    When the user steers "use the staging database" and the turn completes without reaching it
    Then the next turn starts with "use the staging database"
    And the session emitted exactly one user input echo for "use the staging database"
