@done
@BUG-376
@tools
@tool-execution
Feature: Exec stdin detector fault reporting
  """
  The exec-stdin quiet detector (spawn_exec_stdin_detector in rust/tools/src/unified_exec/exec_stdin.rs) records a
  per-exec-session last-fire time in the LAST_FIRE_MS RwLock to enforce the 30 second re-fire cooldown, and delivers
  fires and clears to the agent session's EXEC_STDIN_REQUEST_CALLBACKS entry. Before BUG-376 a poisoned cooldown lock
  read as "cooldown elapsed" and dropped every record and reset, and a detector whose agent session had no callback
  replaced it with a silent no-op closure. Now the poisoned guard is recovered with PoisonError::into_inner, the
  poison is cleared and the recovery is logged with tracing::error, and an undeliverable request is logged with
  tracing::error naming the agent session and the exec session.
  """

  Background: User Story
    As a developer running commands through the unified exec tool
    I want to have exec-stdin detector faults kept working or reported
    So that a broken cooldown or missing prompt callback never fails silently

  Scenario: The re-fire cooldown keeps working after its lock was poisoned
    Given a thread panicked while holding the exec-stdin cooldown lock
    When the detector records a fire for exec session "exec-poisoned"
    Then the detector does not fire again for "exec-poisoned" before the 30 second cooldown elapses

  Scenario: A request for a session without a callback is reported
    Given an agent session has no exec-stdin request callback
    When a command run through the Bash tool in that session stays quiet long enough to fire the detector
    Then an error naming the agent session and the exec session is logged
