@done
@session-management
@phase-1
@session
@agent-loop
@BUG-220
Feature: Loop interruption retry admission
  """
  Loop-abort retry admission captures the interruption epoch and only clears its own internal abort. A later user stop or stale epoch prevents synthetic continuation. Retry diagnostic labels identify the actual retry type rather than always claiming compaction watchdog recovery.
  """

  Background: User Story
    As a session owner
    I want an explicit stop to prevent automatic continuation
    So that internal safety recovery cannot restart stopped work

  Scenario: Genuine loop recovery remains typed and bounded
    Given a deterministic provider repeatedly producing a genuine sustained loop
    When the production agent loop exhausts its configured recovery allowance
    Then every interruption is detector initiated and the corrective retries stop at the limit

  Scenario: User stop invalidates staged loop recovery and stale epochs
    Given a genuine detector abort with a staged corrective retry
    When a user stop arrives before that retry is admitted
    Then the pending recovery is discarded and no new provider request is made
