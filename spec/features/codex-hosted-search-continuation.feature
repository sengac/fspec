@PROV-158
@providers
@provider-abstraction
@integration
Feature: Codex hosted search continuation
  """
  Explicitly negotiated search output uses attributable presentation and typed
  replay. Hosted calls never enter the local function-tool dispatcher.
  """

  Scenario: Negotiated hosted search presents attributable status and sources
    Given a Codex session explicitly negotiating hosted search with replay support
    When hosted search progress completion and source citations arrive
    Then visible output attributes status and sources to the hosted item
    And no hosted call is dispatched as a local tool
    And its typed output survives continuation
