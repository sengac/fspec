@done
@BUG-387
@tools
@tool-execution
@rust
Feature: Exec stdin detector clock and emit errors
  """
  rust/tools/src/unified_exec/exec_stdin.rs: now_ms() returned 0 for a system
  clock before the epoch (stamping ts_ms 0 and making every cooldown read as
  elapsed), and emit_exec_stdin_request silently did nothing when the agent
  session had no callback. Now the re-fire cooldown records
  std::time::Instant (monotonic, no error path); epoch_ms returns an error
  for a pre-epoch clock and the detector logs it and skips that fire; and
  emit_exec_stdin_request returns an error when no callback is registered.
  exec_stdin/tests.rs no longer allows unwrap_used / expect_used.
  Tests: rust/tools/src/unified_exec/exec_stdin/tests.rs.
  """

  Background: User Story
    As an agent session waiting on an exec-stdin prompt
    I want the detector's timestamps and deliveries to be real
    So that a broken clock or a missing callback is reported instead of silently misfiring

  Scenario: A request timestamp before the epoch is an error
    Given a system clock reading before the Unix epoch
    When the exec-stdin request timestamp is taken
    Then it is an error instead of 0

  Scenario: The re-fire cooldown runs on the monotonic clock
    Given a fire recorded on the monotonic clock
    When 29 seconds and then 30 seconds have passed
    Then the cooldown holds at 29 seconds and has elapsed at 30 seconds

  Scenario: Emitting to a session without a callback is an error
    Given an agent session with no exec-stdin request callback
    When an exec-stdin request is emitted for it
    Then the emit fails naming the agent session
