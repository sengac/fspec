@PROV-161
@providers
@provider-settings
@configuration
@phase-1
Feature: Provider runtime physical requests
  """
  Admission and physical requests use resolved output budgets. Codex retains its admission budget without forbidden wire parameters. Anthropic and Gemini preserve signed reasoning by default. Progressive display may be disabled without disabling a transport that requires streaming.
  """

  Background: User Story
    As a developer running fspec sessions
    I want effective settings applied to actual requests and response display
    So that provider capabilities and accounting remain consistent

  Scenario: Resolved output budgets reach physical requests
    Given an OpenAI-compatible model with a saved maximum output override
    When the selected agent prepares a physical request
    Then the serialized output limit and admission reservation equal the effective saved budget

  Scenario: Codex output budgets omit unsupported wire parameters
    Given a Codex model with an effective output budget
    When the selected agent prepares a physical request
    Then admission reserves that budget without serializing a forbidden maximum output parameter

  Scenario: Signed reasoning defaults survive automatic settings
    Given Anthropic and Gemini selections without a thinking preservation override
    When their outgoing histories are prepared
    Then signed reasoning blocks remain present by default

  Scenario: Streaming preferences control progressive display
    Given a provider with progressive streaming disabled and a streaming-required transport
    When a response containing multiple assistant text chunks is delivered
    Then assistant text is buffered until completion without disabling the required transport
