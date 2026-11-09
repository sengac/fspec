@BUG-582
@done
@workflow-automation
@websocket
@tarpc
@rust
@reconnect
@resilience
@connection
@cli
@dialog
@rpc
@RPC-011
Feature: DisconnectDialog CR-1 baseline (action bus + critical dialog + r/q handlers)
  """
  CR-1 baseline absorbed from RPC-010 review. The DisconnectDialog Component is pushed onto the
  Compositor at Priority::Critical when Action::Disconnected fires. While topmost, j/k/?/Tab are
  no-ops; only 'q' (quit App) and 'r' (manual reconnect — resets backoff) are honoured. Action
  enum lives in rust/fspec-tui/src/components/mod.rs (additive variants Disconnected /
  Reconnecting(u32) / Reconnected / ManualReconnect). DisconnectDialog uses the same tui-popup
  widget as RPC-008's HelpDialog — NO new dialog framework.
  TUI-132 (2026-11-09): the DisconnectDialog, its q/r keys and Action::ManualReconnect are removed: nothing showed the dialog since RPC-416 reports a drop inline. Action::Disconnected / Reconnecting / Reconnected remain.
  """

  Background: User Story
    As a fspec power-user driving the rust binary
    I want to have the fspec / fspec daemon / fspec client trio survive realistic interruptions (daemon restart, SIGTERM, multi-client attach, stale daemon.json) and tell me what's going on via fspec status + a health RPC
    So that I can day-drive the rust frontend without the rough edges flagged by RPC-010 review (CR-1 baseline + reconnect supervisor)

  Scenario: WebSocketFspecBackend surfaces WS disconnect as Action::Disconnected
    Given a fspec daemon is running on 127.0.0.1:<port> and a fspec client is attached via WebSocketFspecBackend::connect_with_supervisor
    And the client's App has finished bootstrap (work-units seeded, session created, three subscriber tasks alive)
    When the daemon process is killed
    Then within one render tick the App's action bus receives an Action::Disconnected message
    And the WebSocketFspecBackend's internal client slot becomes None
    And subsequent RPC calls on the backend return Err(BackendError::Disconnected) rather than panicking or hanging

  Scenario: Action::Disconnected pushes an inline reconnecting line, not the DisconnectDialog modal
    Given an App driving a ratatui TestBackend with a WebSocketFspecBackend whose connection has just dropped
    When the action loop processes Action::Disconnected
    Then no DisconnectDialog modal layer is pushed onto the Compositor
    And the focused session's scrollback gains a single inline reconnecting status line
