@done
@phase-1
@provider-abstraction
@providers
@integration
@BUG-219
@critical
Feature: Codex interruption output
  """
  The session provider observer forwards Codex recovery notifications to the same broadcast stream used by the TUI. Ordinary stream drops at the shared user or internal interruption boundary must not publish a reconnection-specific notification. Interruption events and turn completion remain the responsibility of the caller.
  """

  Background: User Story
    As a Codex session owner
    I want session output to report only genuine network recovery
    So that ordinary interruption does not imply a network outage

  Scenario: Ordinary interrupted streaming stays silent in session output
    Given a Codex session forwarding provider notifications to session output
    When an ordinary partial stream is dropped at the interruption boundary
    Then session output contains no reconnection notification and recovery remains inactive
