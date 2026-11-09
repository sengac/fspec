@done
@BUG-328
@session-management
@rpc
@tui
Feature: send_input refuses sessions the daemon does not hold
  """
  send_input and send_input_with_thinking returned () on every layer, so input for a session the daemon did not hold was either dropped with a log line or, since BUG-287, silently resumed the session from disk (SessionManager::live_or_resumed). Both RPCs now return Result<(), String> through the RPC declaration and implementation, MCP dispatch, SessionManagerHandle (and its stub), the sessions handle, FspecBackend and both TUI transports. Input is accepted only for a live session; any other id is refused with "Session not found: <id> (resume it with resume_session before sending input)" and nothing is resumed implicitly (submit_input follows the same rule). The TUI shows a refused send as an [error] line in the session's scrollback.
  """

  Background: User Story
    As a client sending input to a daemon session
    I want input to a session the daemon does not hold to fail
    So that I resume the session explicitly instead of waiting for a turn that never starts

  Scenario: send_input over a transport reports a session the daemon does not hold
    Given a daemon session manager that holds no session "missing-session"
    When a client calls send_input and send_input_with_thinking for "missing-session" over the embedded and websocket transports
    Then every call returns an error naming "missing-session" and telling the client to resume it

  Scenario: A refused send is shown in the session's scrollback
    Given the TUI is showing a session whose input the daemon refuses
    When I submit "hello" in that session
    Then the session's scrollback shows the send error
