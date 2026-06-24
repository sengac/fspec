@done
@provider-abstraction
@providers
@rust
@provider
@PROV-101
Feature: Provider resolution never silently picks a default

  # ARCHITECTURE NOTES (PROV-101 #3):
  # manager.rs detect_default_provider's Claude>Gemini>ZAI>Codex>Copilot>OpenAI
  # priority chain is removed. resolve_unambiguous_provider(credentials):
  #   zero creds  -> Err (existing "No provider credentials available" auth error)
  #   exactly one -> Ok(that provider)            [unambiguous, no choice made]
  #   more than 1 -> Err (none explicitly selected) [no silent Claude pick]
  # Exercised via the public detect_default_provider_for_test shim.

  Background: User Story
    As a developer integrating provider/model/profile selection
    I want provider resolution to refuse to guess when the choice is ambiguous
    So that no code path silently substitutes anthropic/claude

  Scenario: provider resolution accepts a single credentialed provider
    Given credentials for only the openai provider
    When I resolve the provider with no explicit selection
    Then resolution succeeds with the openai provider

  Scenario: provider resolution rejects an ambiguous multi-provider state
    Given credentials for both the anthropic and openai providers
    When I resolve the provider with no explicit selection
    Then resolution returns an error mentioning that none was explicitly selected
    And resolution does not return the claude provider

  Scenario: provider resolution rejects when no credentials are available
    Given no provider credentials are available
    When I resolve the provider with no explicit selection
    Then resolution returns an auth error
