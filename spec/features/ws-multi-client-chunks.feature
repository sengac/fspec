@done
@streaming
@rust
@tarpc
@rpc
@websocket
@fanout
@RPC-007
Feature: WebSocket multi-client unfiltered chunk fan-out
  """
  Multi-client fan-out is unfiltered in this card: every connected WebSocket
  client receives every session's StreamChunks regardless of which client
  created the session. The server's per-connection chunks_fanout task drains
  SharedFspecService::chunks_rx() and emits Envelope::Event { session_id, chunk }
  frames to its own outbound queue; there is no per-client subscription filter
  or read-only observer mode in this card (those are explicitly deferred to a
  follow-up card per the RPC-002 §11 risks list). The sibling logs_fanout task
  follows the same unfiltered pattern for LogRecord and is covered by
  ws-log-event.feature; this feature focuses on the chunk path only.
  """

  Background: User Story
    As a developer connecting two WebSocket clients to the same rpc-server
    I want both clients to see every session's chunks regardless of which client created the session
    So that observer-style tooling and multi-pane UIs work in the simplest possible mode before per-client subscription filters are introduced

  @rpc
  @websocket
  @fanout
  Scenario: Multi-client unfiltered fan-out delivers every session's chunks to every connected client
    Given two WebSocket clients A and B are connected to the same codelet-rpc-server
    And both clients have subscribed to FspecWsClient::chunks_rx()
    When client A calls create_session(None) and send_input(session_id, "hi")
    Then client A observes the StreamChunks for that session on chunks_rx()
    And client B observes the same StreamChunks for that session on chunks_rx()
    And the server applies no per-client subscription filter in this card
