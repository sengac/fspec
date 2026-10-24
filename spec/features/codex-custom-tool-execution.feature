@PROV-157
@providers
@provider-abstraction
@streaming
@integration
Feature: Codex custom tool execution
  """
  Raw custom input remains presentation-only until completion. Existing rig
  dispatch authorization remains authoritative and custom result identities
  survive continuation without bypassing the local executor contract.
  """

  Scenario: Raw input updates remain presentation only
    Given an enabled custom call arriving in separate Unicode input updates
    When partial input arrives before its completed item
    Then updates are visible but only the completed item becomes executable

  Scenario: Existing execution authorization applies to raw tools
    Given a custom patch call subject to an execution denial
    When the completed call reaches the existing tool dispatch hook
    Then the patch executor is not invoked
