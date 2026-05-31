@wip
@deferred
@session-management
@RPC-084
@rust
@agent-loop
@rpc
@streaming
Feature: Agent loop streams replies as multiple StreamChunk::Text deltas
  """
  RPC-084 (child of RPC-072 family). Streaming must go through
  codelet_cli::interactive::run_agent_stream_with_images — the same rig
  multi-turn streaming engine the NAPI loop uses. Non-streaming
  complete_with_tools is forbidden as the primary path.

  All 19+ StreamChunk variants from codelet/napi/src/agent_loop.rs:1310-1700
  must be emitted as in the original.

  Originally scenario "Replies stream as multiple StreamChunk::Text deltas
  before Done" from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want assistant replies to appear as a flowing stream of deltas
    So that I see text as it generates, matching the TS Ink frontend experience

  Scenario: Replies stream as multiple StreamChunk::Text deltas before Done
    Given a Work Agent session backed by a streaming-capable stub provider
    And the stub is configured to emit three text deltas followed by Done
    When the user sends "tell me a haiku"
    Then the scrollback receives at least two StreamChunk::Text chunks before StreamChunk::Done
    And run_agent_stream_with_images was the dispatch path observed by the stub
    And complete_with_tools was NOT used as the primary dispatch path
