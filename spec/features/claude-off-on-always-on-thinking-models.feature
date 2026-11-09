@done
@BUG-439
@providers
@thinking-detection
@rust
Feature: Claude Off is honoured or refused on always-on thinking models
  """
  On Claude 5 models thinking is on by default, but
  ClaudeThinkingFacade::request_config_for_model returned None for Off (no
  thinking key) and resolve_turn_thinking short-circuited Off to None, so the
  model thought at its default effort and the user's Off was silently not
  honoured. Per the Anthropic thinking-troubleshooting table (MODEL-009
  research §4): Sonnet 5.5 turns off with {type: between_tools}, Opus 5 and
  Haiku 5.5 with {type: disabled}, and Opus 5.5, Fable 5 / 5.1 and Mythos
  cannot turn thinking off. request_config_for_model now returns
  Result<Option<Value>, String>: the model's Off form, Err for a model that
  cannot (or is not known to) turn thinking off, and None for Claude 4.x where
  omission means off. resolve_turn_thinking returns Result and the agent loop
  refuses the turn with the reason; a malformed thinking config is an error
  instead of being dropped.
  Tests: rust/tools/tests/bug439_claude_off_on_always_on_thinking_models.rs.
  """

  Background: User Story
    As a user who turns thinking Off on a Claude model that thinks by default
    I want to have Off sent in the form the model accepts, or be told it cannot be turned off
    So that Off is never silently ignored

  Scenario: Off is expressed in the form each always-on model accepts
    Given the models "claude-sonnet-5-5", "claude-opus-5" and "claude-haiku-5-5"
    When thinking Off is configured for each
    Then Sonnet 5.5 sends between_tools and Opus 5 and Haiku 5.5 send disabled

  Scenario: Off is refused on a model that cannot turn thinking off
    Given the model "claude-opus-5-5"
    When thinking Off is configured
    Then the configuration fails saying thinking cannot be turned off

  Scenario: Off still sends nothing on a model whose thinking is off by default
    Given the model "claude-opus-4-6"
    When thinking Off is configured
    Then no thinking configuration is sent
