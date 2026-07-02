@done
@configuration
@ts-parity
@provider-settings
@tui
@rust
@RPC-349
Feature: Provider Settings screen renders empty — rich RPC-103 nav tree never wired into live data path
  """
  Add a pure projection fn (e.g. views/provider_settings/projection.rs) mapping &[ProviderCredentialInfo] (+ openai profiles) -> Vec<ProviderDisplayInfo>; keep dispatch_provider_settings.rs under the 300-LoC ceiling (currently 282)
  handle_provider_credentials_loaded must call set_provider_display_infos(projection) so render_list takes the render_nav_items branch; keep set_providers() too if the legacy raw list is still consumed elsewhere (e.g. d/delete focus + visible_providers)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When the Provider Settings view loads credentials, the dispatch layer MUST project each ProviderCredentialInfo into a ProviderDisplayInfo and call set_provider_display_infos() (NOT only set_providers()), so the rich RPC-103 NavItem tree is rendered instead of the legacy flat list
  #   2. credential_type 'oauth' (provider ids anthropic, codex, github-copilot) projects is_oauth_provider=true; for those providers 'configured' maps to has_oauth_tokens and an oauth_status_label 'Logout from OAuth [name]' plus the provider's oauth_login_methods are emitted on expansion
  #   3. Non-openai, non-oauth providers project requires_api_key=true so an ApiKey edit row appears beneath them when expanded; openai never gets an ApiKey row
  #   4. The openai provider projects its profiles from load_local_server_profiles(); when expanded it shows one Profile row per profile followed by a trailing AddProfile row (always present)
  #   5. Provider rows are collapsed by default; the header 'Provider Settings (N items)' count equals nav_items.len(), which grows when a provider is expanded and shrinks under an active filter (parent-anchored)
  #
  # EXAMPLES:
  #   1. Backend returns [openai, anthropic, gemini]; opening /provider renders three collapsed provider rows via the nav tree (header shows '3 items'), NOT the legacy flat list or '(no providers configured)'
  #   2. Expanding the 'gemini' provider (api_key type) reveals an ApiKey edit row beneath it; pressing Enter on that row opens the EditApiKey detail sub-view
  #   3. Expanding 'anthropic' (oauth, configured) reveals a 'Logout from OAuth [Anthropic]' row plus its OAuth login option rows; an uncredentialed oauth provider shows only the login rows
  #   4. Expanding 'openai' with two saved profiles reveals two Profile rows followed by an 'Add Profile' row, and no ApiKey row
  #
  # ASSUMPTIONS:
  #   1. OAuth login method labels have no Rust registry yet (TS uses getOauthProviderLabels/buildOauthLoginNavItems). Per-provider defaults: anthropic -> Browser + Headless; codex -> Browser + Device; github-copilot -> Device. Mirror TS labels in the projection helper.
  #
  # ========================================
  Background: User Story
    As a Codelet TUI user
    I want to open the Provider Settings (/provider) screen and see each provider expand into its real settings (API-key entry, OAuth login/logout, OpenAI profiles)
    So that I can configure my providers from the Rust TUI exactly as I can in the TypeScript version

  Scenario: Loading credentials populates the rich nav tree, not the legacy flat list
    Given the backend returns provider credentials for "openai", "anthropic", and "gemini"
    When the Provider Settings view folds the loaded credentials
    Then the view's nav tree contains three collapsed provider rows
    And the header item count reports 3 items
    And the legacy "(no providers configured)" placeholder is not used

  Scenario: Expanding an api-key provider reveals an editable API-key row
    Given the Provider Settings view has loaded a "gemini" provider of credential type "api_key"
    When the "gemini" provider row is expanded
    Then an ApiKey row appears beneath the "gemini" provider
    When Enter is pressed on the ApiKey row
    Then the view enters the EditApiKey detail sub-view for "gemini"

  Scenario: Expanding a configured OAuth provider reveals logout and login rows
    Given the Provider Settings view has loaded an "anthropic" provider of credential type "oauth" that is configured
    When the "anthropic" provider row is expanded
    Then a "Logout from OAuth [Anthropic]" row appears beneath the "anthropic" provider
    And one or more OAuth login rows appear beneath the "anthropic" provider

  Scenario: Expanding an uncredentialed OAuth provider reveals only login rows
    Given the Provider Settings view has loaded a "codex" provider of credential type "oauth" that is not configured
    When the "codex" provider row is expanded
    Then no logout row appears beneath the "codex" provider
    And one or more OAuth login rows appear beneath the "codex" provider

  Scenario: Expanding the openai provider reveals profiles and an add-profile row
    Given the Provider Settings view has loaded an "openai" provider with profiles "fast" and "local"
    When the "openai" provider row is expanded
    Then a Profile row appears for "fast"
    And a Profile row appears for "local"
    And a trailing "Add Profile" row appears beneath the "openai" profiles
    And no ApiKey row appears beneath the "openai" provider

  Scenario: Header item count grows when a provider is expanded
    Given the Provider Settings view has loaded a "gemini" provider of credential type "api_key"
    And the header item count reports 1 item
    When the "gemini" provider row is expanded
    Then the header item count reports 2 items
