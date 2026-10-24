@done
@CMPCT-070
@compaction
@agent-core
@phase-1
@validation
Feature: DAG promotion revalidation
  """
  Promotion revalidates the complete candidate against the persisted source
  inside writer ownership; earlier validation cannot authorize later edits.
  """

  Scenario: Revalidate whole candidates inside durable promotion
    Given a committed DAG and candidates changed after earlier validation
    When promotion checks the authoritative persisted source
    Then malformed incomplete and rewritten durable candidates are rejected without changing the generation
