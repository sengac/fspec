@done
@RIG-012 @wip @rust-layer
Feature: Reasoning token mapping in CLI TokenInfo

  """
  Data flow: TokenDisplayUpdate.reasoning_tokens → TokenInfo.reasoning_tokens (via From trait)
  Also: ApiTokenUsage.reasoning_tokens → TokenInfo (via from_usage factory)
  """

  Background:
    Given a developer using extended thinking models
    And the model returns reasoning tokens in its Usage response

  Scenario: TokenInfo maps reasoning tokens from TokenDisplayUpdate
    Given a TokenDisplayUpdate with reasoning_tokens set to 3000
    When the TokenDisplayUpdate is converted to TokenInfo via From trait
    Then the resulting TokenInfo should have reasoning_tokens equal to Some(3000)

  Scenario: TokenInfo from_usage maps reasoning tokens from ApiTokenUsage
    Given an ApiTokenUsage with reasoning_tokens of 2000
    When TokenInfo::from_usage is called
    Then the resulting TokenInfo should have reasoning_tokens equal to Some(2000)
