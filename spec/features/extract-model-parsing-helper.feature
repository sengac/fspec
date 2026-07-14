@RPC-424
Feature: Extract model parsing into shared helper function

  """
  The model parsing logic (is_profile_model, is_codex_model, registry_provider, model_part extraction)
  is copy-pasted across three functions in session_manager.rs:
  - create_session_with_id (~lines 637-669)
  - create_session_from_manifest (~lines 938-970)
  - create_isolated_session_with_id (~lines 1218-1245)

  Extract this into a single parse_model_string() helper in a new model_parsing.rs module.
  The helper returns a ModelParseResult struct containing all parsed fields.
  """

  Background: User Story
    As a Rust developer
    I want to have model parsing in a single shared helper
    So that I avoid duplication and maintenance hazards across session creation paths

  Scenario: Parse standard provider/model string
    Given the model string "anthropic/claude-sonnet-4"
    When parse_model_string is called
    Then it returns registry_provider "anthropic" and model_part "claude-sonnet-4"
    And is_profile_model is false
    And is_codex_model is false

  Scenario: Parse profile model string with colon prefix
    Given the model string "profile:anthropic/claude-opus-4"
    When parse_model_string is called
    Then it returns registry_provider "anthropic" and model_part "claude-opus-4"
    And is_profile_model is true

  Scenario: Parse codex model string
    Given the model string "codex/codex-model"
    When parse_model_string is called
    Then it returns registry_provider "codex" and model_part "codex-model"
    And is_codex_model is true

  Scenario: Reject invalid model string without slash
    Given the model string "invalid"
    When parse_model_string is called
    Then it returns an error with a validation message

  Scenario: Reject empty model string
    Given the model string ""
    When parse_model_string is called
    Then it returns an error with a validation message

  Scenario: All three call sites use the shared helper
    Given the parse_model_string helper exists in model_parsing.rs
    When session_manager.rs is examined
    Then create_session_with_id calls parse_model_string
    And create_session_from_manifest calls parse_model_string
    And create_isolated_session_with_id calls parse_model_string
