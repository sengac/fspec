@BUG-582
@RIG-016
@tui
@agent-view
Feature: Agent view keys steer and queue input while a turn runs
  """
  While the focused session's turn is running, the agent view routes submissions by mode instead of always calling
  send_input: plain Enter submits FspecBackend::submit_input(session, text, InputMode::Steer) and Alt+Enter submits
  InputMode::Queue. The input gate learns that a turn is running from the cached session status (as it already does
  for Compacting); Shift+Enter and the other modified Enters keep inserting a newline, and an idle session's Enter
  still sends a normal turn. Behaviour of the modes themselves: spec/features/interrupt-steer-and-queue-input.feature.
  """

  Background: User Story
    As a user directing a busy agent from the agent view
    I want Enter to steer and Alt+Enter to queue while a turn runs
    So that I can choose how my input reaches the agent without extra commands

  Scenario: Enter steers and Alt+Enter queues while a turn runs
    Given the agent view of a session whose turn is running
    When the user types "check the logs" and presses Enter
    Then the input is submitted as a steer
    And typing "and then deploy" with Alt+Enter submits it as a queued input
