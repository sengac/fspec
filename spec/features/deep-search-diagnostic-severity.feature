@done
@phase-1
@tools
@research-tools
@error-handling
@BUG-238
Feature: DeepSearch diagnostic severity
  """
  Routine startup, dispatch, custom-provider routing, construction and completion
  diagnostics use DEBUG in both shared and NAPI handlers. Genuine build/prompt
  failures and wall-clock timeouts keep their warning severity. Exercise real
  handlers against an isolated local custom provider with a scoped subscriber;
  do not change provider selection, recursion, admission or timeout policy.
  """

  Scenario: Shared DeepSearch success stays out of incident logs
    Given a local custom provider that returns a successful answer
    When the shared DeepSearch handler completes a search
    Then routine entry routing construction and completion records are DEBUG
    And the records retain provider model and ephemeral session correlation
    And the handler emits no WARN or ERROR records

  Scenario: NAPI DeepSearch success stays out of incident logs
    Given a local custom provider that returns a successful answer
    When the NAPI DeepSearch handler completes a search
    Then routine entry routing construction and completion records are DEBUG
    And the records retain provider model and ephemeral session correlation
    And the handler emits no WARN or ERROR records

  Scenario: Shared DeepSearch failures remain actionable
    Given a local custom provider whose request fails
    When the shared DeepSearch handler returns the failure
    Then the genuine failure remains visible at WARN or ERROR

  Scenario: NAPI DeepSearch failures remain actionable
    Given a local custom provider whose request fails
    When the NAPI DeepSearch handler returns the failure
    Then the genuine failure remains visible at WARN or ERROR
