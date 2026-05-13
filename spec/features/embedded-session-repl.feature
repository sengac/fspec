@done
@streaming
@rust
@tarpc
@session-management
@rpc
@embedded
@RPC-007
Feature: Embedded session REPL push channel
  """
  EmbeddedTransport gains five new session RPCs (list_sessions, create_session,
  send_input, interrupt, get_session_status) and a sibling chunks_rx() method
  that returns the StreamChunk broadcast subscription DIRECTLY (no envelope
  encoding, zero-cost path). Backed by Arc<dyn SessionManagerHandle> from
  codelet/core (trait+handle pattern; concrete SessionManager remains in
  codelet/napi). Tests use the StubProvider behind the test-support feature
  emitting deterministic [Text("hi back"), Done].
  """

  Background: User Story
    As a developer integrating the future ratatui agent REPL
    I want to drive a session over the embedded transport — list, create, send input, observe streaming chunks, query status, and interrupt
    So that the basic ratatui REPL in RPC-009 can run against an in-process backend without depending on NAPI or WebSockets

  @rpc
  @embedded
  @session
  Scenario: Embedded list_sessions returns the same Vec<SessionInfo> as the underlying SessionManager
    Given a host has constructed an EmbeddedTransport from a tokio Handle around a SessionManagerHandle backed by the existing SessionManager singleton
    And the SessionManager already holds at least one active session known to NAPI list_sessions
    When a Rust caller invokes FspecServiceClient::list_sessions(context::current()) over the embedded transport
    Then the call returns Ok(Vec<SessionInfo>) with the same length and SessionId values that NAPI list_sessions would return
    And the call does not encode any Envelope frames

  @rpc
  @embedded
  @streaming
  Scenario: Embedded create_session + send_input emits at least one StreamChunk on chunks_rx within 5s
    Given an EmbeddedTransport with a SessionManagerHandle wired to the StubProvider behind the test-support feature
    And the caller has subscribed to EmbeddedTransport::chunks_rx() before sending input
    When the caller calls create_session(role: None) and then send_input(session_id, "hi")
    Then send_input returns Ok(()) immediately without holding a tarpc stream
    And within 5 seconds the chunks_rx receiver yields at least one (SessionId, StreamChunk::Text { .. }) tuple matching the active session

  @rpc
  @session
  @interrupt
  Scenario: interrupt(session_id) flips state and emits StreamChunk::Interrupted on chunks_rx
    Given a session is actively streaming a stub-provider response on either transport
    And the caller is subscribed to chunks_rx() for that session
    When the caller calls interrupt(session_id)
    Then the RPC returns Ok(()) immediately
    And the chunks_rx receiver yields a StreamChunk::Interrupted (or equivalent) for that session
    And a subsequent get_session_status(session_id) reports the session as interrupted

  @rpc
  @session
  @status
  Scenario: get_session_status reflects Idle to Running to Idle transitions equally on both transports
    Given a session is created via create_session on either transport with the StubProvider
    When the caller calls get_session_status(session_id) before any send_input
    Then the returned SessionStatus is Idle
    When the caller calls send_input(session_id, "hi") and immediately calls get_session_status(session_id)
    Then the returned SessionStatus is Running
    When the stub provider has emitted StreamChunk::Done and the caller calls get_session_status(session_id) again
    Then the returned SessionStatus is Idle
    And the same sequence holds when the parity scenario is run on the other transport
