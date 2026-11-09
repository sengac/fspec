@done
@rust
@bug-fix
@provider-settings
@tui
@BUG-664
Feature: Provider Settings sign-in success uses the numbered refresh
  """
  codelet-fspec-tui: app/dispatch_provider_settings_oauth.rs handle_oauth_login_succeeded spawns the BUG-575 numbered refresh (spawn_provider_settings_refresh) with navigate_request(NavigateTarget::Provider). Action::ProviderCredentialsLoaded, App::handle_provider_credentials_loaded, ProviderSettingsView::set_navigate_target / apply_unnumbered_credentials and RefreshState::pending_navigate are deleted. Tests: MockBackend gated list_provider_credentials (call_gates) to hold refresh 1 open. No RPC wire change.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A successful sign-in refreshes Provider Settings through the numbered refresh (RefreshProviderSettings { navigate }) like every other write; the navigate target is the signed-in provider, captured with the current navigation epoch
  #   2. ProviderCredentialsLoaded, set_navigate_target and the unnumbered credentials fold are deleted (no legacy path)
  #
  # EXAMPLES:
  #   1. Refresh 1 is in flight when the anthropic sign-in succeeds; the sign-in's refresh 2 answers first with anthropic signed in, then refresh 1 answers with the old state: the view shows refresh 2's state and the cursor stays on the anthropic row
  #
  # ========================================
  Background: User Story
    As a user signing in to a provider from Provider Settings
    I want to see the newest provider state with the cursor on the provider I signed in to
    So that an older refresh still in flight never shows stale rows or moves my cursor

  Scenario: A sign-in success while an older refresh is in flight shows the newest state
    Given Provider Settings is open with anthropic signed out and a refresh of the provider settings in flight
    When the anthropic sign-in succeeds and its own refresh answers with anthropic signed in
    And the older refresh then answers with anthropic signed out
    Then the view shows anthropic signed in
    And the cursor is on the anthropic row

  Scenario: The old unnumbered navigate path is gone
    Given the fspec-tui source outside tests
    When it is scanned for the old sign-in navigate path
    Then it has no ProviderCredentialsLoaded, set_navigate_target, apply_unnumbered_credentials or pending_navigate
