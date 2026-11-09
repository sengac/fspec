@done
@tui
@connection
@bug-fix
@rust
@BUG-667
Feature: connect_with_supervisor trusts the daemon's CA
  """
  Since RPC-458 every client connect trusts the public web PKI roots plus FSPEC_DAEMON_CA (and a saved daemon's CA), except WebSocketFspecBackend::connect_with_supervisor, whose first connect used tokio_tungstenite::connect_async with the public roots only, so a private-CA daemon was refused until the reconnect supervisor retried through ws_connect. The first connect now goes through codelet_rpc_server::ws_connect too. RPC-008's source-shape scenario that pinned connect_async is retired in favour of this behaviour test.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. WebSocketFspecBackend::connect_with_supervisor makes its first connect through codelet_rpc_server::ws_connect, so it trusts the public roots plus FSPEC_DAEMON_CA like every other client connect (RPC-458)
  #   2. RPC-008's source-shape scenario that pinned tokio_tungstenite::connect_async in the backend is retired; a behaviour test against a private-CA wss daemon replaces it
  #
  # EXAMPLES:
  #   1. FSPEC_DAEMON_CA names the test CA: connect_with_supervisor(wss://localhost:<port>) of a daemon serving the test certificate is connected at once (list_work_units answers, no Disconnected)
  #   2. Without FSPEC_DAEMON_CA the first connect is refused: Action::Disconnected arrives and list_work_units reports the backend disconnected
  #
  # ========================================
  Background: User Story
    As a user whose TUI connects to a private-CA daemon
    I want every TUI WebSocket connect, including the supervised one, to trust FSPEC_DAEMON_CA
    So that the first connect behaves like every reconnect instead of failing the TLS check

  Scenario: A private-CA daemon is reachable through connect_with_supervisor with FSPEC_DAEMON_CA
    Given a TLS daemon serving a certificate for localhost signed by the test CA
    And FSPEC_DAEMON_CA names the test CA
    When WebSocketFspecBackend::connect_with_supervisor connects to wss://localhost:<port>
    Then list_work_units answers at once and no Action::Disconnected was sent

  Scenario: Without FSPEC_DAEMON_CA the first connect is refused
    Given the same TLS daemon and no FSPEC_DAEMON_CA
    When WebSocketFspecBackend::connect_with_supervisor connects to wss://localhost:<port>
    Then Action::Disconnected arrives and list_work_units reports the backend disconnected
