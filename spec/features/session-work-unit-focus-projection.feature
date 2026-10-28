@BUG-230
@tui
@session-management
@multi-session
@rust
Feature: Session card displays follow authoritative activity focus
  """
  Consume SESS-003 revisioned focus at the App single-writer boundary.
  The menu, AgentView header and board occupancy badge are projections of the
  same per-session focus. An explicit cleared state never uses global fallback.
  Board badges indicate occupancy; when several agents focus the same card,
  its navigation representative is the earliest open session while every
  session retains its independent menu chip and focus.
  """

  Background: User Story
    As a developer using concurrent agents
    I want every card display to follow the session's current work
    So that stale or unrelated cards never misrepresent an agent's activity

  Scenario: Switching focus updates the menu and board badge together
    Given an open session working on card "AUTH-001"
    When its authoritative focus changes to card "AUTH-002"
    Then its menu chip displays "AUTH-002" instead of "AUTH-001"
    And the board badge moves from "AUTH-001" to "AUTH-002"

  Scenario: A cleared session does not inherit another session's card
    Given a session working on "AUTH-001" and another session with no activity focus
    When the cleared session is displayed with stale global card selection present
    Then its agent header and menu contain no work-unit suffix
    And the other session remains focused on "AUTH-001"

  Scenario: An older update cannot resurrect cleared focus
    Given a session whose card focus was explicitly cleared
    When an older card update or duplicate cleared update arrives
    Then the session remains open without an active card
    And its board badge remains absent

  Scenario: A reconnect snapshot restores focus without local attachment
    Given a newly connected client observing two existing sessions
    When it receives authoritative card focus for one session and cleared focus for the other
    Then only the first session has a card suffix and a board badge
    And an older initial snapshot cannot replace a newer live focus update

  Scenario: Several sessions can share a card without losing their individual focus
    Given two open sessions focused on card "AUTH-001"
    When the earliest session clears its focus
    Then the other session remains focused on "AUTH-001"
    And the board badge still navigates to the remaining session

  Scenario: Explicit assignment into an existing session uses the authoritative focus path
    Given an existing session focused on card "AUTH-001" and card "AUTH-002" on the board
    When the user explicitly assigns card "AUTH-002" to that session
    Then the backend and all card displays identify "AUTH-002"
    And a delayed acknowledgement for the previous card cannot replace the newer focus

  Scenario: Focused card changes are visible on board agent and mux surfaces
    Given an open session with authoritative focus on card "AUTH-001"
    When its focus changes to card "AUTH-002" while each surface is rendered
    Then the board agent and mux menu chips display "AUTH-002"
    And none of those surfaces displays "AUTH-001" as that session's focus
