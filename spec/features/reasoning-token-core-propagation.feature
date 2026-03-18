@done
@RIG-012
@wip
@rust-layer
Feature: Reasoning token propagation in core streaming display
  """
  Data flow: ApiTokenUsage.reasoning_tokens → TokenDisplayUpdate.reasoning_tokens → StreamingTokenDisplay
  Also: Compaction TokenTracker must include reasoning_tokens in total_tokens()
  """

  Background: 
    Given a developer using extended thinking models
    And the model returns reasoning tokens in its Usage response

  Scenario: TokenDisplayUpdate propagates reasoning tokens from Usage
    Given a StreamingTokenDisplay initialized with previous session values
    And the Usage event contains reasoning_tokens of 5000
    When update_from_usage is called with the Usage event
    Then the returned TokenDisplayUpdate should have reasoning_tokens equal to 5000
    And total_context should include reasoning_tokens in the sum

  Scenario: StreamingTokenDisplay propagates reasoning from final response
    Given a StreamingTokenDisplay for an OpenAI-compatible provider
    And the provider sends no Usage events during streaming
    When update_from_final_response is called with reasoning_tokens of 4000
    Then the returned TokenDisplayUpdate should have reasoning_tokens equal to 4000

  Scenario: Compaction TokenTracker includes reasoning in total_tokens
    Given a compaction model TokenTracker with input_tokens 10000, output_tokens 2000, and reasoning_tokens 5000
    When total_tokens is called
    Then the result should be 17000
    And effective_tokens should also account for reasoning tokens
