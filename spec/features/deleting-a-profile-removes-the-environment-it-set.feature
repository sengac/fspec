@done
@providers
@provider-settings
@bug-fix
@rust
@BUG-612
Feature: Deleting a profile removes the environment it set
  """
  apply_profile_env_vars (sessions profile_environment.rs) copies an openai profile's baseUrl, apiKey, contextWindow and streaming into the daemon's process environment, and list_provider_credentials reports a provider configured from the environment, so a deleted profile's key kept being used and reported until the daemon restarted. The profile environment now writes through a per-variable ledger of layers (profile, value before the write, value written). delete_profile removes the profile's layers: a variable still holding the profile's value goes back to its value before the profile wrote it; a variable written since is left alone; a later layer's prior that was the deleted profile's value becomes that profile's own prior. Product owner, 2026-11-07 (A): a credential delete keeps PROV-133 and unsets the provider's variables whatever their origin, dropping their layers.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A profile's environment writes are recorded in per-variable layers (prior value, written value)
  #   2. Deleting profile P restores what P overwrote where P's value is still current, leaves later writes alone, and never lets a deleted key come back
  #   3. A variable the user set and no profile overwrote is not touched by a profile delete
  #   4. A credential delete (PROV-133) unsets the provider's variables whatever their origin and drops their layers
  #
  # EXAMPLES:
  #   1. Save an openai profile with key sk-test and delete it: openai is no longer configured
  #   2. OPENAI_API_KEY=user-key at start; profile applied (sk-test); deleting it restores user-key
  #   3. Profiles local (sk-a) then other (sk-b) applied; deleting local leaves sk-b; deleting other then unsets the key
  #   4. user-key, profile applied, credentials deleted: unset; deleting the profile afterwards leaves it unset
  #
  # ========================================
  Background: User Story
    As a user managing provider profiles and credentials through the TUI or an RPC client
    I want a deleted profile's key to stop being used and reported as soon as I delete it
    So that the daemon never keeps calling a provider with a key I removed, and configured means what it says

  Scenario: Deleting the applied profile removes its key from the environment
    Given OPENAI_API_KEY is unset and an openai profile "local" with key sk-test is applied
    When the profile "local" is deleted
    Then OPENAI_API_KEY and OPENAI_BASE_URL are unset
    And list_provider_credentials no longer reports openai as configured

  Scenario: Deleting a profile restores the key the user exported
    Given OPENAI_API_KEY is user-key when the daemon starts
    And the openai profile "local" with key sk-test is applied
    When the profile "local" is deleted
    Then OPENAI_API_KEY is user-key

  Scenario: Deleting a profile that set nothing leaves the user's key
    Given OPENAI_API_KEY is user-key when the daemon starts and no profile is applied
    When the openai profile "local" is deleted
    Then OPENAI_API_KEY is user-key

  Scenario: Deleting an earlier profile never brings its key back
    Given OPENAI_API_KEY is unset, the profile "local" with key sk-a is applied, then the profile "other" with key sk-b
    When the profile "local" is deleted
    Then OPENAI_API_KEY is sk-b
    When the profile "other" is deleted
    Then OPENAI_API_KEY is unset

  Scenario: A credential delete still unsets the provider's key
    Given OPENAI_API_KEY is user-key when the daemon starts and the profile "local" with key sk-test is applied
    When the openai credentials are deleted
    Then OPENAI_API_KEY is unset
    When the profile "local" is deleted
    Then OPENAI_API_KEY is still unset
