@BUG-582
@done
@cli
@oauth
@validation
@providers
@high
@PROV-168
@critical
Feature: Anthropic OAuth wire contract
  """
  PROV-168 protocol investigation is attached to the card. Preserve OAuth transport and model-aware thinking. Unknown occupancy before provider usage is distinct from model capacity. This feature has one corresponding integration test target.
  """

  Background: User Story
    As a Claude subscription user
    I want current models to receive a supported OAuth protocol and selected thinking effort
    So that requests succeed or provide actionable failure information

  Scenario: Current model requests preserve the OAuth contract
    Given a subscription session selects Opus 5.5 and medium thinking
    When a request is sent through the Anthropic provider
    Then the request preserves bearer authentication OAuth headers Claude identity and adaptive effort without unrelated feature betas

  Scenario: Rejected client versions provide actionable errors
    Given Anthropic rejects the client version with a higher minimum and a request identifier
    When a subscription request fails before returning usage
    Then the error preserves the server details and explains updating the version source without retrying
