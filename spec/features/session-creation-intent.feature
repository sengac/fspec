@done
@BUG-280
@BUG-281
@tui
@session-management
@integration
@rust
@phase-1
Feature: Board navigation and agent creation preserve explicit work intent
  """
  Board entry resolves authoritative open-session occupancy, using the earliest
  open representative. Disk-only sessions are not implicitly resumed.
  The shared creation dialog captures optional story intent independently of
  legacy global selection. Its title and asynchronous completion use that same
  intent. Completion binds the returned session ID explicitly, once, even when
  observation precedes acknowledgement. Generic initialization never attaches
  ambient selection. Existing revisioned focus and explicit resume remain
  authoritative. Mux entry uses this policy without leaving the grid.
  The original ten-scenario contract is preserved across three features to
  satisfy the one-feature/one-test-file workflow gate: this file contains the
  three board-entry scenarios; session-creation-completion.feature contains
  four completion scenarios; generic-agent-creation.feature contains three
  generic-creation scenarios, including New Agent starts unassigned.
  """

  Background: User Story
    As a developer using concurrent agents
    I want navigation and creation to respect explicit story intent
    So that unrelated and resumed agents keep their own work

  Scenario: Board entry navigates to the earliest associated open agent
    Given unrelated and multiple associated agents are open
    When the user enters the associated story from the board or mux
    Then the earliest associated agent is focused without a creation dialog
    And no agent assignment changes

  Scenario: Unassociated board entry offers story-specific creation
    Given a story has no associated open agent with or without unrelated agents
    When the user enters that story from the board or mux
    Then a story-specific dialog offers normal isolated and cancel choices
    And no session is created or reassigned before confirmation

  Scenario: Board entry preserves an explicitly resumed agent
    Given an explicitly resumed agent with its own revisioned story focus
    When the user enters an unrelated story and delayed older focus arrives
    Then story creation is offered without changing the resumed agent focus
