@done
@BUG-171
@REFAC-060
@session
@tool-execution
Feature: Exec-stdin detector pushes a clear when the prompt condition ends
  """
  BUG-171 (tools layer, split from exec-stdin-push-emission.feature by REFAC-060): once the exec-stdin detector has fired for an exec session it emits a clear through set_exec_stdin_request(None) when the command produces output again, the child exits, or the exec session leaves the store; the clear surfaces as an ExecStdinRequestCleared chunk. Tests: rust/sessions/tests/exec_stdin_detector_clear_bug171.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT (BUG-171)
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The exec-stdin detector must observe the end of the prompt condition and push a clear: once it has fired for an exec session, it must emit a clear to the agent-session callback within one detector tick (~2s) when (a) the child exits, (b) the session is removed from the store, or (c) the command produces output again (quiet < threshold). The clear flows through the existing set_exec_stdin_request(None) path and surfaces as an ExecStdinRequestCleared chunk. If the detector had not fired yet, no clear is emitted (nothing to clear); after a non-exit clear the detector keeps watching and may re-fire after the cooldown.
  #
  # ========================================
  Background: User Story
    As a user of the Rust TUI agent
    I want to have the exec-stdin composer overlay appear automatically when a running command goes quiet waiting for input
    So that I can type into the running command's stdin without switching sessions or waiting for a status change

  Scenario: Detector clear on output resumption pushes an exec-stdin cleared chunk
    Given a stored exec-stdin request on a Running agent session for a live exec session
    When the command produces output again so the session is no longer quiet
    Then the detector emits a clear to the agent-session callback within one detector tick
    And an exec-stdin cleared StreamChunk is pushed on the session chunk stream
    And the agent session status remains running

  Scenario: Detector clear on child exit emits a clear and the stored request is gone
    Given a stored exec-stdin request on a Running agent session for a live exec session
    When the child exits and the reaper removes the exec session from the store
    Then the detector emits a clear to the agent-session callback within one detector tick
    And an exec-stdin cleared StreamChunk is pushed on the session chunk stream
    And the stored request is cleared and the agent session status remains running

  Scenario: Detector clear on session removal from the store
    Given a stored exec-stdin request on a Running agent session for a live exec session
    When the exec session is removed from the store while the agent session stays Running
    Then the detector emits a clear to the agent-session callback within one detector tick
    And an exec-stdin cleared StreamChunk is pushed on the session chunk stream
