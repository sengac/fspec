@done
@napi
@parity
@streaming
@rpc
@RPC-007
Feature: NAPI session co-listener parity after StreamChunk lift
  """
  After lifting StreamChunk into codelet/rpc-types and introducing
  SessionManagerHandle in codelet/core, codelet/napi becomes ONE listener
  on the same broadcast::Sender housed in SharedFspecService — not the
  only listener. The existing GLOBAL_CHUNK_CALLBACK + GlobalChunkCallback
  pattern at codelet/napi/src/session_manager.rs:55-87 is preserved: it
  reads from the SAME broadcast and forwards each (session_id, chunk)
  tuple into the existing ThreadsafeFunction so the TS shape stays
  byte-equal. A Vitest smoke test confirms the global chunk callback
  continues to fire with the unchanged shape.
  """

  Background: User Story
    As a TS frontend developer
    I want sessionSetGlobalChunkCallback to keep firing for new sessions with the unchanged StreamChunk shape after the Rust lift
    So that the existing TS REPL and the new ratatui REPL co-listen on the same SessionManager singleton without interfering with each other

  @rpc
  @napi
  @parity
  Scenario: NAPI sessionSetGlobalChunkCallback continues to fire alongside a Rust embedded subscriber on the same SessionManager
    Given the SessionManager singleton is shared by a NAPI host and an EmbeddedTransport via the same SessionManagerHandle
    And the TS frontend has registered a callback via sessionSetGlobalChunkCallback
    And a Rust embedded caller has subscribed to EmbeddedTransport::chunks_rx()
    When a session is created via sessionManagerCreate and input is sent via sessionSendInput
    Then the TS callback registered by sessionSetGlobalChunkCallback fires for each StreamChunk with the existing TS shape unchanged
    And the Rust embedded subscriber observes byte-equal StreamChunks on chunks_rx()
