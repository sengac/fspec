@done
@logging
@rust
@tarpc
@rpc
@embedded
@RPC-007
Feature: Embedded LogEvent push channel via tracing Layer
  """
  EmbeddedTransport hosts may opt in to a custom tracing_subscriber::Layer
  registered at EmbeddedTransport::with_log_layer that captures level, target, message,
  timestamp into a LogRecord and pushes onto SharedFspecService::logs_tx.
  Embedded callers subscribe via EmbeddedTransport::logs_rx() to observe the
  same LogRecord stream that the WebSocket transport encodes as
  Envelope::LogEvent(LogRecord) frames. NAPI's existing setRustLogCallback
  path is preserved unchanged.
  """

  Background: User Story
    As a developer running the embedded backend in-process
    I want to observe tracing emissions as structured LogRecord events on logs_rx()
    So that the ratatui REPL can render log output the same way it would receive Envelope::LogEvent frames over the WebSocket transport

  @rpc
  @logging
  @embedded
  Scenario: Tracing emissions are observable as LogEvent on both transports
    Given codelet-rpc-server has registered the LogRecord tracing::Layer at startup
    And an EmbeddedTransport host has registered the same Layer at EmbeddedTransport::with_log_layer
    And a WebSocket client is connected and subscribed to FspecWsClient::logs_rx()
    And an embedded caller is subscribed to EmbeddedTransport::logs_rx()
    When the host emits tracing::info!("hello")
    Then the WebSocket client receives an Envelope::LogEvent(LogRecord) frame with message "hello" and level INFO
    And the embedded caller receives a LogRecord on logs_rx() with the same message and level
