@phase-1
@cli
@providers
@provider-abstraction
@regression
Feature: Preserve committed tool outcomes when Codex accepts completion
  """
  Architecture notes:
  The streaming consumer exits as soon as the real done tool records accepted
  completion. A committed Codex response can contain additional calls which have
  not executed. The consumer records their cancellation before publishing done,
  preserving the accepted result and the original invocation identities.
  """

  Background: User Story
    As a Codex session owner
    I want accepted completion to stop the remaining committed tools safely
    So that the next conversation turn can replay a complete tool history

  Scenario: Accepted completion cancels the unexecuted remainder of a committed batch
    Given a Codex response commits an accepted done call followed by another tool call
    When the streaming consumer accepts completion from the real done tool
    Then the remaining tool never executes
    And the accepted done result remains unchanged
    And the remaining call and its accepted completion cancellation are published before done
    And no user interruption is reported
    And the next turn succeeds with both original invocation identities and outcomes
