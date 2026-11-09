@done
@cli
@oauth
@validation
@providers
@high
@PROV-168
@critical
Feature: Rejected Anthropic context provenance
  """
  PROV-168 protocol investigation is attached to the card. Preserve OAuth transport and model-aware thinking. Unknown occupancy before provider usage is distinct from model capacity. This feature has one corresponding integration test target.
  """

  Background: User Story
    As a Claude subscription user
    I want current models to receive a supported OAuth protocol and selected thinking effort
    So that requests succeed or provide actionable failure information

  Scenario: Rejection before usage leaves occupancy unknown
    Given a new Anthropic session has known model capacity but no measured usage
    When the request is rejected before returning usage
    Then the header keeps context unknown rather than inventing zero percent or measured usage
