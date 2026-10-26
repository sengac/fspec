@done
@agent-core
@session-management
@persistence
@performance
@testing
@BUG-270
Feature: Avoid redundant recovery staging for healthy durable streams
  """
  BackgroundOutput retains durable-before-publication semantics. A healthy journal
  append can be the durable admission itself, avoiding a second recovery-file
  transaction. The existing recovery lease excludes deletion and serializes the
  session lane. Older recovery work must drain before new publication. Failed
  journal admission falls back to bounded recovery with the exact original ID.
  This reduces duplicate durability work, not synchronous journal latency itself.
  """

  Scenario: Healthy stream admission does not require a writable recovery directory
    Given a live session with an empty recovery lane that cannot hold record files
    When text and thinking deltas are emitted
    Then both deltas are published after their journal durability barriers
    And cold recovery contains both deltas exactly once without recovery files

  Scenario: Failed direct admission retains exact identity for recovery
    Given a live session whose journal directory durability barrier fails
    When a text delta is emitted
    Then the delta is not published and its exact record is retained for recovery
    When storage recovers and persistence is retried twice
    Then cold recovery contains the original identity exactly once

  Scenario: Existing recovery work is not bypassed by direct admission
    Given an older pending transcript event and a later stream delta
    When the later delta is emitted
    Then cold recovery contains the older event before the later delta
    And the later delta is published only after recovery drains

  Scenario: Deleted sessions cannot publish through direct admission
    Given a stream whose persisted session has been deleted
    When text and thinking deltas are emitted
    Then neither delta is published and no transcript is recreated

  Scenario: Tool dispatch still requires durable journal admission
    Given a tool invocation published through healthy stream admission
    When its dispatch journal barrier fails
    Then tool dispatch is refused
    When storage recovers and persistence is retried twice
    Then the invocation and dispatch identities each occur exactly once

  Scenario: Healthy admission leaves recovery lane permissions unchanged
    Given a live session with an empty read-only recovery lane
    When a healthy text delta is durably published
    Then the recovery lane remains read-only and empty
