@done
@BUG-241
@providers
@tui
@integration
@provider-abstraction
Feature: Transient provider waiting
  """
  A small additive state-only ProviderWaiting chunk carries an optional safe
  message through existing embedded and WebSocket transports. It is never a
  transcript notification. Per-session waiting text overrides the running
  composer spinner only; it does not change session lifecycle status. Clear
  waiting on output, terminal state, new input, and provider/session teardown.
  Observer attachment synchronizes current waiting, including an explicit clear;
  clears are state-only updates and never inactive transcript notifications.
  """

  Scenario: Waiting changes chrome without entering the transcript
    Given a running session and a transient provider waiting update
    When the waiting update is applied repeatedly and then cleared
    Then the running chrome reflects only the current waiting message
    And the transcript and other sessions remain unchanged

  Scenario: Output and turn boundaries dismiss waiting
    Given a running session showing provider waiting
    When assistant output or a terminal or input boundary arrives
    Then waiting is cleared without changing normal session behavior
    And no waiting message is captured in transcript or finish animations
