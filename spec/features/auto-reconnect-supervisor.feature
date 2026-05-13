@done
@workflow-automation
@websocket
@tarpc
@rust
@reconnect
@resilience
@connection
@rpc
@RPC-011
Feature: Auto-reconnect supervisor (exponential backoff + Reconnecting/Reconnected actions)

  """
  WebSocketFspecBackend gains a second constructor `connect_with_supervisor(url, action_tx)` that
  spawns a reconnect supervisor task living at the transport layer. Backoff schedule is 250ms →
  500 → 1000 → 2000 → 5000 cap; reset on first successful frame. Each retry emits
  Action::Reconnecting(attempt: u32) so the DisconnectDialog updates inline. On successful
  reconnect: re-issue list_work_units + create_session(None) + resubscribe broadcasts, then emit
  Action::Reconnected so the App pops the dialog. SIGTERM observable side: client sees WS Close
  frame with reason="going_away" (RFC 6455 code 1001) within 100 ms and starts the backoff loop.
  """

  Background: User Story
    As a fspec power-user driving the rust binary
    I want to have the fspec / fspec daemon / fspec client trio survive realistic interruptions (daemon restart, SIGTERM, multi-client attach, stale daemon.json) and tell me what's going on via fspec status + a health RPC
    So that I can day-drive the rust frontend without the rough edges flagged by RPC-010 review (CR-1 baseline + reconnect supervisor)

  Scenario: Auto-reconnect backoff schedule
    Given a fspec client whose daemon has just died
    And the supervisor task has been spawned by WebSocketFspecBackend::connect_with_supervisor
    When the daemon stays dead for 60 seconds
    Then the supervisor emits Action::Reconnecting(attempt) frames with the following delays before each attempt:
      | attempt | delay_ms |
      | 1       | 250      |
      | 2       | 500      |
      | 3       | 1000     |
      | 4       | 2000     |
      | 5       | 5000     |
      | 6       | 5000     |
      | 7       | 5000     |
    And the attempt counter is strictly monotonically increasing

  Scenario: Auto-reconnect happy path
    Given a fspec client whose daemon has just died
    And the DisconnectDialog is topmost showing "auto-reconnecting (attempt 1)…"
    When a new fspec daemon binds the same port within 2 seconds
    Then the supervisor's next connect_async succeeds
    And the supervisor re-issues list_work_units + create_session(None) on the new client
    And it respawns the three subscriber tasks against the new chunks/logs/work_units broadcasts
    And it emits Action::Reconnected on the App action bus
    And the App pops the DisconnectDialog from the Compositor
    And the WorkUnitsListView re-seeds from the snapshot returned by the new daemon

  Scenario: Reconnect re-issues create_session(None) and replaces active session id
    Given a fspec client with active_session = SessionId("S-old") before disconnect
    When the supervisor reconnects against a fresh daemon
    Then it calls backend.create_session(None) and gets back SessionId("S-new")
    And it emits Action::SessionCreated(SessionId("S-new")) onto the action bus
    And the App's repl_active_session() returns Some(SessionId("S-new"))
    And the REPL transcript is NOT destructively truncated (old transcript lines remain on screen)

  Scenario: Auto-reconnect Reconnecting Action updates the dialog text inline
    Given a TestBackend App with the DisconnectDialog topmost
    When the action loop dispatches Action::Reconnecting(3)
    Then the rendered Buffer contains "auto-reconnecting (attempt 3)…"
    And no new dialog layer is pushed (the existing DisconnectDialog mutates state)

  Scenario: Client receives ServerGoingAway when daemon shuts down gracefully
    Given a fspec client connected to a daemon
    When the daemon receives SIGTERM and broadcasts a WS Close frame with reason "going_away" (RFC 6455 code 1001)
    Then the client's WebSocketFspecBackend observes the close-with-reason inside 100 ms
    And it emits Action::Disconnected onto the App action bus
    And the DisconnectDialog renders the same "daemon disconnected | r to reconnect | q to quit" text
    And the supervisor starts the same 250ms-first-attempt backoff loop
