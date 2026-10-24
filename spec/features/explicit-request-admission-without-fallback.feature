@testing
@providers
@context-management
@compaction
@CMPCT-082
Feature: Explicit request admission without fallback
  """
  Physical inference requires explicit configuration or authoritative registry
  limits. Unknown capabilities and unavailable admission state block sending.
  Only declared adapter diagnostics identify context overflow; free-form error
  text cannot activate alternate recovery. Preserve committed DAG generations.
  """

  Background: User story
    As a session owner
    I want unsafe inference to stop without substitute success
    So that unknown limits and unrelated errors cannot change committed memory

  Scenario: Missing model limits cannot become usable defaults
    Given a Codex model without registry limits or explicit limits
    When the model limits are resolved for admission
    Then context and output capacity remain unavailable

  Scenario: Unknown output capacity cannot become an output reservation
    Given an explicit context capacity and unknown output capacity
    When the usable request budget is calculated
    Then no positive usable budget is returned

  Scenario: Unrelated provider text does not request compaction
    Given an unknown error mentioning invalid_request_error tokens and maximum
    When recovery classifies the error through its source chain
    Then compaction is not requested

  Scenario: Declared typed overflow remains detectable through wrappers
    Given a declared Codex context length rejection
    When recovery classifies the error through streaming and context wrappers
    Then compaction is requested
