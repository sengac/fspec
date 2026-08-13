@done
@RPC-081
@rpc
@rust
@agent-loop
@session-management
@history
@wip
Feature: Agent loop: conversation history (session.inner.messages round-trip + session_restore_messages parity)
  """
  Implementation home for restoration: port the body of rust/napi/src/session_bindings.rs:2401-2567 (session_restore_messages) into rust/sessions/src/handle_impl.rs::restore_session_messages, replacing the current 5-line stub. The handler walks each envelope's message.content blocks, builds parallel rig::message::Message + StreamChunk vectors, then pushes rig messages into session.inner.lock().await.messages and dispatches StreamChunks via session.handle_output. Skip-rule for system-reminder envelopes (joined text contains both '<system-reminder>' and '<!-- type:') is preserved verbatim from the NAPI source.

  Test strategy: restoration tests live in rust/sessions/tests/rpc081_restore_session_messages.rs and exercise the SessionManagerHandle impl directly with hand-crafted envelope JSON strings (no NAPI dependency). Agent-loop history threading is pinned with source-shape regression tests in rust/agent-loop/tests/rpc081_inner_messages_threading.rs that grep-assert the agent_loop body source contains zero literal 'vec![Message { role: MessageRole::User' single-element constructions and DOES contain the canonical '&mut inner_session' threading into run_agent_stream_with_images.

  Call-site contract: agent loop body already passes &mut inner_session into run_with_provider! and into run_agent_stream_with_images (rust/agent-loop/src/agent_loop.rs:867-1019 + rust/agent-loop/src/dispatch.rs:38-105). The new RPC-081 coverage pins this with structural tests + the behavioural restoration round-trip. For restoration, the canonical NAPI source is rust/napi/src/session_bindings.rs:2401-2567; the NAPI-free target lives in the existing trait impl at rust/sessions/src/handle_impl.rs:274 which currently returns Ok(()) without touching messages.
  """

  Background: User Story
    As a fspec user
    I want to have each follow-up prompt remember the prior turns in the same session
    So that conversational context survives across turns just like the TypeScript Ink frontend

  Scenario: restore_session_messages replays a one-user-one-assistant transcript into inner.messages and the output stream
    Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    And a user MessageEnvelope JSON whose content is [{"type":"text","text":"hello"}]
    And an assistant MessageEnvelope JSON whose content is [{"type":"text","text":"hi back"}]
    When restore_session_messages is invoked with those two envelopes via SessionManagerHandle
    Then session.inner.lock().await.messages.len() equals 2
    And the first inner message is a rig::message::Message::User whose joined text equals "hello"
    And the second inner message is a rig::message::Message::Assistant whose joined text equals "hi back"
    And the broadcasted StreamChunks for that session are, in order, UserInput("hello"), Text("hi back"), Done

  Scenario: Assistant restoration replays thinking, text, and tool_use blocks then a terminating Done
    Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    And an assistant MessageEnvelope JSON whose content is [{"type":"thinking","thinking":"hmm"},{"type":"text","text":"reading"},{"type":"tool_use","id":"t1","name":"Read","input":{"path":"/tmp/x"}}]
    When restore_session_messages is invoked with that envelope via SessionManagerHandle
    Then the broadcasted StreamChunks for that session are, in order, Thinking("hmm"), Text("reading"), ToolCall with id "t1" and name "Read" and input '{"path":"/tmp/x"}', and Done
    And session.inner.lock().await.messages contains exactly one rig::message::Message::Assistant whose joined text equals "reading"

  Scenario: User restoration replays tool_result blocks as StreamChunk::ToolResult and does not append to inner messages
    Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    And a user MessageEnvelope JSON whose content is [{"type":"tool_result","tool_use_id":"t1","content":"contents","is_error":false}]
    When restore_session_messages is invoked with that envelope via SessionManagerHandle
    Then the broadcasted StreamChunks for that session contain a ToolResult with tool_call_id "t1" and content "contents" and is_error false
    And session.inner.lock().await.messages.len() equals 0

  Scenario: System-reminder envelopes are silently skipped during restoration
    Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    And a user MessageEnvelope JSON whose content is [{"type":"text","text":"<system-reminder>\n<!-- type:fspecWorkflow -->\nstale\n</system-reminder>"}]
    When restore_session_messages is invoked with that envelope via SessionManagerHandle
    Then session.inner.lock().await.messages.len() equals 0
    And no StreamChunk is broadcasted to the session's output for that envelope

  Scenario: restore_session_messages returns Err on an unknown session id without panicking
    Given a SessionManagerHandle whose underlying SessionManager has no session registered under the id "00000000-0000-0000-0000-000000000000"
    When restore_session_messages is invoked with that id and an empty envelope vector
    Then the call returns Err whose message contains "Session not found"
    And the process does not panic

  Scenario: restore_session_messages returns Err on malformed envelope JSON without mutating inner.messages
    Given a SessionManager has created a fresh BackgroundSession with messages.len() == 0
    When restore_session_messages is invoked with the single envelope string "{ not json"
    Then the call returns Err whose message contains "Failed to parse envelope"
    And session.inner.lock().await.messages.len() still equals 0

  Scenario: Boundary — codelet-sessions still has zero dependency on codelet-napi after the restoration port
    Given the restore_session_messages port has landed in rust/sessions/src/handle_impl.rs
    When cargo metadata is invoked for the codelet-sessions package
    Then the resulting transitive package set does not contain "codelet-napi"
    And no .rs file under rust/sessions/src/ contains the substring "codelet_napi"
