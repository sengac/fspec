@done
@session-management
@RPC-080
@rust
@agent-loop
@rpc
@persistence
@refac007
Feature: Agent loop persists user, assistant, tool_result, and token state per turn
  """
  RPC-080 (child of RPC-072 family). REFAC-007 parity: every turn must
  flow through persist_user_message BEFORE the LLM call,
  persist_assistant_message on Done, persist_tool_result on each
  ToolResult, persist_token_state on Done, and persist_assistant_message
  on Error/Interrupted — same call sites as the canonical NAPI agent loop
  at rust/napi/src/agent_loop.rs:529 (user), 1436-1446 (tool result),
  1532 (error), 1537 (interrupted), 1542-1548 (done + token state).

  The persistence helpers themselves live in rust/agent-loop/src/persist.rs
  (lifted by RPC-072 Phase A as a verbatim NAPI-free copy). This card
  proves they are invoked from the agent loop body and from
  BackgroundOutput at the canonical call sites, that the on-disk envelope
  shape matches the NAPI canonical format, and that persistence failures
  log via tracing without aborting the stream.

  Source-shape scenarios pin the call-site contract; behavioural
  scenarios drive the helpers directly against a hermetic temp data
  directory and read back the persisted manifest. The full end-to-end
  resume-replay scenario is covered by the existing
  rpc072-work-agent-roundtrip family and is not duplicated here.
  """

  Background: User Story
    """
    As a fspec user
    I want every turn's user/assistant/tool-result/token state persisted to disk
    at the canonical NAPI call sites
    So that /resume after a binary restart replays the exact scrollback I left,
    not an empty session
    """

  Scenario: persist_user_message writes a User MessageEnvelope before the LLM stream begins
    Given a hermetic session manifest exists on disk for session id S
    When persist_user_message is invoked with session id S and text "hello"
    Then the manifest gains exactly one MessageEnvelope with message_type "user"
    And the envelope's payload is a UserMessage with content [Text("hello")]
    And the envelope's provider is the literal string "user"

  Scenario: persist_assistant_message_internal writes an Assistant envelope with provider and stop_reason
    Given a hermetic session manifest exists on disk for session id S
    When persist_assistant_message_internal is invoked with provider "stub" and content [Text("hi back")] and stop_reason Some("end_turn")
    Then the manifest gains exactly one MessageEnvelope with message_type "assistant"
    And the envelope's provider equals "stub"
    And the envelope's stop_reason equals "end_turn"

  Scenario: PROV-039 — stop_reason=None becomes the literal "unknown" on disk
    Given a hermetic session manifest exists on disk for session id S
    When persist_assistant_message_internal is invoked with stop_reason None
    Then the on-disk Assistant envelope's stop_reason equals "unknown"
    And the stop_reason is NOT the legacy sentinel "end_turn"

  Scenario: persist_tool_result_internal writes a User envelope tagged with provider "tool"
    Given a hermetic session manifest exists on disk for session id S
    When persist_tool_result_internal is invoked with tool_call_id "call_abc", content "contents", is_error false
    Then the manifest gains exactly one MessageEnvelope with message_type "user"
    And the envelope's provider equals the literal string "tool"
    And the envelope's payload is a UserMessage with a ToolResult content whose tool_use_id is "call_abc"
    And the ToolResult content equals "contents" and is_error is false

  Scenario: persist_token_state updates the session manifest's cumulative token totals
    Given a hermetic session manifest exists on disk for session id S
    When persist_token_state is invoked with input_tokens 100, output_tokens 50
    Then the manifest's persisted token state shows input_tokens 100 and output_tokens 50

  Scenario: persist_user_message returns Err on a missing manifest without panicking
    Given no manifest exists on disk for an arbitrary session id S
    When persist_user_message is invoked with session id S and text "hello"
    Then persist_user_message returns Err(String) referencing the load failure
    And no thread panics

  Scenario: Source-shape — agent_loop body invokes persist_user_message before dispatching to the provider
    Given the source of rust/agent-loop/src/agent_loop.rs
    When the file is scanned
    Then it imports persist_user_message from crate::persist
    And the function body contains a call to persist_user_message(&session.id, input)
    And that call is followed by the provider dispatch (no provider dispatch precedes it within the same turn block)

  Scenario: Source-shape — BackgroundOutput's StreamEvent::ToolResult arm persists assistant before tool_result
    Given the source of rust/agent-loop/src/background_output.rs
    When the file is scanned
    Then the StreamEvent::ToolResult arm calls self.persist_assistant_message()
    And the same arm subsequently calls persist_tool_result_internal(...)
    And the assistant-flush call precedes the tool-result persist call textually within that arm

  Scenario: Source-shape — BackgroundOutput's StreamEvent::Done arm persists assistant then token state
    Given the source of rust/agent-loop/src/background_output.rs
    When the file is scanned
    Then the StreamEvent::Done arm calls self.persist_assistant_message_with_stop_reason(stop_reason)
    And the same arm subsequently calls persist_token_state(&self.session.id, input_tokens, output_tokens)

  Scenario: Source-shape — BackgroundOutput's StreamEvent::Error arm flushes accumulated assistant content
    Given the source of rust/agent-loop/src/background_output.rs
    When the file is scanned
    Then the StreamEvent::Error arm calls self.persist_assistant_message()

  Scenario: Source-shape — BackgroundOutput's StreamEvent::Interrupted arm flushes accumulated assistant content
    Given the source of rust/agent-loop/src/background_output.rs
    When the file is scanned
    Then the StreamEvent::Interrupted arm calls self.persist_assistant_message()

  Scenario: Boundary — persistence calls in codelet-agent-loop import from crate::persist (not codelet_napi)
    Given the codelet-agent-loop crate
    When its source tree is scanned
    Then no .rs file under rust/agent-loop/src/ references codelet_napi::persist
    And persist.rs lives at rust/agent-loop/src/persist.rs
    And it exports persist_user_message, persist_assistant_message_internal, persist_tool_result_internal, and persist_token_state as pub
