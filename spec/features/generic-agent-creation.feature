@done
@BUG-281
@tui
@session-management
@integration
@rust
@phase-1
Feature: Generic agent creation never inherits ambient work selection
  """
  New Agent and right-edge Shift Right use the shared dialog with absent story
  intent, both normal and isolated. Generic initialization and remote observation
  never bind global selection. Navigation within open agents is not creation.
  Resumed and existing sessions retain their own authoritative focus.
  """

  Background: User Story
    As a developer using concurrent agents
    I want generic new agents to start without a story
    So that neither stale board selection nor another agent supplies their work

  Scenario: New Agent starts unassigned despite stale selection
    Given stale board selection and a current agent with different or cleared focus
    When New Agent is confirmed with normal or isolated creation
    Then the dialog is generic and the new session has no story
    And existing authoritative focus is unchanged

  Scenario: Shift Right navigates before offering unassigned creation
    Given multiple open agents and stale board selection
    When Shift Right navigates within agents then creates at the right edge in agent or mux mode
    Then navigation creates nothing and the confirmed new agent is unassigned
    And existing authoritative focus is unchanged

  Scenario: Generic session observation and completion never attach ambient selection
    Given stale board selection and an unrelated focused agent
    When a generic session is observed and creation completion is delivered repeatedly
    Then the session is initialized once without any story assignment
    And no unrelated focus is cleared or reassigned
