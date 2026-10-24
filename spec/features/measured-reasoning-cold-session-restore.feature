@done
@CMPCT-062
@compaction
@context-management
@agent-core
@phase-1
@bug
Feature: Measured reasoning survives compacted session restoration
  """
  Real physical request settlement owns cumulative reasoning and billing.
  SessionManager restores that authoritative state after compaction in a fresh
  process. Repeating restoration must not bill a settled request again.
  """

  Scenario: Measured reasoning remains cumulative after compacted cold session restore
    Given two completed physical requests with measured output and reasoning
    When the session compacts and a fresh process restores its saved generation
    Then live cached and emitted reasoning equal the cumulative measured reasoning
    And repeating restoration preserves billing and physical settlement identities
