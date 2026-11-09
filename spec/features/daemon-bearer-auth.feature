@BUG-582
@RPC-447
@rpc
@multi-workspace
Feature: Daemon bearer-token authentication
  """
  SharedFspecService::set_auth_token(token) arms the WebSocket server (codelet-rpc-server
  handle_connection): a handshake whose Authorization header is not "Bearer <token>" is answered
  with HTTP 401. codelet_rpc_server::ws_connect(url, bearer) opens a client socket sending the
  header; WebSocketFspecBackend carries the bearer through its reconnect supervisor.
  """

  Background: User Story
    As a developer exposing a daemon beyond loopback
    I want the daemon to require a bearer token
    So that only clients holding the token can drive it

  Scenario: A token-protected daemon rejects a handshake without the bearer
    Given a daemon service armed with token "s3cret"
    When a client connects without an Authorization header
    Then the handshake is rejected with HTTP 401

  Scenario: A client with the matching bearer is served
    Given a daemon service armed with token "s3cret"
    When a client connects with bearer "s3cret" and lists work units
    Then the RPC succeeds

  Scenario: A daemon without a token accepts plain clients
    Given a daemon service without a token
    When a client connects without an Authorization header and lists work units
    Then the RPC succeeds
