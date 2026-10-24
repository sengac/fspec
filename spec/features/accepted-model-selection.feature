@BUG-226
@tui
@model-selection
@integration
Feature: Display only accepted session model selections
  """
  Selection requests are not model identity. The TUI publishes identity from
  backend session metadata alongside refreshed model chrome. Per-session
  serialization and revision checks prevent older reads or writes from
  overwriting newer results. Feedback is routed to the originating session.
  No inference request is needed to reconcile selection.
  """

  Background: User Story
    As an agent session user
    I want model displays to reflect accepted backend selections
    So that a rejected request cannot misidentify my provider

  Scenario: Pending and rejected selection preserves accepted identity
    Given a session displays accepted model A
    When model B is requested and rejected as busy
    Then model A remains selected before and after completion
    And the session shows the rejection reason

  Scenario: Successful selection publishes backend identity rather than request alias
    Given a session displays accepted model A
    When a model alias is accepted as canonical model B
    Then the selector and header display canonical model B

  Scenario: Selection feedback stays with its originating session
    Given a session displays accepted model A
    When its rejected selection completes after focus moves to another session
    Then only the originating session receives the rejection notice
    And the other session identity is unchanged

  Scenario: Delayed earlier selection cannot overwrite a newer request
    Given a session displays accepted model A
    When model B is delayed and model C is requested next
    Then backend writes complete in request order
    And reversed response delivery still displays model C

  Scenario: Earlier chrome hydration cannot overwrite an accepted selection
    Given an earlier model A chrome read is delayed
    When model B is accepted before that read is delivered
    Then delayed hydration does not replace model B

  Scenario: A later rejection retains an earlier accepted pending selection
    Given a session displays accepted model A
    When model B is accepted and a subsequent model C request is rejected
    Then reversed response delivery retains accepted model B and shows the rejection

  Scenario: Reconciliation failure retains accepted display and surfaces feedback
    Given a session displays accepted model A
    When a model switch fails and backend metadata cannot be refreshed
    Then model A remains selected and the session shows the failure

  Scenario: Selection without a runtime does not publish an unaccepted model
    Given a session displays accepted model A without an async runtime
    When model B is requested
    Then model A remains selected and the session shows an unavailable notice
