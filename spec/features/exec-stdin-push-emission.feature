@BUG-171
@session
@tool-execution
@wip
Feature: Exec-stdin slot transitions push StreamChunks without a status flip

  """
  BUG-171 (sessions layer): BackgroundSession::set_exec_stdin_request emits a push StreamChunk on every slot transition — ExecStdinRequest when a request is stored, ExecStdinRequestCleared when the slot goes to None (child-exit alive-check, successful write_exec_stdin, explicit clear). Mirrors the set_status → session_state_change flow; NO agent session status flip (the session stays Running).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT (BUG-171)
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A stored exec-stdin request must surface in the TUI composer slot via a push StreamChunk emitted from BackgroundSession::set_exec_stdin_request, without any agent session status flip (session stays Running)
  #   2. The cleared chunk must be emitted whenever the exec-stdin slot transitions from Some to None: exec session child exits (alive-check), write_exec_stdin succeeds, or the exec session is removed from the store
  #
  # ========================================

  Background: User Story
    As a user of the Rust TUI agent
    I want to have the exec-stdin composer overlay appear automatically when a running command goes quiet waiting for input
    So that I can type into the running command's stdin without switching sessions or waiting for a status change

  Scenario: Detector fire while session stays Running pushes an exec-stdin request chunk
    Given the agent session is Running and focused with no HITL prompt in the slot
    When the exec-stdin quiet detector fires for a live exec session and the agent session callback stores the request on the BackgroundSession
    Then an exec-stdin request StreamChunk with that request is pushed on the session chunk stream
    And the agent session status remains running

  Scenario: Clearing the exec-stdin slot pushes an exec-stdin cleared chunk
    Given a stored exec-stdin request on a Running agent session
    When the exec-stdin slot transitions from Some to None
    Then an exec-stdin cleared StreamChunk is pushed on the session chunk stream

  Scenario: Exec session child exit clears the stored request without a status flip
    Given a stored exec-stdin request on a Running agent session
    When the underlying exec session child exits and the alive check runs
    Then the stored request is cleared
    And the agent session status remains running

  Scenario: Successful write_exec_stdin pushes a cleared chunk so the overlay unmounts
    Given the exec-stdin composer overlay is visible for a live exec session
    When the user presses Enter and the backend write to the exec session stdin succeeds
    Then an exec-stdin cleared StreamChunk is pushed
    And the overlay is gone on the next frame
