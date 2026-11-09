@done
@BUG-171
@REFAC-060
@session
@tool-execution
Feature: Exec-stdin detector re-fire cooldown
  """
  BUG-171 (tools layer, split from exec-stdin-push-emission.feature by REFAC-060): a non-exit detector clear resets the per-exec-session re-fire cooldown, the 30 second cooldown still holds within one continuous quiet period, and a detector that never fired emits no clear. Tests: rust/sessions/tests/exec_stdin_detector_cooldown_bug171.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT (BUG-171)
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A non-exit detector clear (command produced output again) resets the per-exec-session re-fire cooldown, so a fresh quiet period after the command resumed can fire again without waiting out the previous fire's 30s window. The P2 30s cooldown still applies within one continuous quiet period (e.g. right after a user submit that leaves the command silent).
  #   2. If the detector had not fired for an exec session, no clear is emitted (nothing to clear).
  #
  # ========================================
  Background: User Story
    As a user of the Rust TUI agent
    I want to have the exec-stdin composer overlay appear automatically when a running command goes quiet waiting for input
    So that I can type into the running command's stdin without switching sessions or waiting for a status change

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
