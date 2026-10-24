@BUG-210
@phase-1
@agent-core
@context-management
@compaction
@napi
Feature: NAPI sessions share automatic compaction continuation
  """
  The NAPI lifecycle adapter dispatches through the shared background agent loop.
  Concurrent sessions publish their durable context, progress, and completion on
  the session manager broadcast with their originating session identity intact.
  """

  Background: User Story
    As a user running concurrent NAPI sessions
    I want unfinished work to continue after automatic compaction
    So that changing session focus cannot redirect another agent's events

  Scenario: NAPI sessions use the shared automatic recovery lifecycle
    Given an unfinished NAPI session encounters a context overflow
    When its session hook runs the agent loop
    Then a committed summary is followed by inference without new user input
    And its events retain the originating session identity
