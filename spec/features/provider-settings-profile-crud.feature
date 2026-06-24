@done
@crud
@rust
@tui
@provider-settings
@PROV-111
Feature: Profile nav routing, prefill, per-profile delete and end-to-end refresh

  """
  Final wiring slice of the PROV-106 epic (parity with the TS profile-management
  flows). Earlier slices shipped the backend write path (PROV-108
  provider-config-profile-persistence), the transport/app dispatch
  (PROV-109 provider-config-profile-dispatch), and the ProfileForm UI state +
  renderer (PROV-110 provider-settings-profile-form). PROV-111 connects them:

  list_actions::enter_on_nav_item routes a Profile row Enter into
  ProviderSettingsMode::EditProfile prefilled from the FULL stored
  ProfileConfig (looked up by the focused row's profile name), and an
  AddProfile row Enter into ProviderSettingsMode::CreateProfile (baseUrl
  default http://localhost:8888). delete_on_nav_item routes a Profile row 'd'
  into a per-profile delete-confirm whose Primary acceptance emits
  Action::ConfirmDeleteProfile for ONLY providers.openai.profiles.<name>.

  profiles_config.rs gains a path-injectable full-config loader
  (load_openai_profile_configs_from) mirroring its existing display-string
  loader, returning name -> ProfileDefinition with project-over-user merge.
  ProviderSettingsView stores that per-profile config map (folded inside
  handle_provider_credentials_loaded alongside the display-string slice) and
  exposes profile_config_for(name) so the edit form prefills by name. The
  display string "{name} → {baseUrl}" carried on the Profile row is split back
  to the bare name for the lookup.

  The existing PROV-102 dispatch test is updated to assert the new EditProfile
  mode (intentional in-ACDD behavior change). After any successful save/delete
  the PROV-109 dispatch refresh reloads both slices so the affected row
  repaints end-to-end.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing Enter on a Profile row opens an EditProfile form prefilled from the stored ProfileConfig (name shown non-editable; fields baseUrl, apiKey, contextWindow, maxOutputTokens, compactionThreshold)
  #   2. Pressing Enter on the '+ Add Profile' row opens a CreateProfile form with baseUrl defaulted to DEFAULT_PROFILE_BASE_URL (http://localhost:8888), apiKey empty, and the cursor in the editable name field
  #   3. Pressing 'd' on a Profile row opens a per-profile delete-confirm whose acceptance emits ConfirmDeleteProfile for ONLY providers.openai.profiles.<name>; the AddProfile row still has no delete action
  #   4. profiles_config.rs gains a path-injectable full-config loader (load_openai_profile_configs_from) mirroring its display-string loader, returning name -> ProfileDefinition (project overrides user by name); customModels/unknown keys are not part of ProfileDefinition but are preserved by the backend read-modify-write (PROV-108)
  #   5. The ProviderSettingsView stores the per-profile full ProfileConfig keyed by profile name (folded in handle_provider_credentials_loaded) and exposes profile_config_for(name) so list_actions prefills the EditProfile form by the focused Profile row's name; the display string "{name} → {baseUrl}" is split back to the bare name for lookup
  #   6. After any successful save or delete, the existing PROV-109 dispatch refresh (list_provider_credentials -> ProviderCredentialsLoaded) re-loads BOTH the openai profile display slice and the per-profile config map so the affected row repaints end-to-end
  #   7. The existing PROV-102 test (Enter on a Profile -> Detail/Summary) is updated to assert the new EditProfile mode — an intentional, in-ACDD behavior change, not a regression
  #
  # EXAMPLES:
  #   1. User presses Enter on the 'fireworks' profile row -> EditProfile mode for 'fireworks' with baseUrl 'https://api.fireworks.ai/inference/v1' and apiKey prefilled, name 'fireworks' non-editable
  #   2. User presses Enter on '+ Add Profile', the create form opens with baseUrl 'http://localhost:8888', empty apiKey, name field active
  #   3. User presses 'd' on the 'home' profile row, accepts the confirm -> ConfirmDeleteProfile{provider 'openai', profile 'home'} is emitted
  #   4. load_openai_profile_configs_from over a user config with a fireworks profile (baseUrl + apiKey + contextWindow) returns {'fireworks': ProfileDefinition{...}} with the stored fields parsed
  #   5. Folding ProviderCredentialsLoaded stores the per-profile config map so profile_config_for('fireworks') returns the stored ProfileDefinition used to prefill the edit form
  #
  # QUESTIONS (ANSWERED):
  #   Q: Where does the edit-form prefill get the full ProfileConfig from, since the Rust nav tree only carries profile display strings (name -> baseUrl)?
  #   A: Load the full per-profile ProfileConfig when folding ProviderCredentialsLoaded, store it on the view keyed by profile name, and have list_actions look it up by the focused Profile row's name. profiles_config.rs gains a path-injectable full-config loader mirroring its display-string loader.
  #
  #   Q: Should create/edit/delete-profile be new ProviderSettingsMode variants?
  #   A: Yes — CreateProfile/EditProfile carrying the ProfileForm state (shipped in PROV-110). PROV-111 routes the nav rows into those modes and the delete-confirm into ConfirmDeleteProfile.
  #
  # NOTE ON SLICE BOUNDARIES (already covered, NOT re-tested here):
  #   - Backend openai-guard / missing-file / customModels+sibling preservation: PROV-108 (provider-config-profile-persistence.feature)
  #   - App::dispatch SaveProfile/DeleteProfile + status + list refresh: PROV-109 (provider-config-profile-dispatch.feature)
  #   - ProfileForm field navigation, save validation, parse-on-build, Tab/Esc: PROV-110 (provider-settings-profile-form.feature)
  #
  # ========================================

  Background: User Story
    As a fspec user managing OpenAI-compatible local-server profiles in Provider Settings
    I want to press Enter/d on profile and Add-Profile nav rows to open prefilled edit, create and delete-confirm flows
    So that I can manage my real profiles through an editable form and the row repaints after each write instead of hitting a dead read-only screen

  @PROV-111
  Scenario: Enter on a profile row opens the edit form prefilled from the stored config
    Given the Provider Settings nav tree has an expanded "openai" provider with a stored profile "fireworks"
    And the per-profile config map carries "fireworks" with baseUrl "https://api.fireworks.ai/inference/v1" and an apiKey
    And the cursor is on the "fireworks" profile row
    When I press Enter
    Then the view enters EditProfile mode for provider "openai" and profile "fireworks"
    And the form base URL is prefilled with "https://api.fireworks.ai/inference/v1"
    And the form api key is prefilled from the stored config
    And the form name editing flag is false so "fireworks" is not editable

  @PROV-111
  Scenario: Enter on the Add Profile row opens a create form with defaults
    Given the Provider Settings nav tree has an expanded "openai" provider with a trailing Add Profile row
    And the cursor is on the Add Profile row
    When I press Enter
    Then the view enters CreateProfile mode for provider "openai"
    And the form base URL defaults to "http://localhost:8888"
    And the form api key is empty
    And the form name editing flag is true

  @PROV-111
  Scenario: Pressing d on a profile row opens a per-profile delete confirm that targets only that profile
    Given the Provider Settings nav tree has an expanded "openai" provider with profiles "fireworks" and "home"
    And the cursor is on the "home" profile row
    When I press "d"
    Then a delete confirmation dialog is open
    When I accept the delete confirmation
    Then a ConfirmDeleteProfile action is emitted for provider "openai" and profile "home"

  @PROV-111
  Scenario: Pressing d on the Add Profile row has no delete action
    Given the Provider Settings nav tree has an expanded "openai" provider with a trailing Add Profile row
    And the cursor is on the Add Profile row
    When I press "d"
    Then no delete confirmation dialog is open
    And the key is consumed without emitting an action

  @PROV-111
  Scenario: The full-config loader returns parsed ProfileDefinitions keyed by name
    Given a user config fspec-config.json with an openai profile "fireworks" carrying a baseUrl, an apiKey and a contextWindow
    And an empty project config directory
    When load_openai_profile_configs_from is called with the user and project directories
    Then the result maps "fireworks" to a ProfileDefinition whose base URL, api key and context window match the stored values

  @PROV-111
  Scenario: The full-config loader merges with project overriding user by name
    Given a user config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://user.example/v1"
    And a project config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://project.example/v1"
    When load_openai_profile_configs_from is called with the user and project directories
    Then the result maps "fireworks" to a ProfileDefinition whose base URL is "https://project.example/v1"

  @PROV-111
  Scenario: Folding loaded credentials stores the per-profile config map for prefill
    Given a Provider Settings view
    When a per-profile config map containing "fireworks" is stored on the view
    Then profile_config_for("fireworks") returns the stored ProfileDefinition
    And profile_config_for("missing") returns nothing

  @PROV-111
  Scenario: The profile row display string is split back to the bare name for lookup
    Given a Provider Settings nav tree whose "openai" profile row label is "fireworks → https://api.fireworks.ai/inference/v1"
    And the per-profile config map carries "fireworks" with an apiKey
    And the cursor is on that profile row
    When I press Enter
    Then the view enters EditProfile mode for profile "fireworks"
    And the form is prefilled from the stored "fireworks" config
