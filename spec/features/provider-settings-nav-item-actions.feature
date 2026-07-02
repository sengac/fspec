@done
@agent-view
@provider-settings
@rust
@ts-parity
@provider
@tui
@bug
@PROV-102
Feature: Provider Settings nav-item action dispatch
  """
  Fix lives in views/provider_settings/list.rs (Enter + d arms) delegating to a new list_actions.rs that dispatches by focused_nav_item().kind. The mismatched visible_providers()[selected_index] path is reached ONLY when nav_items is empty. The delete-confirm Primary path in mod.rs uses a new nav_tree_ops::delete_target_provider_id() that prefers the focused NavItem's provider_id.
  Parity gap (documented, not fabricated): the Rust frontend has no profile-create, profile-edit, OAuth-login, OAuth-disconnect or per-profile-delete modes. AddProfile Enter and d on Profile/AddProfile/OAuthLogin are explicit no-ops; OAuthLogin/OAuthStatus Enter route to the honest OAuthNotice placeholder. These rows still use the correct provider_id so no mismatched-index path remains. Wiring the real flows is out of scope (separate cards).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When nav_items is populated, list-mode Enter and d dispatch by the focused NavItem's kind and its own provider_id, never by indexing visible_providers() with selected_index
  #   2. Enter on a Profile row opens that profile's provider Detail (Summary) view keyed by the row's own provider_id
  #   3. Enter on an OAuthLogin or OAuthStatus row opens the OAuth notice keyed by the row's own provider_id (OAuth login/disconnect not yet implemented in the Rust frontend)
  #   4. Enter on an AddProfile row is consumed without opening any Detail view (the Rust frontend has no profile-creation flow yet)
  #   5. The opened Detail view's provider_id always equals the focused NavItem's provider_id, never re-derived from selected_index
  #   6. Pressing d on a Provider, ApiKey, or OAuthStatus row opens the delete-credentials confirm for the focused NavItem's own provider_id, and accepting it deletes that provider
  #   7. Pressing d on a Profile, AddProfile, or OAuthLogin row is consumed without opening the delete confirm
  #   8. When nav_items is empty (legacy set_providers-only callers), Enter and d retain their pre-existing visible_providers behavior
  #
  # EXAMPLES:
  #   1. With openai, anthropic, gemini loaded and openai expanded, pressing Enter on the openai 'fast' profile row opens Detail for provider_id openai (not anthropic, which is registry index 1)
  #   2. With anthropic expanded among multiple providers, pressing Enter on its OAuthStatus row opens Detail OAuthNotice for anthropic (not gemini, which selected_index would index in visible_providers)
  #   3. With anthropic expanded, pressing Enter on an OAuthLogin row opens Detail OAuthNotice for anthropic (old code returned None here and silently did nothing)
  #   4. Pressing Enter on the openai AddProfile row leaves the view in List mode and opens no Detail view
  #   5. Pressing d on the anthropic OAuthStatus row then accepting the confirm emits ConfirmDeleteProviderCredentials for anthropic (not gemini)
  #   6. Pressing d on the openai 'fast' profile row leaves delete_confirm closed (no per-profile delete in the Rust frontend)
  #   7. With only set_providers used (nav_items empty), Enter on an api_key provider still transitions to Detail Summary and d on a configured provider still opens the delete confirm
  #
  # ========================================
  Background: User Story
    As a fspec TUI user navigating Provider Settings
    I want to press Enter or d on any expanded child row
    So that the action targets the row's own provider, never a mismatched provider derived from the cursor index

  Scenario: Enter on an OpenAI profile row opens the OpenAI edit form, not Anthropic
    Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    And the openai provider is expanded so its "fast" profile row is visible
    And the cursor is on the openai "fast" profile row
    When I press Enter
    Then the EditProfile form that opens has provider_id "openai"
    And the form is prefilled from the stored "fast" config
    And the form's provider_id is not "anthropic"

  Scenario: Enter on an OAuthStatus row opens the notice for its own provider, not a mismatched one
    Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    And the anthropic provider is expanded so its OAuthStatus row is visible
    And the cursor is on the anthropic OAuthStatus row
    When I press Enter
    Then the Detail view that opens has provider_id "anthropic"
    And the Detail view is the OAuthNotice sub-view
    And the Detail view's provider_id is not "gemini"

  Scenario: Enter on an OAuthLogin row opens the notice instead of silently doing nothing
    Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    And the anthropic provider is expanded so its OAuthLogin row is visible
    And the cursor is on an anthropic OAuthLogin row
    When I press Enter
    Then the Detail view that opens has provider_id "anthropic"
    And the Detail view is the OAuthNotice sub-view

  Scenario: Enter on the AddProfile row opens the create form
    Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    And the openai provider is expanded so its AddProfile row is visible
    And the cursor is on the openai AddProfile row
    When I press Enter
    Then the view enters CreateProfile mode for "openai"

  Scenario: d on an OAuthStatus row deletes that provider, not a mismatched one
    Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    And the anthropic provider is expanded so its OAuthStatus row is visible
    And the cursor is on the anthropic OAuthStatus row
    When I press d
    And I accept the delete confirmation
    Then the emitted action is ConfirmDeleteProviderCredentials for "anthropic"
    And the emitted action is not for "gemini"

  Scenario: d on a profile row opens the per-profile delete confirm
    Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    And the openai provider is expanded so its "fast" profile row is visible
    And the cursor is on the openai "fast" profile row
    When I press d
    Then a per-profile delete confirmation dialog is open
    And accepting it emits ConfirmDeleteProfile for openai profile "fast"

  Scenario: Legacy set_providers callers keep their pre-existing Enter and d behavior
    Given a Provider Settings view populated only via set_providers with a configured api_key provider
    And the nav tree is empty
    When I press Enter on the provider row
    Then the Detail view that opens is the Summary sub-view for that provider
    When I press d on the configured provider row
    Then a delete confirmation dialog is open
