@PROV-027
Feature: Anthropic provider settings regression hardening

  """
  REGRESSION tests preventing PROV-019 class bugs for Claude provider settings.
  TS tests in src/tui/ for provider settings UX: edit triggers OAuth flow,
  delete disconnects OAuth, and connected status indicator displays correctly.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Provider settings must show OAuth-specific UX for Claude subscription
  #   2. Edit action must start OAuth flow, not API key editor
  #   3. Delete action must disconnect OAuth, not delete API key
  #
  # ASSUMPTIONS:
  #   1. This card is pure TESTING — regression tests against existing code from PROV-025
  #
  # ========================================

  Background: User Story
    As a developer with a Claude Max/Pro subscription
    I want provider settings to correctly handle OAuth connect and disconnect
    So that I do not encounter the same UX bugs as PROV-019 for Codex

  @regression @provider-settings
  Scenario: Edit action on Claude OAuth provider starts OAuth flow
    Given the Claude provider is selected in provider settings
    And the Claude provider has OAuth tokens stored
    When the user presses 'e'
    Then the OAuth login flow starts (browser or headless)
    And the API key editor form is not shown

  @regression @provider-settings
  Scenario: Delete action on Claude OAuth provider disconnects OAuth
    Given the Claude provider has OAuth tokens stored
    When the user presses 'd'
    Then the OAuth tokens are cleared from claude_auth.json
    And the provider shows "(not configured)" status

  @regression @provider-settings
  Scenario: Claude provider with OAuth tokens shows connected status
    Given the Claude provider has valid OAuth tokens stored
    When the provider settings list is rendered
    Then the Claude row displays a checkmark with "OAuth" and "[Claude]" source
