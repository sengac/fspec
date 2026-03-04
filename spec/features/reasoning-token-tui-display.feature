@done
@RIG-012 @wip @typescript-layer
Feature: Reasoning token TUI display and persistence

  """
  Data flow: NAPI TokenTracker.reasoningTokens → TypeScript SessionHeader display
  Also: Context fill calculation, token persistence, and session restore
  """

  Background:
    Given a developer using extended thinking models
    And the model returns reasoning tokens in its Usage response

  Scenario: SessionHeader displays reasoning tokens when present
    Given the TypeScript TokenTracker interface includes reasoningTokens optional number
    And a session with 10000 input tokens, 2000 output tokens, and 5000 reasoning tokens
    When the SessionHeader component renders
    Then the token display should show reasoning tokens with a brain emoji indicator
    And reasoning tokens should not be shown when the value is 0 or undefined

  Scenario: Token persistence saves and restores reasoning tokens
    Given a session with accumulated reasoning_tokens of 8000
    When persistTokenState is called for the session
    Then reasoning_tokens should be included in the persisted data
    And when the session is restored via resume
    Then the restored token state should include reasoning_tokens of 8000
