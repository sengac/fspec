@done
@phase-1
@tui
@session-management
@BUG-220
Feature: Interruption reason rendering
  """
  SessionContext records typed interruption notices in scrollback using the existing Interrupted chunk kind. Older reasonless events retain the generic notice instead of being guessed to mean user cancellation.
  """

  Background: User Story
    As a session owner
    I want interruption notices to name their cause
    So that a safety abort is not mistaken for Escape

  Scenario: Scrollback distinguishes user and detector interruption with legacy fallback
    Given user and loopDetector interruption events and a legacy reasonless event
    When the events enter session scrollback
    Then the notices name explicit causes and keep the legacy notice generic
