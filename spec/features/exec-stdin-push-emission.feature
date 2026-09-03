@done
@BUG-171
@session
@tool-execution
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
  #   4. The exec-stdin detector must observe the end of the prompt condition and push a clear: once it has fired for an exec session, it must emit a clear to the agent-session callback within one detector tick (~2s) when (a) the child exits, (b) the session is removed from the store, or (c) the command produces output again (quiet < threshold). The clear flows through the existing set_exec_stdin_request(None) path and surfaces as an ExecStdinRequestCleared chunk. If the detector had not fired yet, no clear is emitted (nothing to clear); after a non-exit clear the detector keeps watching and may re-fire after the cooldown.
  #   5. A non-exit detector clear (command produced output again) resets the per-exec-session re-fire cooldown, so a fresh quiet period after the command resumed can fire again without waiting out the previous fire's 30s window. The P2 30s cooldown still applies within one continuous quiet period (e.g. right after a user submit that leaves the command silent).
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

  Scenario: A non-exit detector clear resets the per-exec-session re-fire cooldown
    Given a stored exec-stdin request on a Running agent session that fired the detector
    When the command produces output again and the detector clears the stored request
    Then the detector may re-fire after a fresh quiet period without waiting out the previous 30 second window
    And a second exec-stdin request StreamChunk with a newer fire timestamp is pushed on the session chunk stream

  Scenario: A continuous quiet period still obeys the 30 second cooldown
    Given a stored exec-stdin request on a Running agent session that fired the detector
    When the command stays quiet for at least 30 seconds without producing output or exiting
    Then the detector does not emit a second request before the 30 second cooldown elapses

  Scenario: A detector that never fired emits no clear
    Given an agent session with exec-stdin callbacks registered and a live exec session that exits before the quiet threshold
    When the detector ticks while the exec session is gone and the detector never fired
    Then no exec-stdin request or cleared StreamChunk is pushed on the session chunk stream
    And the agent session slot remains empty
