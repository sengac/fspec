@RIG-011
Feature: codelet ApiTokenUsage Reasoning Tokens

  """
  Key files: codelet/core/src/token_usage.rs (ApiTokenUsage struct, update_from_usage, total_context)
  Data flow: rig::completion::Usage → codelet_core::ApiTokenUsage
  """

  Background: User Story
    As a developer
    I want ApiTokenUsage to include reasoning_tokens and account for them in total_context
    So that token tracking in codelet accurately reflects reasoning token consumption

  Scenario: ApiTokenUsage includes reasoning_tokens field
    Given the codelet-core ApiTokenUsage struct is defined
    When I inspect the ApiTokenUsage struct fields
    Then it should have a reasoning_tokens field of type u64
    And the Default impl should set reasoning_tokens to 0

  Scenario: ApiTokenUsage updates from rig Usage with reasoning tokens
    Given a rig Usage with reasoning_tokens Some(3000)
    When update_from_usage is called on ApiTokenUsage
    Then ApiTokenUsage.reasoning_tokens should be 3000

  Scenario: ApiTokenUsage updates from rig Usage with None reasoning tokens
    Given a rig Usage with reasoning_tokens None
    When update_from_usage is called on ApiTokenUsage
    Then ApiTokenUsage.reasoning_tokens should be 0

  Scenario: ApiTokenUsage total_context includes reasoning tokens
    Given an ApiTokenUsage with input_tokens 10000 output_tokens 500 and reasoning_tokens 2000
    When total_context() is called
    Then the result should be 12500

  Scenario: ApiTokenUsage total_context without reasoning tokens
    Given an ApiTokenUsage with input_tokens 10000 output_tokens 500 and reasoning_tokens 0
    When total_context() is called
    Then the result should be 10500
