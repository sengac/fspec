@done
@config-management
@session
@persistence
@rust
@profiles
@provider-settings
@PROV-136
Feature: Profile rename persistence
  """
  Rename is a delete-old-key + write-new-key read-modify-write of providers.openai.profiles in fspec-config.json. A pre-write collision check rejects a rename onto an existing different profile name. customModels and sibling profiles are preserved. Implemented as rename_profile_at in rust/sessions/src/profile_persistence.rs.
  """

  Background: User Story
    As a provider settings user
    I want to rename an OpenAI profile so the config file writes the new name and removes the old one
    So that my renamed profile is persisted correctly without duplicates or data loss

  Scenario: Renaming a profile writes the new name and removes the old name
    Given the config has an openai profile "work-vllm" with base URL and API key set
    When the profile is renamed to "work-vllm-2" and saved
    Then the config has a profile named "work-vllm-2"
    Then the config no longer has a profile named "work-vllm"
    Then the renamed profile keeps its original base URL and API key

  Scenario: Saving with an unchanged name overwrites the same profile
    Given the config has an openai profile "work-vllm" with an API key
    When the profile is saved with the same name and a new API key
    Then the config still has exactly one profile named "work-vllm"
    Then the profile has the new API key

  Scenario: Renaming onto an existing profile name is rejected
    Given the config has openai profiles "work-vllm" and "fast"
    When the profile "work-vllm" is renamed to "fast" and saved
    Then the rename is rejected with an error
    Then both profiles "work-vllm" and "fast" remain unchanged

  Scenario: Renaming preserves the profile custom models
    Given the config has an openai profile "work-vllm" with a customModels array
    When the profile is renamed to "work-vllm-2" and saved
    Then the profile "work-vllm-2" still has its customModels array
