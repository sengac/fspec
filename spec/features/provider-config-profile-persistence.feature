@done
@ts-parity
@rust
@profiles
@persistence
@PROV-108
Feature: Backend profile write path — save/delete profile in fspec-config.json
  """
  New module codelet/sessions/src/profile_persistence.rs holds save_profile/delete_profile (env-resolved) + save_profile_at/delete_profile_at (path-injectable cores). Reuses fspec_user_dir/read_config_value/write_config_value from profile_sections (made pub(crate)) to avoid duplication and NOT bloat profile_sections.rs (485 prod LoC).
  Wire type codelet_rpc_types::ProfileDefinition (base_url, api_key, optional context_window/max_output_tokens + flat compaction_threshold_type/value) mirrors CustomModelDefinition convention. conversions::profile_def_from_wire folds flat compaction fields into profile_sections::CompactionThreshold.
  Wired through 5 crates mirroring RPC-347: SessionManagerHandle::save_profile/delete_profile (core default no-op), sessions handle_impl override (openai-guard Err on save, delegates to profile_persistence), rpc FspecService add/delete, napi bindings. Tests offline via FSPEC_USER_DIR temp dir + path-injectable cores.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Profiles are only supported for the openai provider; saving a profile for any other provider returns an error and leaves config untouched
  #   2. Saving a profile is a read-modify-write that preserves the profile's existing customModels and all unrelated keys (sibling profiles, top-level config keys)
  #   3. Saving sets baseUrl and apiKey; optional fields (contextWindow, maxOutputTokens, compactionThreshold) are written when present and removed when absent
  #   4. Saving to a missing config file creates the file with the providers.openai.profiles.<name> structure
  #   5. Deleting a profile removes only the named profile, preserving siblings; deleting from a missing file or a non-existent profile is an idempotent no-op leaving config byte-identical
  #
  # EXAMPLES:
  #   1. save_profile creates a new openai profile in a missing config file with baseUrl and apiKey set
  #   2. save_profile updates an existing profile's baseUrl/apiKey while preserving its customModels array
  #   3. save_profile preserves a sibling profile and a top-level theme key untouched
  #   4. save_profile writes contextWindow, maxOutputTokens and compactionThreshold {type,value} when supplied
  #   5. save_profile for provider anthropic returns Err mentioning OpenAI and leaves config byte-identical
  #   6. delete_profile removes profile work-vllm but leaves sibling profile home intact
  #   7. delete_profile on a missing config file returns Ok and writes nothing
  #   8. delete_profile on a non-existent profile leaves the config byte-identical
  #
  # ========================================
  Background: User Story
    As a TUI user managing local-server provider profiles
    I want to create, edit and delete an openai profile so it persists to ~/.fspec/fspec-config.json
    So that my profile connection settings survive across sessions without losing my custom models or other config

  Scenario: Saving a new profile to a missing config file creates it
    Given no fspec-config.json file exists
    When I save an openai profile "work-vllm" with baseUrl "http://localhost:8888" and apiKey "sk-test"
    Then the call returns Ok
    And the profile "work-vllm" has baseUrl "http://localhost:8888"
    And the profile "work-vllm" has apiKey "sk-test"

  Scenario: Saving an existing profile preserves its custom models
    Given an openai profile "work-vllm" exists with a custom model "alpha"
    When I save the profile "work-vllm" with baseUrl "http://localhost:9999" and apiKey "sk-new"
    Then the call returns Ok
    And the profile "work-vllm" has baseUrl "http://localhost:9999"
    And the profile "work-vllm" still has the custom model "alpha"

  Scenario: Saving a profile preserves sibling profiles and top-level keys
    Given a config with openai profiles "work-vllm" and "home" and a top-level "theme" key
    When I save the profile "work-vllm" with baseUrl "http://localhost:1111" and apiKey "sk-x"
    Then the call returns Ok
    And the sibling profile "home" is unchanged
    And the top-level "theme" key is unchanged

  Scenario: Saving a profile writes supplied optional fields
    Given no fspec-config.json file exists
    When I save an openai profile "work-vllm" with contextWindow 32000, maxOutputTokens 4096 and compactionThreshold percentage 80
    Then the call returns Ok
    And the profile "work-vllm" has contextWindow 32000
    And the profile "work-vllm" has maxOutputTokens 4096
    And the profile "work-vllm" has compactionThreshold type "percentage" and value 80

  Scenario: Saving a profile for a non-openai provider is rejected
    Given a config with an openai profile "work-vllm"
    When I save a profile for provider "anthropic"
    Then the call returns Err mentioning OpenAI
    And the configuration is left byte-identical

  Scenario: Deleting a profile removes only the named profile
    Given a config with openai profiles "work-vllm" and "home"
    When I delete the profile "work-vllm"
    Then the call returns Ok
    And the profile "work-vllm" is gone
    And the sibling profile "home" is unchanged

  Scenario: Deleting from a missing config file is a no-op
    Given no fspec-config.json file exists
    When I delete the profile "work-vllm"
    Then the call returns Ok
    And no fspec-config.json file is written

  Scenario: Deleting a non-existent profile leaves config unchanged
    Given a config with an openai profile "work-vllm"
    When I delete the profile "does-not-exist"
    Then the call returns Ok
    And the configuration is left byte-identical
