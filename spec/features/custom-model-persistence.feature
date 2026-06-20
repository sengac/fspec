@done
@configuration
@persistence
@RPC-346
Feature: Backend custom-model persistence (save/delete on local-server profiles)

  """
  Extend CustomModelDef (profile_sections.rs:86-89) to full definition. Add serde rename_all = camelCase + skip_serializing_if = Option::is_none on every optional field; id stays required. Add a CompactionThreshold struct ({type: tokens|percentage, value}) mirroring TS CompactionThresholdConfig, also camelCase. Derive Serialize + Deserialize + Debug + Clone + PartialEq.
  save_custom_model(provider_id, profile_name, def, original_model_id: Option<&str>) -> io::Result<()> and delete_custom_model(provider_id, profile_name, model_id) -> io::Result<()> in profile_sections.rs. Sync std::fs read-modify-write of the WHOLE config as serde_json::Value (preserve unrelated keys via preserve_order), mutating only providers.openai.profiles.<name>.customModels. Reuse fspec_user_dir() for the path. Guard provider_id == openai (else no-op Ok). Missing profile = Ok no-op. Empty array after delete => remove the customModels key. Sync chosen to match the existing read path load_local_server_profiles; the async RPC bridge is RPC-347.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CustomModelDef is extended to a full definition: id (required) plus optional displayName, facade, contextWindow, maxOutputTokens, compactionThreshold, reasoning, hasVision; it derives both Serialize and Deserialize with camelCase wire names matching the TS CustomModelDefinition
  #   2. save_custom_model in ADD mode (no original id) appends the definition to the profile's customModels array
  #   3. save_custom_model in EDIT mode (original id given) replaces the entry whose id equals the original id with the new definition, preserving array order
  #   4. delete_custom_model removes the entry whose id matches; when the array becomes empty the customModels key is omitted from the profile entirely (parity with TS undefined)
  #   5. Both operations are whole-file read-modify-write: unrelated config keys (other providers, other profiles, top-level keys, and the target profile's own baseUrl/apiKey) are preserved exactly
  #   6. If the target profile does not exist the operation is a no-op (leaves the file unchanged and never creates the profile)
  #   7. Operations are guarded to the openai provider only (parity with the TS saveProfile guard); a non-openai provider id is rejected without touching the file
  #   8. Optional definition fields that are None are omitted from the serialized JSON (no null keys), keeping the config backward-compatible
  #
  # EXAMPLES:
  #   1. Add to empty: profile work-vllm has no customModels; saving a new model my-model in add mode leaves customModels equal to exactly [my-model] and the profile's baseUrl/apiKey untouched
  #   2. Edit replaces in place: customModels is [alpha, beta]; saving a definition with id alpha2 and original id alpha yields [alpha2, beta]
  #   3. Delete one of many: customModels is [alpha, beta]; deleting alpha leaves [beta]
  #   4. Delete last entry: customModels is [alpha]; deleting alpha removes the customModels key from the profile entirely
  #   5. Preserve siblings: config has profiles work-vllm and home plus a top-level theme key; adding a model to work-vllm leaves the home profile and the theme key unchanged
  #   6. Missing profile no-op: saving a custom model to a profile name that does not exist leaves the config file byte-for-byte unchanged
  #   7. Full round-trip: saving a definition with every field set (facade gemini, contextWindow, maxOutputTokens, compactionThreshold percentage 80, reasoning true, hasVision true) then reloading the profiles returns a matching definition
  #
  # ========================================

  Background: User Story
    As a Codelet TUI user managing local-server profiles
    I want to have the backend persist custom-model add/edit/delete to my fspec-config.json
    So that the custom models I define survive restarts and show up on next load, just like the TypeScript build

  Scenario: Add a custom model to a profile with no custom models
    Given an "openai" profile "work-vllm" with no custom models
    When I save a new custom model "my-model" in add mode
    Then the profile's custom models are exactly ["my-model"]
    And the profile's baseUrl and apiKey are unchanged

  Scenario: Edit replaces the matching custom model in place
    Given an "openai" profile "work-vllm" with custom models ["alpha", "beta"]
    When I save a definition with id "alpha2" and original id "alpha"
    Then the profile's custom models are exactly ["alpha2", "beta"]

  Scenario: Delete one of several custom models
    Given an "openai" profile "work-vllm" with custom models ["alpha", "beta"]
    When I delete the custom model "alpha"
    Then the profile's custom models are exactly ["beta"]

  Scenario: Deleting the last custom model removes the customModels key
    Given an "openai" profile "work-vllm" with custom models ["alpha"]
    When I delete the custom model "alpha"
    Then the profile has no customModels key

  Scenario: Unrelated config is preserved on save
    Given a config with "openai" profiles "work-vllm" and "home" and a top-level "theme" key
    When I save a new custom model "my-model" to "work-vllm" in add mode
    Then the "home" profile is unchanged
    And the top-level "theme" key is unchanged

  Scenario: Saving to a missing profile is a no-op
    Given an "openai" profile "work-vllm" exists
    When I save a custom model to the profile "does-not-exist"
    Then the config file is unchanged

  Scenario: Full definition round-trips through save and reload
    Given an "openai" profile "work-vllm" with no custom models
    When I save a custom model with every field set
    And I reload the local-server profiles
    Then the reloaded profile contains a matching custom model definition

  Scenario: A custom model with a float compaction value still loads
    Given an "openai" profile "work-vllm" whose custom model stores compactionThreshold.value as the float 80.0
    When I reload the local-server profiles
    Then the profile loads and the custom model's compaction value is 80

  Scenario: A stored display name is surfaced when merging custom models
    Given a custom model stored with displayName "My Model" and another with no stored display name
    When the custom models are merged into wire model rows
    Then the first row's display name is "My Model" and the second falls back to its id
