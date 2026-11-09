@BUG-582
@done
@config-management
@BUG-461
@session
@persistence
@rust
Feature: User config read failures are reported and never overwritten
  """
  Readers and writers of the user's ~/.fspec/fspec-config.json hid a
  malformed file: sessions read_config_value returned None on an unreadable
  or malformed file, so save_profile_at, rename_profile_at and
  save_persisted_model_string_to started from {} and overwrote the user's
  config, delete_profile_at silently did nothing and
  load_persisted_model_string_from treated it as no model;
  save_mux_config_with_dirs also replaced an unreadable or non-object user
  config with {}; and the fspec-tui profiles_config loader (no production
  caller since provider settings moved to RPC) showed no profiles.
  Now every one reads through provider_settings::read_config (a missing
  file is an empty object; an unreadable, malformed or non-object file is
  an error naming the file): writers return the error and leave the file
  unchanged, load_persisted_model_string returns Result and startup shows a
  failed read instead of choosing a model, and load_openai_profiles_from
  returns Result. The legacy default-model.json loader is BUG-463.
  Tests: rust/fspec-tui/tests/bug461_user_config_read_failures_are_reported.rs.
  """

  Background: User Story
    As a user whose fspec-config.json has a typo
    I want to have an unreadable or malformed user config reported naming the file, and saves refused instead of overwriting it
    So that my profiles and settings are never silently ignored or replaced

  Scenario: Saving a profile refuses a malformed user config and leaves it unchanged
    Given the user fspec-config.json contains "{not json"
    When an openai profile is saved
    Then the save fails naming fspec-config.json
    And the user fspec-config.json still contains "{not json"

  Scenario: Saving the last-used model refuses a malformed user config and leaves it unchanged
    Given the user fspec-config.json contains "{not json"
    When the last-used model is saved
    Then the save fails naming fspec-config.json
    And the user fspec-config.json still contains "{not json"

  Scenario: Saving the mux config refuses a malformed user config and leaves it unchanged
    Given the user fspec-config.json contains "{not json"
    When the mux config is saved to the user scope
    Then the save fails naming fspec-config.json
    And the user fspec-config.json still contains "{not json"

  Scenario: Loading the last-used model from a malformed user config is an error
    Given the user fspec-config.json contains "{not json"
    When the last-used model is loaded
    Then loading fails naming fspec-config.json

  Scenario: The provider settings profile loader reports a malformed config
    Given the user fspec-config.json contains "{not json"
    When the openai profiles are loaded for provider settings
    Then loading fails naming fspec-config.json

  Scenario: Startup shows a malformed user config instead of choosing a model
    Given the user fspec-config.json contains "{not json"
    When the TUI starts
    Then an error dialog shows "The last used model could not be read"
    And no startup model is set
