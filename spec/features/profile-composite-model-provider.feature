@BUG-318
@session
@model-selection
Feature: Profile-qualified model strings resolve to their provider
  """
  The canonical profile model string is "{provider}:{profile}/{model}" (build_model_string,
  parse_persisted, the TS parseModelString port, and the composite set_model stores).
  codelet_sessions::model_parsing::parse_model_string returns the part before ':' as the registry
  provider, so session creation resolves credentials and reports provider_id for the provider,
  not the profile name.
  """

  Background: User Story
    As a user who selects a model from a named provider profile
    I want new sessions created from that default to use the profile's provider
    So that credentials and provider reporting are correct

  Scenario: A profile-qualified model string parses to its provider
    Given the model string "openai:qwen/Qwen3-80B"
    When parse_model_string is called
    Then it returns registry_provider "openai" and model_part "Qwen3-80B"
    And is_profile_model is true

  Scenario: A session created from a profile-qualified default reports the provider
    Given the global default model is "openai:qwen/Qwen3-80B" with a seeded qwen profile
    When a new session is created
    Then the session model reports provider "openai" and model "Qwen3-80B"
