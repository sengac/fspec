@done
@BUG-368
@tools
@tool-execution
@rust
Feature: Unified exec reports the real exit status
  """
  rust/tools/src/unified_exec reported guessed values: an exit without a
  code (signal) became -1, a reaped session without a stashed status became
  -1, a missing quiet time became 0, and PTY sessions reported the liveness
  anchor's kill status because spawning/pty.rs dropped pty_child.wait()'s
  result. ExecExit (unified_exec/exec_exit.rs) now carries the real end
  state: the PTY wait task records the PTY child's status before killing the
  anchor, ProcessStore::try_wait and recover_exit return ExecExit, and
  UnifiedExecResult reports exit_code, signal or exit_status_unavailable.
  Tests: rust/tools/tests/bug368_unified_exec_real_exit_status.rs.
  """

  Background: User Story
    As an agent running commands through unified exec
    I want to know exactly how a command ended
    So that a killed or PTY command is never reported with a made-up exit code

  Scenario: A command killed by a signal reports the signal
    Given a unified exec command that kills itself with SIGKILL
    When the command finishes
    Then the result reports a signal and no exit code

  Scenario: A PTY command reports its own exit code
    Given a unified exec PTY command that exits with code 3
    When the command finishes
    Then the result reports exit code 3

  Scenario: An unavailable exit status is reported as unavailable
    Given an exec exit whose status could not be recovered
    When it is applied to a unified exec result
    Then the result has no exit code and no signal and says the exit status is unavailable
