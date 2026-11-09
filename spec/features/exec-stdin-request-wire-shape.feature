@tool-execution
@done
@session
@rpc
@rust
@TOOL-022
@BUG-415
Feature: Exec-stdin request wire shape
  """
  The TUI probe path deserializes codelet_rpc_types::ExecStdinRequest from JSON
  over the WebSocket transport (TOOL-022 P2). The round-trip test lives in
  rust/sessions/tests/exec_stdin_prompt_p2.rs; BUG-415 gave it this scenario
  (it mapped to none) and made the shared close_exec test helpers fail on a
  failed close instead of discarding the result.
  """

  Background: User Story
    As a TUI user answering a command that waits for input
    I want the exec-stdin request to arrive intact over the WebSocket transport
    So that the prompt shows the right exec session, command and timing

  Scenario: The wire exec-stdin request round-trips through JSON
    Given a wire ExecStdinRequest { exec_session_id, command, quiet_seconds, ts_ms }
    When the value is serialized to JSON and deserialized back
    Then the deserialized value equals the original
