@done
@PROV-169
@model-selection
@providers
@context-window
Feature: Local-server profile models report only declared context windows
  """
  build_local_profile_sections (rust/sessions/src/profile_sections.rs) used to stamp
  profile.context_window.unwrap_or(128_000) onto every discovered and custom model of a local-server
  profile, publishing an invented 128k limit as ModelEntry.context_window over list_providers.
  ModelEntry.context_window is now Option<u32> (rust/rpc-types/src/dialogs.rs): merge_profile_models
  takes the profile's Option and each custom model's own value wins over it; with neither the window
  is None. Catalog entries (cloud_models) map a models.dev context of 0 to None. The model selector
  row shows no size for None.
  """

  Background: User Story
    As a fspec user with a local-server model profile
    I want a model's context window to be shown only when the profile or the model declares it
    So that clients never budget against an invented 128k limit

  Scenario: A discovered model of a profile without a declared context window has none
    Given a local-server profile that declares no context window
    When its discovered model "qwen3" is merged into the provider section
    Then the model entry "qwen3" has no context window

  Scenario: A profile's declared context window applies to its discovered models
    Given a local-server profile that declares a context window of 32768
    When its discovered model "qwen3" is merged into the provider section
    Then the model entry "qwen3" has a context window of 32768

  Scenario: A custom model's own context window wins over the profile's
    Given a local-server profile that declares no context window
    And a custom model "big" that declares a context window of 131072
    When the custom model is merged into the provider section
    Then the model entry "big" has a context window of 131072
