@wip
@deferred
@session-management
@rust
@agent-loop
@rpc
@retry
@net001
Feature: Agent loop classifies 429 errors and emits NET-001 reconnect notifications
  """
  RPC-087 (child of RPC-072 family). Error classification + retry must
  go through the recovery_network / compaction / thinking / truncation /
  stall / image helpers from rust/cli/src/interactive/ so 429s
  trigger NET-001 reconnect notifications instead of raw JSON in
  scrollback + dialog.

  Originally scenario "A transient 429 triggers NET-001 reconnect
  notifications, not a fatal error dialog" from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want a transient 429 to surface as a "Reconnecting..." notification
    So that I don't get an ErrorDialog modal for a recoverable rate limit

  Scenario: A transient 429 triggers NET-001 reconnect notifications, not a fatal error dialog
    Given a Work Agent session backed by a stub provider scripted to return HTTP 429 once then succeed
    When the user sends "hello"
    Then the scrollback receives a StreamChunk::UserNotification matching /Reconnecting/
    And the scrollback receives a final successful StreamChunk::Text from the assistant
    And no StreamChunk::Error is emitted
    And no ErrorDialog modal is pushed onto the Compositor
