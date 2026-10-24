@done
@SESS-003
@session
@rpc
@persistence
@session-management
Feature: Authoritative session work-unit focus
  """
  Activity focus is separate from authorization and worktree binding. A shared
  session service resolves card metadata against the invoking session project.
  Revisioned snapshots and events carry explicit cleared state and change source.
  Manifest updates use the existing serialized writer transaction. Legacy manifests
  restore as NONE; completed or deleted cards reconcile to NONE. Rig remains
  independent of session management. Busy inner locks defer reminder reconciliation.
  """

  Background: User Story
    As an operator using concurrent agent sessions
    I want authoritative durable activity focus
    So that all clients and tools agree without relaxing session permissions

  Scenario: Session focus transitions are ordered isolated and project validated
    Given two live sessions with cards in their own project
    When one session selects a card switches cards and explicitly clears focus
    Then revisions increase only for changes and the other session remains unchanged
    And unknown cards are rejected without changing focus
    And clearing activity retains authorization and worktree bindings

  Scenario: Busy session reminders reconcile the latest focus without deadlock
    Given a live session whose inner agent lock is held
    When its focus changes twice and is explicitly cleared
    Then the changes return without waiting for the inner lock
    And the next environment reminder reflects the latest cleared focus only

  Scenario: Focus restore preserves history and reconciles missing or completed cards
    Given persisted sessions with active cleared legacy completed and deleted focus
    When the real session manager restores their manifests
    Then active focus uses current card metadata and other focus is explicitly NONE
    And concurrent message persistence is preserved
    And clearing activity retains restored authorization bindings
    And session search reports persisted focus rather than guessing from session names
