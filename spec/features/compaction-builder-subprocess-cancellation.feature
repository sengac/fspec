@BUG-214
@integration
@compaction
@context-management
@phase-1
Feature: Cancelling compaction stops subprocesses owned by its builder
  """
  Architecture notes:
  The compaction builder has an ephemeral tool identity distinct from its
  original session. Cancelling the original operation must close subprocess
  sessions created by that builder as well as dropping inference futures.
  Exercise the real builder and a local deterministic provider that requests
  a long-running command. Observe actual process termination, not only an
  interruption flag or dropped future. Keep another owner's process alive
  to verify cleanup is scoped. Test teardown may close remaining subprocesses
  after observing failure but cannot count that cleanup as cancellation success.
  """

  Scenario: Original-session cancellation terminates only its compaction builder subprocesses
    Given automatic compaction has a builder running a long-lived subprocess under an ephemeral identity
    And an unrelated owner has a running subprocess
    When the original session is interrupted while its builder is pending
    Then the compaction operation is cancelled and its builder subprocess terminates promptly
    And the unrelated owner's subprocess remains running
    And no compaction success or continuation inference occurs
