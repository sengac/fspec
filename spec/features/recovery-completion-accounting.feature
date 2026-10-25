@done
@BUG-265
@session
@completion
Feature: Recovery completion accounting
  """
  The real shared agent loop owns dispatch classification. Recovery may sync
  chrome state and arm Done before output/provider construction, but does not
  start another user task. Nudge budgets belong to genuine user tasks; Done
  rejections belong to the active goal lifetime. An identical goal write is not
  a replacement. Preserve CONT-008 generation/replacement race guards and
  CONT-009 arming order through the approved mechanical BackgroundSession split.
  Runtime regressions use real sessions, admission and native Done against a
  synthetic loopback provider, including actual overflow and loop detection.
  No isolated replacement accounting model stands in for the shared loop.
  """

  Background: User Story
    As a user relying on bounded continuation and goal verification
    I want recovery to preserve the unfinished task's completion accounting
    So that automatic retries cannot renew exhausted budgets or hide repeated rejected completion claims

  Scenario: Automatic compaction preserves the consumed nudge budget
    Given a real background task has consumed its only zero-progress nudge
    When a context overflow is compacted and the shared loop retries automatically
    Then the resumed task has no fresh nudge allowance
    And only three inference requests occur with no extra user input

  Scenario: Loop-abort recovery preserves the consumed nudge budget
    Given a real background task has consumed its only zero-progress nudge
    When repetitive provider output triggers the shared loop's corrective retry
    Then the resumed task has no fresh nudge allowance
    And only three inference requests occur with corrective context and no extra user input

  Scenario: A genuine next user task receives a fresh nudge budget
    Given a real background task exhausted its one-nudge allowance
    When the user submits a genuine next task
    Then each task consumes exactly one nudge and finishes at its budget boundary

  Scenario: Rejections across automatic compaction reach goal escalation
    Given two native Done calls were rejected for the active goal
    When automatic compaction resumes the task and two more native Done calls are rejected
    Then the same goal reaches four rejections and pauses for repeated completion failures
    And the registry and inner session retain four rejections

  Scenario: Rejections across loop-abort recovery reach goal escalation
    Given two native Done calls were rejected for the active goal
    When a repetitive-output corrective retry resumes the task and two more native Done calls are rejected
    Then the same goal reaches four rejections and pauses for repeated completion failures
    And the registry and inner session retain four rejections

  Scenario: Genuine user dispatch and identical goal writes preserve rejections
    Given a native Done rejection belongs to the active goal
    When the user submits another task and later writes the identical goal before another task
    Then each rejected native Done adds to the same goal lifetime count
    And Done remains armed before every provider request

  Scenario: Replacing and clearing a goal reset its rejection accounting
    Given a native Done rejection belongs to the active goal
    When the user replaces the goal and receives a rejection for the replacement
    Then the replacement has only its own rejection and its new verification setting
    When the user clears that goal and submits another task
    Then both rejection counts are zero and no stale goal or armed Done remains
