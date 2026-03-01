@done
@PROV-017
Feature: TUI OAuth Login Flow for Provider Settings
  """
  New oauth input handler file: src/tui/inputHandlers/oauthModeHandler.ts — handles keyboard input for oauth-method-select (arrow selection, Enter to pick), oauth-browser-waiting (Esc to cancel), oauth-device-waiting (Esc to cancel), oauth-error (Enter to retry, Esc to go back), oauth-success (Enter/Esc to go back to list)
  providerSettingsModeMapper.ts needs new mappings for oauth-method-select, oauth-browser-waiting, oauth-device-waiting, oauth-success, and oauth-error hook modes → corresponding PanelMode variants. OAuth state (oauthStatus, oauthError, oauthUserCode, oauthVerificationUrl) stored in useProviderSettingsState
  Integration points: (1) useProviderSettingsInput.ts dispatches to new oauthModeHandler, (2) useProviderSettingsState.ts adds startBrowserLogin(), startDeviceLogin(), cancelOauth() operations that call NAPI bindings, (3) ProviderSettingsPanel.tsx renders OAuth views, (4) Provider reload detects OAuth tokens via codex_oauth_get_tokens() during provider config loading
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When user selects the codex provider and no OAuth tokens exist, a 'Login with ChatGPT' option appears in the provider list alongside the existing API key edit option
  #   2. Selecting 'Login with ChatGPT (browser)' calls codex_oauth_browser_login() NAPI binding, shows a spinner with 'Waiting for authorization...', and resolves when the browser callback completes
  #   3. Selecting 'Login with ChatGPT (headless)' calls codex_oauth_device_login_start() to get user_code and verification_url, displays them to the user, then calls codex_oauth_device_login_poll() which resolves when the user completes auth on another device
  #   4. On successful OAuth login (browser or device), the provider list reloads and the codex provider shows as configured with a green checkmark
  #   5. On OAuth failure (timeout, network error, user cancel), an error message is shown with the option to retry or go back to the provider list
  #   6. Pressing Escape during an OAuth waiting state cancels the flow and returns to the provider list
  #   7. If OAuth tokens already exist (codex_oauth_get_tokens returns non-null), the codex provider shows as configured and no login option is needed
  #   8. The OAuth login option is codex-specific — only the codex provider shows OAuth login methods; other providers continue to use only API key entry and profiles
  #   9. New PanelMode variants are needed: oauth-method-select, oauth-browser-waiting, oauth-device-waiting, oauth-success, oauth-error — following the existing mode pattern in ProviderSettingsPanel
  #   10. NAPI bindings must be rebuilt (napi build) so that codex_oauth_browser_login, codex_oauth_device_login_start, codex_oauth_device_login_poll, codex_oauth_refresh_token, and codex_oauth_get_tokens appear in index.d.ts and are importable from @sengac/codelet-napi
  #
  # EXAMPLES:
  #   1. User expands codex provider with no OAuth tokens and no API key: sees 'Login with ChatGPT (browser)', 'Login with ChatGPT (headless)', and the existing 'edit API key' option in the expanded list
  #   2. User selects 'Login with ChatGPT (browser)': screen shows spinner with 'Waiting for authorization...' text, browser opens to auth URL, user authorizes in browser, callback completes, screen shows '✓ Connected to ChatGPT' success message, provider list refreshes showing codex as configured
  #   3. User selects 'Login with ChatGPT (headless)': screen shows user_code 'ABCD-1234' and verification URL 'https://auth.openai.com/codex/device' with spinner and 'Enter the code on another device' text, user completes auth on phone, polling resolves, screen shows success, provider list refreshes
  #   4. Browser OAuth times out after 5 minutes: user sees error message 'OAuth login timed out' with options to retry (Enter) or go back (Esc)
  #   5. User presses Escape while 'Waiting for authorization...' spinner is showing: flow is cancelled, screen returns to provider list with no error
  #   6. User opens provider settings, codex already has OAuth tokens from previous login: codex provider shows green checkmark with 'OAuth' source label, no login options needed, can use 'e' to edit API key or 'd' to remove credentials
  #   7. User selects 'Login with ChatGPT (browser)' on error screen (retry): flow restarts from scratch, new spinner shown, new browser tab opens
  #
  # ASSUMPTIONS:
  #   1. The NAPI .node binary needs rebuilding (napi build) before this card can be implemented — the Rust code from PROV-015 is written but index.d.ts does not yet export the codex_oauth functions
  #
  # ========================================
  Background: User Story
    Given I am on the provider settings screen

  # Example [0]: Codex provider expanded with no tokens
  # Rules: [0], [7], [8]
  Scenario: Codex provider shows OAuth login options when no tokens exist
    Given the codex provider has no OAuth tokens
    And the codex provider has no API key configured
    When I expand the codex provider
    Then I should see "Login with ChatGPT (browser)" option
    And I should see "Login with ChatGPT (headless)" option
    And I should see the existing API key edit option

  # Example [0]: Other providers unaffected
  # Rules: [7]
  Scenario: Non-codex providers do not show OAuth login options
    Given the anthropic provider has no API key configured
    When I expand the anthropic provider
    Then I should not see any "Login with ChatGPT" options
    And I should see the existing API key edit option

  # Example [1]: Browser OAuth happy path
  # Rules: [1], [3]
  Scenario: Successful browser OAuth login flow
    Given the codex provider has no OAuth tokens
    When I select "Login with ChatGPT (browser)"
    Then I should see a spinner with "Waiting for authorization..." text
    When the browser OAuth callback completes successfully
    Then I should see "Connected to ChatGPT" success message
    And the provider list should reload
    And the codex provider should show as configured with a green checkmark

  # Example [2]: Device auth happy path
  # Rules: [2], [3]
  Scenario: Successful device auth login flow
    Given the codex provider has no OAuth tokens
    When I select "Login with ChatGPT (headless)"
    Then I should see the user code displayed
    And I should see the verification URL displayed
    And I should see a spinner with "Enter the code on another device" text
    When the device auth polling completes successfully
    Then I should see a success message
    And the provider list should reload
    And the codex provider should show as configured with a green checkmark

  # Example [3]: Browser OAuth timeout
  # Rules: [4]
  Scenario: Browser OAuth login times out after 5 minutes
    Given the codex provider has no OAuth tokens
    When I select "Login with ChatGPT (browser)"
    And the browser OAuth flow times out
    Then I should see an error message containing "timed out"
    And I should see instructions to retry with Enter or go back with Escape

  # Example [4]: Escape cancels OAuth flow
  # Rules: [5]
  Scenario: Escape cancels browser OAuth waiting state
    Given the codex provider has no OAuth tokens
    And I have started the browser OAuth flow
    And I see the "Waiting for authorization..." spinner
    When I press Escape
    Then the OAuth flow should be cancelled
    And I should return to the provider list
    And no error message should be displayed

  # Example [4]: Escape cancels device auth flow
  # Rules: [5]
  Scenario: Escape cancels device auth waiting state
    Given the codex provider has no OAuth tokens
    And I have started the device auth flow
    And I see the user code and verification URL
    When I press Escape
    Then the OAuth flow should be cancelled
    And I should return to the provider list
    And no error message should be displayed

  # Example [5]: Existing tokens detected
  # Rules: [6]
  Scenario: Codex provider shows as configured when OAuth tokens exist
    Given the codex provider has existing OAuth tokens
    When the provider settings screen loads
    Then the codex provider should show a green checkmark
    And the codex provider should show "OAuth" as the source label
    And no OAuth login options should be displayed in the expanded list

  # Example [6]: Retry after error
  # Rules: [4]
  Scenario: Retry browser OAuth after error
    Given the codex provider has no OAuth tokens
    And I am on the OAuth error screen after a failed browser login
    When I press Enter to retry
    Then the browser OAuth flow should restart
    And I should see a spinner with "Waiting for authorization..." text

  # Example [6]: Go back after error
  # Rules: [4]
  Scenario: Go back to provider list after OAuth error
    Given the codex provider has no OAuth tokens
    And I am on the OAuth error screen after a failed browser login
    When I press Escape
    Then I should return to the provider list
    And no error message should be displayed

  # Integration: NAPI bindings availability
  # Rules: [9]
  @integration
  Scenario: NAPI codex OAuth bindings are importable
    Given the NAPI module has been rebuilt
    Then codex_oauth_browser_login should be available as a function
    And codex_oauth_device_login_start should be available as a function
    And codex_oauth_device_login_poll should be available as a function
    And codex_oauth_get_tokens should be available as a function
    And codex_oauth_refresh_token should be available as a function
