@done
@agent-core
@session-management
@persistence
@BUG-249
Feature: Preserve durable transcript ordering and completeness
  """
  A session-wide durable ordinal orders acknowledgements independently of wall
  clocks. Missing interior session or request sequences fail explicitly.
  Durable admission requires a readable truthful tool lifecycle, including the
  candidate delta. Retries of an existing immutable record do not apply it twice.
  """

  Scenario: Durable session order ignores tied and backward timestamps
    Given acknowledged requests whose timestamps tie or move backward
    When the transcript store is reopened
    Then records retain their exact durable admission order

  Scenario: Missing acknowledged interior deltas fail explicitly
    Given three acknowledged deltas in one request
    When an interior delta file is missing on reopen
    Then recovery fails explicitly instead of silently shortening the answer
