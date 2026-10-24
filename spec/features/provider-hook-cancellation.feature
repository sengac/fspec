@done
@CMPCT-082
@testing
@providers
@context-management
@compaction
Feature: Provider guard cancellation never becomes successful completion
  """
  Ordinary streaming guards retain cancellation as an explicit failure at every
  phase. A before-request cancellation sends no request. No completion notice or
  substitute compaction can disguise a cancelled inference.
  """

  Scenario Outline: A cancelled streaming guard cannot become successful completion
    Given an ordinary provider guard cancels <phase>
    When the real streaming agent consumes the provider response
    Then cancellation returns an explicit failure without successful completion
    And a before-request cancellation sends no physical inference

    Examples:
      | phase          |
      | before request |
      | during text    |
      | at finish      |
