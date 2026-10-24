@CMPCT-065
@providers
@context-management
@compaction
@phase-1
Feature: Builtin OpenAI usage retains measurement evidence
  """
  Normalize measured inclusive usage only after the provider reports it.
  Missing usage cannot become authoritative measured zero at any Chat boundary.
  Explicit measured zero remains valid. No substitute request or estimate is used.
  """

  Scenario: Streaming Chat missing usage is rejected
    Given a physical OpenAI Chat stream omits usage measurements
    When the selected adapter consumes the complete response
    Then it reports missing usage without a measured zero or successful final

  Scenario: Nonstreaming Chat missing usage is rejected
    Given a physical nonstreaming OpenAI Chat response omits usage measurements
    When the selected streaming adapter consumes the complete response
    Then it reports missing usage without a measured zero or successful final

  Scenario: Direct Chat missing usage is rejected
    Given a physical direct OpenAI Chat response omits usage measurements
    When the selected adapter completes the request
    Then it reports missing usage instead of measured zero occupancy

  Scenario: Explicit measured zero remains valid at every Chat boundary
    Given physical OpenAI Chat responses explicitly report zero input output and total usage
    When streaming nonstreaming and direct adapters consume those responses
    Then each returns measured zero usage without inventing missing evidence
