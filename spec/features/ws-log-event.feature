@done
@BUG-572
@logging
@rust
@tarpc
@rpc
@websocket
@wire-format
@RPC-007
@infrastructure
Feature: WebSocket LogEvent and Event bincode wire format
  """
  Both Envelope::Event { session_id: SessionId, chunk: StreamChunk } and
  Envelope::LogEvent(LogRecord) ride the SAME bincode-of-Envelope wire
  format that RPC-005 established for Envelope::Rpc and RPC-006 used for
  Envelope::WorkUnitsUpdate. No JSON debug envelope, no separate channel,
  no custom shape. The server-side tracing::Layer captures level/target/
  message/timestamp into a LogRecord and the per-connection logs_fanout
  task emits Envelope::LogEvent(record) frames; chunks_fanout emits
  Envelope::Event { session_id, chunk } frames.
  BUG-572: forwarding a LogEvent frame must not add records to the log
  broadcast (the transport's own below-warning logging is not broadcast),
  so the fan-out cannot feed on itself and flood the client.
  """

  Background: User Story
    As a developer reasoning about the wire protocol
    I want Event and LogEvent to ride the same bincode-encoded Envelope as Rpc and WorkUnitsUpdate
    So that the wire format remains uniform and a captured frame round-trips through bincode without ambiguity

  @rpc
  @websocket
  @wire-format
  Scenario: Event and LogEvent ride bincode-encoded Envelope on the WebSocket wire
    Given a WebSocket client is connected and subscribed to chunks_rx() and logs_rx()
    When a session emits a StreamChunk and the host emits a tracing event
    Then a synthesized Envelope::Event { session_id, chunk } round-trips via bincode without ambiguity
    And a synthesized Envelope::LogEvent(LogRecord) round-trips via bincode without ambiguity
    And neither bincode-encoded frame decodes as JSON nor uses any custom shape outside Envelope

  @rpc
  @websocket
  Scenario: Forwarding log records over the WebSocket does not feed the log broadcast
    Given a WebSocket client is connected and subscribed to logs_rx()
    When the host emits a "hello" record and, once the client has received it, a "marker" record
    Then no record the client receives before "marker" has a tungstenite, tokio_tungstenite or tarpc target below WARN
