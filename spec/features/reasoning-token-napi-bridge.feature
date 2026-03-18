@done
@RIG-012
@wip
@napi-layer
Feature: Reasoning token NAPI bridge
  """
  Data flow: TokenInfo.reasoning_tokens → StreamEvent::Tokens → NAPI TokenTracker.reasoning_tokens → JavaScript
  Also: BackgroundSession caches reasoning tokens for sync access
  """

  Background: 
    Given a developer using extended thinking models
    And the model returns reasoning tokens in its Usage response

  Scenario: NAPI TokenTracker includes reasoning_tokens field
    Given a NAPI TokenTracker struct definition
    Then it should have a reasoning_tokens field of type Option<u32>
    And the field should be exposed to JavaScript as reasoningTokens

  Scenario: StreamEvent::Tokens conversion maps reasoning tokens to NAPI TokenTracker
    Given a StreamEvent::Tokens with TokenInfo containing reasoning_tokens of 5000
    When the stream event is converted to a StreamChunk::TokenUpdate
    Then the resulting NAPI TokenTracker should have reasoning_tokens equal to Some(5000)

  Scenario: Background session caches reasoning tokens for sync access
    Given a BackgroundSession receiving TokenUpdate events with reasoning_tokens
    When update_tokens is called with reasoning_tokens of 6000
    Then session_get_tokens should return reasoning_tokens of 6000
