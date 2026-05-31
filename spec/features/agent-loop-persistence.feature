@wip
@deferred
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
  ToolResult, and persist_token_state on Done — same call sites as
  codelet/napi/src/agent_loop.rs:529, :1558, :1666-1690.

  The persistence helpers themselves were already lifted to
  codelet/agent-loop/src/persist.rs in RPC-072 Phase A. This card
  invokes them from the agent loop body.

  Originally scenario "A turn's messages are persisted and restorable
  across binary restart" from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want my Work Agent sessions to survive a binary restart with full scrollback intact
    So that /resume replays the same conversation I left, not an empty session

  Scenario: A turn's messages are persisted and restorable across binary restart
    Given a Work Agent session has completed one user/assistant/tool_call/tool_result turn
    When the fspec binary is killed and restarted
    And the same session is resumed via /resume
    Then session_restore_messages replays StreamChunk::UserInput, StreamChunk::Text, StreamChunk::ToolCall, and StreamChunk::ToolResult in original order
    And session.get_tokens() reflects the persisted token state
