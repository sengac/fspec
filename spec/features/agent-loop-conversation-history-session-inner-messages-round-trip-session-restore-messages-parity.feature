@BUG-474
@done
@RPC-081
@rpc
@rust
@agent-loop
@session-management
@history
@wip
@BUG-432
@BUG-308
Feature: Agent loop: conversation history (session.inner.messages round-trip + session_restore_messages parity)
  """
  Implementation home for restoration: port the body of rust/napi/src/session_bindings.rs:2401-2567 (session_restore_messages) into rust/sessions/src/handle_impl.rs::restore_session_messages, replacing the current 5-line stub. The handler walks each envelope's message.content blocks, builds parallel rig::message::Message + StreamChunk vectors, then pushes rig messages into session.inner.lock().await.messages and dispatches StreamChunks via session.handle_output. Envelopes marked _runtimeReminder {version:1, replaceable:true} are skipped; other user messages, including tool results, are installed so tool pairs survive. Since 2699693b the client-visible transcript is paged from the persisted archival transcript rather than built from the envelopes (BUG-308).

  Test strategy: restoration tests live in rust/sessions/tests/rpc081_restore_session_messages.rs and exercise the SessionManagerHandle impl directly with hand-crafted envelope JSON strings (no NAPI dependency). Agent-loop history threading is pinned with source-shape regression tests in rust/agent-loop/tests/rpc081_inner_messages_threading.rs that grep-assert the agent_loop body source contains zero literal 'vec![Message { role: MessageRole::User' single-element constructions and DOES contain the canonical '&mut inner_session' threading into run_agent_stream_with_images.

  Call-site contract: agent loop body passes &mut inner_session to providers::run_selected_provider (rust/agent-loop/src/agent_loop.rs), which forwards it into run_with_provider! (agent_loop/providers.rs, macro in dispatch.rs) and into the direct run_agent_stream_with_images calls (agent_loop/provider_openai.rs, provider_custom.rs). The new RPC-081 coverage pins this with structural tests + the behavioural restoration round-trip. For restoration, the canonical NAPI source is rust/napi/src/session_bindings.rs:2401-2567; the NAPI-free target lives in the existing trait impl at rust/sessions/src/handle_impl.rs:274 which currently returns Ok(()) without touching messages.
  """

  Background: User Story
    As a fspec user
    I want to have each follow-up prompt remember the prior turns in the same session
    So that conversational context survives across turns just like the TypeScript Ink frontend

  Scenario: restore_session_messages installs a one-user-one-assistant transcript into inner.messages
    Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    And a user MessageEnvelope JSON whose content is [{"type":"text","text":"hello"}]
    And an assistant MessageEnvelope JSON whose content is [{"type":"text","text":"hi back"}]
    When restore_session_messages is invoked with those two envelopes via SessionManagerHandle
    Then session.inner.lock().await.messages.len() equals 2
    And the first inner message is a rig::message::Message::User whose joined text equals "hello"
    And the second inner message is a rig::message::Message::Assistant whose joined text equals "hi back"
    And no UserInput or Text StreamChunk is built from the envelopes, because the visible transcript is paged from the persisted archival transcript

  Scenario: Assistant restoration installs thinking, text, and tool_use blocks as one assistant message
    Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    And an assistant MessageEnvelope JSON whose content is [{"type":"thinking","thinking":"hmm"},{"type":"text","text":"reading"},{"type":"tool_use","id":"t1","name":"Read","input":{"path":"/tmp/x"}}]
    When restore_session_messages is invoked with that envelope via SessionManagerHandle
    Then session.inner.lock().await.messages contains exactly one rig::message::Message::Assistant
    And its content is, in order, reasoning "hmm", text "reading", and a tool call with id "t1" and name "Read" and arguments {"path":"/tmp/x"}

  Scenario: User restoration installs tool_result blocks as a user tool-result message
    Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    And a user MessageEnvelope JSON whose content is [{"type":"tool_result","tool_use_id":"t1","content":"contents","is_error":false}]
    When restore_session_messages is invoked with that envelope via SessionManagerHandle
    Then session.inner.lock().await.messages contains exactly one rig::message::Message::User
    And its only content is a tool result for "t1" whose text is "contents"

  Scenario: Replaceable runtime-reminder envelopes are silently skipped during restoration
    Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    And a user MessageEnvelope JSON marked "_runtimeReminder" {"version":1,"replaceable":true} whose content is a stale system-reminder
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
    Then the call returns Err whose message contains "invalid replay JSON"
    And session.inner.lock().await.messages.len() still equals 0

  Scenario: Boundary — codelet-sessions still has zero dependency on codelet-napi after the restoration port
    Given the restore_session_messages port has landed in rust/sessions/src/handle_impl.rs
    When cargo metadata is invoked for the codelet-sessions package
    Then the resulting transitive package set does not contain "codelet-napi"
    And no .rs file under rust/sessions/src/ contains the substring "codelet_napi"
