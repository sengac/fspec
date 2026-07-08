@done
@rpc
@reconnect
@rust
@connection
@tui
@RPC-416
Feature: Inline reconnect status in scrollback (replace-in-place + auto-dismiss)
  """
  Lost affordances decision (RPC-416): removing the DisconnectDialog drops its 'q to quit' / 'r to reconnect' (ManualReconnect) keybindings. Decision: rely entirely on the transport supervisor's automatic capped-backoff reconnect (250ms->5s) which recovers without any manual key, so no affordance is re-homed. Canonical inline line strings the implementation must emit: disconnect/retry line contains 'Reconnecting' (and '(attempt N)' once Action::Reconnecting(n) is seen); success line contains 'Reconnected'. Lines are ChunkKind::Notification chunks pushed/replaced/removed by stable seq on the ORIGINATING session (tracked as (SessionId, seq) on App state), never by current focus.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On disconnect, an inline reconnecting status line appears in the focused session's scrollback and no DisconnectDialog modal is shown
  #   2. Each reconnect attempt updates the same status line in-place (attempt count); no additional reconnect lines are pushed
  #   3. On successful reconnect the same line is replaced in-place with a success message, then auto-dismissed from scrollback after a short delay
  #   4. Replace and remove always target the originating session by SessionId, even if the focused session changed after the disconnect
  #   5. If a new disconnect occurs during the success-display window, the pending auto-dismiss is cancelled and the line reverts to a reconnecting state
  #   6. If the originating session is closed before the auto-dismiss timer fires, the removal is a silent no-op with no panic
  #   7. The DisconnectDialog modal is removed entirely and never appears for any disconnect/reconnect flow
  #
  # EXAMPLES:
  #   1. On disconnect while connected to a session, the focused session's transcript shows a reconnecting line and the modal never covers the screen
  #   2. During retries the single line reads 'reconnecting... (attempt 3)' and the transcript still has only one reconnect line
  #   3. When the connection recovers the reconnecting line turns into a reconnected success line in the same spot
  #   4. A couple of seconds after reconnecting, the success line disappears and the transcript is clean again
  #   5. If the connection drops again right after the success line shows, the line goes back to reconnecting instead of vanishing
  #   6. If the session that showed the reconnecting line is closed before the success line auto-clears, nothing crashes and no stale line is left behind
  #
  # ASSUMPTIONS:
  #   1. Transport auto-reconnect (250ms->5s capped backoff) recovers the connection without any manual key, so removing the modal's q/r keybindings does not lose required functionality
  #
  # ========================================
  Background: User Story
    As a fspec power-user driving the Rust TUI over WebSocket
    I want to see reconnect progress inline in my session transcript that updates in place and clears itself once reconnected
    So that I get clear, non-intrusive feedback about connection recovery without a modal interrupting my view

  Scenario: Disconnect shows an inline reconnecting line in the focused session
    Given a focused session with an open transcript
    When the connection drops and Action::Disconnected is dispatched
    Then the focused session's scrollback gains a single inline reconnecting status line
    And no DisconnectDialog modal is present on the compositor

  Scenario: The DisconnectDialog modal never appears for a disconnect or reconnect flow
    Given a focused session with an open transcript
    When the connection drops, retries, and then recovers
    Then the compositor never contains the disconnect-dialog layer at any point in the flow

  Scenario: Each reconnect attempt updates the same inline line in place
    Given a focused session showing an inline reconnecting line after a disconnect
    When Action::Reconnecting(3) is dispatched for the third retry attempt
    Then the same inline line updates in place to show the attempt count
    And the focused session's scrollback still contains only one reconnect line

  Scenario: Successful reconnect replaces the inline line in place with a success message
    Given a focused session showing an inline reconnecting line after a disconnect
    When the connection recovers and Action::Reconnected is dispatched
    Then the same inline line is replaced in place with a reconnected success message
    And the focused session's scrollback still contains only one reconnect line

  Scenario: The success line auto-dismisses from scrollback after the short delay
    Given a focused session showing an inline reconnected success line after a recovery
    When the auto-dismiss delay elapses and the pending clear action is processed
    Then the success line is removed from the focused session's scrollback
    And the focused session's scrollback contains no reconnect line

  Scenario: A re-drop during the success window cancels the dismiss and reverts to reconnecting
    Given a focused session showing an inline reconnected success line after a recovery
    When the connection drops again before the auto-dismiss delay elapses
    Then the inline line reverts to a reconnecting status
    And the line is still present after the original auto-dismiss delay would have elapsed

  Scenario: Replace and remove target the originating session even after focus changes
    Given a focused session A showing an inline reconnecting line after a disconnect
    And a second session B is created and becomes focused
    When the connection recovers and Action::Reconnected is dispatched
    Then session A's scrollback shows the reconnected success line
    And session B's scrollback contains no reconnect line

  Scenario: Closing the originating session before the timer fires is a silent no-op
    Given a focused session A showing an inline reconnected success line after a recovery
    When session A is closed before the auto-dismiss delay elapses
    And the pending clear action is processed after the delay
    Then no panic occurs and no stale reconnect line remains

  Scenario: A re-drop after the originating session closed shows a fresh reconnecting line in the focused session
    Given a focused session A showing an inline reconnected success line after a recovery
    And session A is closed and session B becomes focused
    When the connection drops again before the auto-dismiss delay elapses
    Then session B's scrollback shows an inline reconnecting line
    And no stale reconnect notice remains tracked for the closed session A
