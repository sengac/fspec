@BUG-582
@done
@BUG-325
@model-selection
@tui
@rpc
Feature: Custom-model writes under an OpenAI profile section reach the profile
  """
  The daemon keys every profile section "{provider}:{profile}" (sessions/src/profile_sections.rs build_profile_provider_info), and the model selector used that key as the provider id of add/update/delete_custom_model, which the daemon refuses (add/update) or silently ignores (delete), while the TUI discarded every result. The wire ProviderInfo now carries the base provider explicitly in a provider_id field (appended after is_unreachable); model-selector rows copy it and the CRUD forms send it, never the section key. delete_custom_model refuses a non-openai provider id like add/update, and every write the daemon cannot apply (missing profile, missing model) is an error. The TUI logs a failed write, shows it inside the model selector and as a notice in the session's scrollback.
  """

  Background: User Story
    As a user managing custom models on an OpenAI profile from the model selector
    I want my add, edit and delete actions to reach that profile, or tell me why they did not
    So that custom models never silently fail to save

  Scenario: Custom-model writes from a profile section name the base provider
    Given the daemon lists the OpenAI profile section for "my-profile" with the custom model "mycustom"
    When I add, edit and delete a custom model from that section in the model selector
    Then each write is sent with provider id "openai" and profile "my-profile"

  Scenario: A custom model added from a profile section is saved to that profile
    Given an OpenAI profile "my-profile" in the user config
    And the model selector shows the daemon's section for that profile
    When I add the custom model "newmodel" from that section
    Then the profile "my-profile" lists the custom model "newmodel"

  Scenario: Deleting a custom model with a section key as provider id is refused
    Given an OpenAI profile "my-profile" with the custom model "mycustom"
    When a client calls delete_custom_model with provider id "openai:my-profile"
    Then the call returns an error naming the provider id "openai:my-profile"
    And the profile "my-profile" still lists the custom model "mycustom"

  Scenario: A failed custom-model write is shown to the user
    Given the model selector is open for a session on the OpenAI profile section for "my-profile"
    When deleting the custom model "missing" fails because the profile does not have it
    Then the model selector shows the error
    And the session's scrollback shows the error
