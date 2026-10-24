@done
@agent-core
@session-management
@persistence
@BUG-250
Feature: Retain and export transcript outcomes with their owning session
  """
  Canonical session deletion owns private lane cleanup. Display export exposes
  typed safe outcomes and original identities without model dialogue mutation.
  Recovery and deletion share the stable recovery lease before session ownership.
  Manifest removal commits deletion before private-lane retirement; late cleanup
  errors explicitly report promotion and remain retryable without resurrection.
  """

  Scenario: Session deletion removes its private transcript
    Given a session with a durable terminal outcome
    When that session is deleted
    Then its transcript records are removed with the owning session

  Scenario: Transcript export retains only public display records
    Given a session with a durable scrubbed provider failure
    When its display-only transcript is exported
    Then the exported record retains its original identity timestamp and typed public outcome

  Scenario: Failed transcript deletion preserves the owning session
    Given a session whose private transcript cannot be removed
    When session deletion encounters the transcript storage failure
    Then deletion reports the failure and retains the committed session for retry
