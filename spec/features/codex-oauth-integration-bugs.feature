@PROV-019
Feature: Codex OAuth Integration Broken - Token Expiry, Provider Settings UX, and Session Routing Failures

  """
  Rust: mod.rs passes Some(0) to force immediate refresh, refreshing_client.rs handles token lifecycle. TS: listModeHandler.ts and ProviderSettingsView.tsx route 'e' key for OAuth providers. useProviderSettingsState.ts builds provider status display.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tokens loaded from disk must trigger immediate refresh — expires_in=0 forces refresh on first API call
  #   2. Pressing 'e' on Codex provider must launch OAuth flow, never show API key editor
  #   3. Pressing 'd' on Codex provider with OAuth tokens must disconnect OAuth (not delete API key)
  #   4. Codex provider with OAuth tokens must show '✓ OAuth [ChatGPT]' status, not '(not configured)'
  #   5. Non-OAuth providers (e.g., Anthropic, OpenAI) must still show API key editor on 'e' press
  #
  # EXAMPLES:
  #   1. CodexProvider loads week-old tokens from auth.json, passes Some(0) to RefreshingCodexClient, first API call triggers refresh, gets 200 OK
  #   2. User presses 'e' on Codex provider row — browser OAuth flow starts, not API key editor form
  #   3. User presses 'e' on Anthropic provider row — API key editor form shows as usual
  #   4. Codex with valid OAuth tokens shows '✓ OAuth [ChatGPT]' on the collapsed provider row
  #   5. User presses 'd' on Codex provider with OAuth tokens — tokens are cleared and provider shows '(not configured)'
  #
  # ========================================

  Background: User Story
    As a developer
    I want to use Codex OAuth integration without 401 errors, API key UX confusion, or missing status indicators
    So that I can actually use the ChatGPT Codex provider end-to-end

  Scenario: Edit key on Codex provider starts OAuth flow
    Given the Codex provider is selected in provider settings
    When the user presses 'e'
    Then the browser OAuth login flow starts
    Then the API key editor form is not shown


  Scenario: Edit key on non-OAuth provider shows API key editor
    Given a non-OAuth provider like Anthropic is selected in provider settings
    When the user presses 'e'
    Then the API key editor form is shown


  Scenario: Codex provider with OAuth tokens shows connected status
    Given the Codex provider has valid OAuth tokens stored
    When the provider settings list is rendered
    Then the Codex row displays a checkmark with 'OAuth' and '[ChatGPT]' source


  Scenario: Delete on Codex provider disconnects OAuth
    Given the Codex provider has OAuth tokens stored
    When the user presses 'd'
    Then the OAuth tokens are cleared from storage
    Given the Codex provider is selected in provider settings
    Then the provider shows '(not configured)' status

