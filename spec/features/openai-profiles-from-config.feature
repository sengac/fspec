@done
@PROV-100
@tui
@ts-parity
@provider-settings
@configuration
@rust
Feature: OpenAI custom profiles loaded from fspec-config.json

  """
  PROV-100 — Custom OpenAI profiles were never loaded from fspec-config.json.
  TS stores them under providers.openai.profiles (name -> { baseUrl, ... }) in
  ~/.fspec/fspec-config.json, deep-merged with project <cwd>/spec/fspec-config.json
  (project overrides user), loaded via loadProviderProfiles('openai'). The Rust
  port had no reader for this store and the dispatch always passed &[] for
  openai_profiles, so OpenAI only ever showed "+ Add Profile".

  Fix: a pure, path-injectable loader
    profiles_config::load_openai_profiles_from(user_config_dir, project_root) -> Vec<String>
  reads <user_config_dir>/fspec-config.json and <project_root>/spec/fspec-config.json,
  merges providers.openai.profiles key-by-key (project overrides user), and returns
  display strings sorted by profile name. Each display string is
  "{name} → {baseUrl}" (just "{name}" when baseUrl is missing/empty). Missing
  files, empty content, or malformed JSON yield an empty list (silent fallback,
  no panic) mirroring TS loadConfig.

  A thin wrapper load_openai_profiles() resolves HOME (read-only) + current_dir
  and delegates; dispatch_provider_settings.rs::handle_provider_credentials_loaded
  passes its result into project_display_infos(&list, &profiles) instead of &[],
  so the expanded OpenAI provider renders Profile child rows above "+ Add Profile".
  The projection (projection.rs) and nav-item builder (nav_item.rs) already support
  a non-empty openai_profiles slice (openai-only, per TS Rule 29).

  All loader tests inject temp dirs — no real $HOME, no env mutation, no network.
  """

  Scenario: User config profile is loaded and formatted as name then arrow then baseUrl
    Given a user config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://api.fireworks.ai/inference"
    And an empty project config directory
    When load_openai_profiles_from is called with the user and project directories
    Then the result is the single display string "fireworks → https://api.fireworks.ai/inference"

  Scenario: Project profile overrides user profile by name
    Given a user config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://user.example/v1"
    And a project config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://project.example/v1"
    When load_openai_profiles_from is called with the user and project directories
    Then the result is the single display string "fireworks → https://project.example/v1"

  Scenario: User and project profiles are merged and sorted by name
    Given a user config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://api.fireworks.ai/inference"
    And a project config fspec-config.json with an openai profile "together" whose baseUrl is "https://api.together.xyz/v1"
    When load_openai_profiles_from is called with the user and project directories
    Then the result is the display strings "fireworks → https://api.fireworks.ai/inference" then "together → https://api.together.xyz/v1" in that order

  Scenario: Profile without a baseUrl renders as just the name
    Given a user config fspec-config.json with an openai profile "local" that has no baseUrl
    And an empty project config directory
    When load_openai_profiles_from is called with the user and project directories
    Then the result is the single display string "local"

  Scenario: Missing config files yield no profiles
    Given a user config directory with no fspec-config.json
    And an empty project config directory
    When load_openai_profiles_from is called with the user and project directories
    Then the result is an empty list

  Scenario: Malformed JSON yields no profiles without panicking
    Given a user config fspec-config.json whose contents are malformed JSON
    And an empty project config directory
    When load_openai_profiles_from is called with the user and project directories
    Then the result is an empty list

  Scenario: A loaded profile renders as a Profile row above Add Profile in the OpenAI nav tree
    Given an OpenAI ProviderDisplayInfo whose profiles slice is the loader output "fireworks → https://api.fireworks.ai/inference"
    And the openai provider is expanded
    When the nav items are built
    Then a Profile row "fireworks → https://api.fireworks.ai/inference" appears immediately above the Add Profile row
