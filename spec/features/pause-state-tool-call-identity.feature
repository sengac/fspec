@BUG-582
@BUG-291
@session-management
@codelet
Feature: Pause state names the gated tool call
  """
  get_pause_state returns codelet_rpc_types::PauseState. Its third field used to be named tool_call_id but carried the
  pause details text (sessions/src/conversions.rs pause_state_to_rpc mapped ToolPauseState.details into it). The field
  is renamed details in place (same bincode position and type) and a real tool_call_id is appended. The id comes from a
  per-session active-tool-call registry in codelet_tools::tool_pause: the CLI stream loop records each streamed
  ToolCall's id there (the same id tool progress is attributed to) and clears it when that call's result arrives, and
  the agent loop's pause handler builds the state with PauseState::for_session, which reads it.
  """

  Background: User Story
    As a client that renders tool approval pauses
    I want the pause state to carry its details and the gated tool call's id separately
    So that I can match a pause to its ToolCall and still show the details text

  Scenario: The stream loop records the running tool call for the session
    Given a session whose turn is running a Bash tool call "rig016-call"
    When the tool is executing
    Then the session's active tool call is "rig016-call"
    And once the tool result arrives the session has no active tool call

  Scenario: A pause raised during a tool call carries that call's id
    Given a session whose active tool call is "call-7"
    When a tool requests a pause with details "rm -rf build"
    Then the pause state's tool_call_id is "call-7"
    And its details are "rm -rf build"

  Scenario: The RPC pause state keeps details and the tool call id apart
    Given an internal pause state with details "rm -rf build" and tool call id "call-7"
    When it is converted for get_pause_state
    Then the RPC pause state's details are "rm -rf build"
    And its tool_call_id is "call-7"
