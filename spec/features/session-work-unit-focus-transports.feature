@done
@SESS-003
@session
@rpc
@session-management
Feature: Session work-unit focus across transports
  """
  Both transports delegate to the same session focus API. Focus changes carry
  session identity, revision and source. SessionInfo initial snapshots include
  explicit NONE; clients only apply newer revisions to avoid stale resurrection.
  """

  Background: User Story
    As an operator connected through multiple clients
    I want revisioned activity focus events and reconnect snapshots
    So that every client observes the same authoritative session state

  Scenario: Embedded and WebSocket clients receive focus events and reconnect snapshots
    Given a real session manager shared by embedded and WebSocket clients
    When a client selects switches and clears session focus
    Then every subscribed client receives the same revisioned focus changes
    And a reconnecting client receives the authoritative cleared snapshot
    And an older snapshot cannot supersede a newer focus revision
