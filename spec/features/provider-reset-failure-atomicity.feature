@done
@session
@compaction-resume
@phase-1
@CMPCT-061
Feature: Provider reset failures preserve session history and model identity
  """
  Resetting provider replay state is a prerequisite to clearing history or
  switching models. A reset failure must propagate before mutating messages,
  display output or selected model labels. Structured replay acceptance remains
  specified separately in structured-generation-aware-session-replay.feature.
  """

  Scenario: Failed provider reset cannot clear preserved session history
    Given a session whose provider rejects resetting its replay checkpoint
    When the session history is cleared
    Then the failure is returned without deleting history or announcing a cleared session

  Scenario: Failed provider reset cannot report a successful model switch
    Given a session whose provider rejects resetting its replay checkpoint
    When a different model is selected
    Then the reset failure is returned without changing the selected model
