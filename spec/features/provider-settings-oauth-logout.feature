@done
@provider-settings
@tui
@PROV-035
Feature: Replace OAuth status echo with actionable Logout line in Provider Settings
  """
  No new types, modes, or components needed. Changes are confined to: label text in buildNavItems(), Enter handler in listModeHandler, footer string in getFooterHints(), and optional color tweak in ProviderSettingsPanel renderer. The SettingsNavItem type, HookMode, PanelMode, disconnect-oauth confirmation dialog, and deleteConfirmModeHandler all remain unchanged.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The oauth-status label must read 'Logout from OAuth [<brand>]' where brand is Claude or ChatGPT
  #   2. Pressing Enter on the oauth-status row must trigger the disconnect-oauth confirmation dialog
  #   3. The 'd' keybind on oauth-status must continue to work (backward compat) but is no longer advertised in the footer
  #   4. Footer hint for oauth-status must show 'Enter: logout' instead of 'd: disconnect'
  #   5. Applies to all OAuth providers: Anthropic (Claude) and Codex (ChatGPT)
  #   6. The oauth-status row only appears when provider.hasOAuthTokens is true (no change to this condition)
  #   7. The disconnect-oauth confirmation dialog ('Disconnect Claude/ChatGPT OAuth? (y/n)') is unchanged — only how it's triggered changes
  #
  # EXAMPLES:
  #   1. Anthropic with OAuth connected: expanding shows 'Logout from OAuth [Claude]' as first child item
  #   2. Codex with OAuth connected: expanding shows 'Logout from OAuth [ChatGPT]' as first child item
  #   3. User presses Enter on 'Logout from OAuth [Claude]' → disconnect confirmation dialog appears
  #   4. User presses 'd' on 'Logout from OAuth [Claude]' → disconnect confirmation dialog appears (backward compat)
  #   5. Footer shows 'Enter: logout · / filter · Tab: Switch to models · Esc: close' when oauth-status is selected
  #   6. Provider without OAuth (e.g. Gemini) has no oauth-status row when expanded — no change
  #   7. OAuth provider without tokens (not logged in) has no oauth-status row — only login options shown
  #
  # ========================================
  Background: User Story
    Given the Provider Settings TUI is open

  # --- OAuth status label ---
  Scenario: Expanding Anthropic with OAuth shows logout line
    Given Anthropic has valid OAuth tokens connected
    When I expand the Anthropic provider
    Then I see "Logout from OAuth [Claude]" as a child item

  Scenario: Expanding Codex with OAuth shows logout line
    Given Codex (ChatGPT) has valid OAuth tokens connected
    When I expand the Codex provider
    Then I see "Logout from OAuth [ChatGPT]" as a child item

  # --- Enter triggers disconnect ---
  Scenario: Enter on logout line triggers disconnect confirmation
    Given Anthropic has valid OAuth tokens connected
    And I have the cursor on "Logout from OAuth [Claude]"
    When I press Enter
    Then a confirmation dialog appears: "Disconnect Claude OAuth? (y/n)"

  # --- Backward compat: 'd' still works ---
  Scenario: Pressing 'd' on logout line triggers disconnect confirmation
    Given Anthropic has valid OAuth tokens connected
    And I have the cursor on "Logout from OAuth [Claude]"
    When I press "d"
    Then a confirmation dialog appears: "Disconnect Claude OAuth? (y/n)"

  # --- Footer hint ---
  Scenario: Footer shows Enter logout hint when logout line is selected
    Given Anthropic has valid OAuth tokens connected
    And I have the cursor on "Logout from OAuth [Claude]"
    Then the footer shows "Enter: logout · / filter · Tab: Switch to models · Esc: close"

  # --- Negative cases: no change ---
  Scenario: Non-OAuth provider has no logout line when expanded
    Given Google Gemini has an API key configured
    When I expand the Google Gemini provider
    Then I do NOT see any OAuth logout or status items

  Scenario: OAuth provider without tokens has no logout line
    Given Anthropic has no OAuth tokens stored
    When I expand the Anthropic provider
    Then I do NOT see a logout line
    And I see OAuth login options for Claude
