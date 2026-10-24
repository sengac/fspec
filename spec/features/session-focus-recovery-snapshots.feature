@BUG-230
@tui
@rpc
@session-management
@rust
Feature: Session focus recovery snapshots reach connected clients
  """
  SESS-003 sends full session snapshots after a lagging server subscription.
  These are live recovery frames, not only connection-time bootstrap data.
  The WebSocket focus adapter forwards revisioned focus from snapshots as well
  as individual events, leaving stale-frame rejection to the per-session store.
  """

  Background: User Story
    As a developer using a connected TUI
    I want live recovery snapshots to repair missed focus changes
    So that a lagging connection cannot leave a stale card indefinitely

  Scenario: Live recovery snapshots deliver explicit cleared activity
    Given a WebSocket backend subscribed to session focus updates
    When the server sends a recovery snapshot with an active card followed by explicit cleared focus
    Then the subscriber receives both focus states with their authoritative revisions
    And it does not need to reconnect or wait for another card change
