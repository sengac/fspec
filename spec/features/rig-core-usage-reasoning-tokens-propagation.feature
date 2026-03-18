@RIG-011
Feature: rig-core Usage Reasoning Tokens Propagation
  """
  Key files: codelet/patches/rig-core/src/completion/request.rs (Usage struct),
  codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs (Responses API conversion + OutputTokensDetails),
  codelet/patches/rig-core/src/providers/openai/responses_api/streaming.rs (Responses streaming token_usage()),
  codelet/patches/rig-core/src/providers/openai/completion/mod.rs (Completions API Usage struct — needs CompletionTokensDetails),
  codelet/patches/rig-core/src/providers/openai/completion/streaming.rs (Completions streaming)
  """

  Background: User Story
    As a developer
    I want reasoning tokens propagated through rig-core Usage structs and OpenAI API conversions
    So that downstream consumers can track reasoning token consumption accurately

  # ----- Layer 1: rig-core completion::Usage struct -----
  Scenario: rig-core Usage struct includes reasoning_tokens field
    Given the rig-core completion Usage struct is defined
    When I inspect the Usage struct fields
    Then it should have a reasoning_tokens field of type Option<u64>
    And the Default impl should set reasoning_tokens to None
    And the new() constructor should set reasoning_tokens to None

  Scenario: Usage Add impl correctly sums reasoning_tokens
    Given a Usage with reasoning_tokens Some(100)
    And another Usage with reasoning_tokens Some(200)
    When the two Usage values are added together
    Then the result should have reasoning_tokens Some(300)

  Scenario: Usage AddAssign impl correctly accumulates reasoning_tokens
    Given a Usage with reasoning_tokens Some(100)
    When I add-assign a Usage with reasoning_tokens Some(200)
    Then the original should have reasoning_tokens Some(300)

  Scenario: Usage Add handles None reasoning_tokens gracefully
    Given a Usage with reasoning_tokens Some(100)
    And another Usage with reasoning_tokens None
    When the two Usage values are added together
    Then the result should have reasoning_tokens Some(100)

  Scenario: Usage Add handles both None reasoning_tokens
    Given a Usage with reasoning_tokens None
    And another Usage with reasoning_tokens None
    When the two Usage values are added together
    Then the result should have reasoning_tokens None

  # ----- Layer 2: OpenAI Responses API → completion::Usage -----
  Scenario: OpenAI Responses API non-streaming propagates reasoning tokens into Usage
    Given an OpenAI Responses API CompletionResponse with output_tokens_details.reasoning_tokens of 1500
    When the response is converted to completion::CompletionResponse via TryFrom
    Then the Usage should have reasoning_tokens Some(1500)

  Scenario: OpenAI Responses API non-streaming propagates cache_read_input_tokens
    Given an OpenAI Responses API CompletionResponse with input_tokens_details.cached_tokens of 8000
    When the response is converted to completion::CompletionResponse via TryFrom
    Then the Usage should have cache_read_input_tokens Some(8000)

  Scenario: OpenAI Responses API streaming propagates reasoning tokens
    Given a StreamingCompletionResponse with usage containing output_tokens_details.reasoning_tokens of 2000
    When token_usage() is called on the streaming response
    Then the returned Usage should have reasoning_tokens Some(2000)

  Scenario: OpenAI Responses API streaming propagates cache_read_input_tokens
    Given a StreamingCompletionResponse with usage containing input_tokens_details.cached_tokens of 5000
    When token_usage() is called on the streaming response
    Then the returned Usage should have cache_read_input_tokens Some(5000)

  # ----- Layer 2b: OpenAI Completions API → completion::Usage -----
  Scenario: OpenAI Completions API Usage struct includes completion_tokens_details
    Given the OpenAI Completions API Usage struct is defined
    When I inspect the Usage struct fields
    Then it should have a completion_tokens_details field of type Option<CompletionTokensDetails>
    And CompletionTokensDetails should have a reasoning_tokens field

  Scenario: OpenAI Completions API streaming propagates reasoning tokens
    Given an OpenAI Completions API streaming chunk with completion_tokens_details containing reasoning_tokens of 3000
    When the Usage event is emitted during streaming
    Then the emitted completion::Usage should have reasoning_tokens Some(3000)

  Scenario: OpenAI Completions API non-streaming propagates reasoning tokens
    Given an OpenAI Completions API CompletionResponse with completion_tokens_details containing reasoning_tokens of 1200
    When the response is converted to completion::Usage
    Then the Usage should have reasoning_tokens Some(1200)

  Scenario: OpenAI Completions API GetTokenUsage propagates reasoning tokens
    Given an OpenAI Completions API Usage with completion_tokens_details containing reasoning_tokens of 800
    When token_usage() is called via GetTokenUsage trait
    Then the returned completion::Usage should have reasoning_tokens Some(800)
