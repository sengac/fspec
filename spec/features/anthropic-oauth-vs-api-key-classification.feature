@done
@provider-settings
@configuration
@tui
@PROV-099
Feature: Anthropic OAuth-vs-api-key classification in the provider settings projection

  """
  projection.rs::project_one: change `has_oauth_tokens = is_oauth && info.configured` to `is_oauth && info.configured && info.masked_key.is_none()`. masked_key=Some means env api key present (api-key config), so not OAuth-logged-in. No new wire field; projection reads existing ProviderCredentialInfo.masked_key.
  Offline tests: projection-layer tests are pure (hand-built ProviderCredentialInfo, no env/fs). No network.
  Regression guard: RPC-349 expanding_configured_oauth_provider_reveals_logout_and_login_rows feeds masked_key=None so the logout row still appears (still passes).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   3. Anthropic is classified as OAuth-logged-in (has_oauth_tokens=true, Logout row) only when it is configured WITHOUT an env api key (masked_key=None); a present env api key means has_oauth_tokens=false
  #   4. Anthropic always exposes its OAuth login rows AND the api-key row (requires_api_key=true), matching the TS screen that shows both
  #
  # EXAMPLES:
  #   4. project a configured anthropic with masked_key=Some -> has_oauth_tokens=false, oauth_status_label=None, requires_api_key=true
  #   5. project a configured anthropic with masked_key=None -> has_oauth_tokens=true, oauth_status_label=Some("Logout from OAuth [Anthropic]")
  #   6. project a configured anthropic with masked_key=Some -> oauth_login_methods is non-empty (browser + code) AND requires_api_key=true (both rows offered)
  #
  # QUESTIONS (ANSWERED):
  #   Q: When BOTH ANTHROPIC_API_KEY and a real OAuth auth file are present, should the logout row appear? (No dedicated wire field exists at projection layer to distinguish them.)
  #   A: Env api key wins: when both are present, masked_key=Some so has_oauth_tokens=false (api-key row, no logout). A fully correct both-present resolution would need a dedicated wire field and is out of scope for PROV-099.
  #
  # ASSUMPTIONS:
  #   1. Env api key wins: when both are present, masked_key=Some so has_oauth_tokens=false (api-key row, no logout). A fully correct both-present resolution would need a dedicated wire field and is out of scope for PROV-099.
  #
  # ========================================

  Background: User Story
    As a developer using the pure-Rust fspec Provider Settings screen
    I want to see my ANTHROPIC_API_KEY shown as a masked env-sourced api-key credential
    So that I am not wrongly told I am logged in via OAuth when I only set an env API key

  Scenario: Anthropic with an env API key is not classified as OAuth-logged-in
    Given a configured anthropic credential whose masked_key is Some
    When project_display_infos projects the credential
    Then the anthropic display info has_oauth_tokens is false
    And the anthropic display info oauth_status_label is None
    And the anthropic display info requires_api_key is true

  Scenario: Anthropic configured without an env API key shows the logout row
    Given a configured anthropic credential whose masked_key is None
    When project_display_infos projects the credential
    Then the anthropic display info has_oauth_tokens is true
    And the anthropic display info oauth_status_label is Some("Logout from OAuth [Anthropic]")

  Scenario: Anthropic offers both OAuth login rows and an api-key row
    Given a configured anthropic credential whose masked_key is Some
    When project_display_infos projects the credential
    Then the anthropic display info oauth_login_methods is non-empty
    And the anthropic display info requires_api_key is true
