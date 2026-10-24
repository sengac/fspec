@done
@persistence
@context-management
@compaction
@phase-1
@CMPCT-064
Feature: Session manifest lost acknowledgement
  """
  Atomic promotion is irreversible even if the writer exits without returning
  acknowledgement. A fresh process reads the promoted revision, never stages.
  """

  Scenario: Lost acknowledgement cannot roll back a promoted revision
    Given a new revision was durably promoted without its acknowledgement reaching the caller
    When a fresh store opens the session
    Then it observes the promoted revision without creating another commit
