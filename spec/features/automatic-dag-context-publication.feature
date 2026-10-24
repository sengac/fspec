@agent-core
@context-management
@compaction
@CMPCT-059
Feature: Automatic DAG context publication
  """
  Preserves the automatic builder publication acceptance from durable-versioned-dag-boundary.
  Observer publication and failure ownership must be exercised by the real builder path.
  The same acceptance scenario exercises successful publication and observer
  rejection against independent durable sessions, including completion ordering.
  """

  Scenario: Automatic builder publication refreshes the authoritative context
    Given a durable session and a fitting candidate from the selected builder
    When automatic compaction promotes the candidate
    Then observers receive the committed generation and context before completion
    And publication failure retains the exact pending identity and inference barrier
