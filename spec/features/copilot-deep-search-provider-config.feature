@done
@authentication
@providers
@PROV-057
Feature: Copilot DeepSearch sub-agent provider configuration
  """
  PROV-057 L3 (DeepSearch half): rust/napi/src/deep_search_provider_config.rs
  must import select_copilot_facade and handle the github-copilot branch
  so DeepSearch sub-agents can use Copilot as their provider without
  tripping the "Unsupported provider for DeepSearch sub-agent" error.
  """

  Background: User Story
    As a fspec user
    I want DeepSearch sub-agents to use my github-copilot provider
    So that sub-agent research queries work when Copilot is my selected provider

  @copilot
  @deep-search
  Scenario: DeepSearch sub-agents can use github-copilot as their provider
    Given a session is configured with provider "github-copilot"
    When a DeepSearch sub-agent is spawned
    Then request_config_for_provider("github-copilot", model, prompt, false) returns Ok
    And the returned config preamble is built using select_copilot_facade
    And the returned config does NOT trigger the "Unsupported provider for DeepSearch sub-agent" error
