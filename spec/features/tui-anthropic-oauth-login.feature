@PROV-025
Feature: TUI provider settings UX for Anthropic subscription connect and disconnect

  """
  PROVIDER_REGISTRY in provider-config.ts: change Anthropic entry authType from 'api-key' to 'oauth'. The isOAuthProvider() function then returns true for both 'codex' and 'anthropic'. All existing OAuth flow plumbing (nav items, expanded options, 'e'/'d' key handlers) will activate for Anthropic.
  useProviderSettingsState.ts reload() — for each OAuth provider, check tokens: Codex uses sync codexOauthGetTokens(), Anthropic uses async claudeOauthGetTokens(). Add a provider-keyed dispatch: if provider is 'anthropic', await the Claude async check. Add new NAPI imports: claudeOauthBrowserLogin, claudeOauthHeadlessStart, claudeOauthHeadlessComplete, claudeOauthGetTokens, claudeOauthClearTokens.
  New PanelMode variant: 'oauth-headless-code-entry' with { providerId, authorizeUrl, pkceVerifier, codeInput }. ProviderSettingsPanel renders this as: title 'Claude Headless Login', the authorize URL as a link, a text input for code#state, and hint text. oauthModeHandler.ts handles input: character typing into codeInput, Enter to submit (calls headless_complete), Esc to cancel.
  startBrowserLogin/startDeviceLogin/disconnectOauth in useProviderSettingsState need provider-specific dispatch: if providerId === 'codex' call codex_oauth_* bindings, if providerId === 'anthropic' call claude_oauth_* bindings. The startDeviceLogin for Anthropic becomes startHeadlessLogin — calls headless_start(), transitions to 'oauth-headless-code-entry' mode (not 'oauth-device-waiting').
  ProviderSettingsPanel.tsx rendering changes: (1) OAuth waiting screen title shows 'Claude OAuth Login' or 'Codex OAuth Login' based on providerId, (2) success screen shows '✓ Connected to Claude' or '✓ Connected to ChatGPT', (3) new oauth-headless-code-entry panel renders URL + text input. The nav item labels show 'Login with Claude (browser)' and 'Login with Claude (headless)' for Anthropic.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Anthropic provider registry entry must change authType from 'api-key' to 'oauth' — this makes isOAuthProvider('anthropic') return true, enabling OAuth login options in the expanded provider list alongside the existing API key edit option
  #   2. When user expands the Anthropic provider and no Claude OAuth tokens exist, two OAuth login options appear: 'Login with Claude (browser)' and 'Login with Claude (headless)' — mirroring PROV-017's Codex pattern but with Claude-specific labels
  #   3. Selecting 'Login with Claude (browser)' calls claude_oauth_browser_login() NAPI binding, shows spinner with 'Waiting for authorization...', and resolves when the browser flow completes (user pastes code#state into local form)
  #   4. Selecting 'Login with Claude (headless)' uses the two-phase NAPI flow: claude_oauth_headless_start() returns authorize_url and pkce_verifier, TUI displays URL and a text input for code#state paste, then claude_oauth_headless_complete(code_with_state, pkce_verifier) exchanges and persists tokens
  #   5. On successful OAuth login (browser or headless), the provider list reloads and Anthropic shows '✓ OAuth [Claude]' status with green checkmark — distinct from '✓ sk-ant-... [env]' API key status
  #   6. Pressing 'e' on Anthropic provider when it has OAuth tokens launches browser OAuth flow (not API key editor) — same fix pattern as PROV-019 rule [1] for Codex
  #   7. Pressing 'd' on Anthropic provider with OAuth tokens calls claude_oauth_clear_tokens() to disconnect — provider status reverts to '(not configured)' or shows API key status if one exists
  #   8. Escape during any OAuth waiting state (browser waiting, headless URL display, headless code entry) cancels the flow and returns to the provider list — uses the same oauthGeneration counter pattern from PROV-017
  #   9. Claude OAuth token check is async (claude_oauth_get_tokens returns Promise) unlike Codex (sync) — the reload() function in useProviderSettingsState must await Claude token check and handle the async NAPI boundary
  #   10. The headless flow needs a new PanelMode variant 'oauth-headless-code-entry' for displaying the authorize URL and collecting the pasted code#state — this is distinct from Codex's 'oauth-device-waiting' which just polls automatically
  #   11. Non-OAuth providers (OpenAI, Gemini, etc.) are completely unaffected — only Anthropic and Codex get OAuth login options. Anthropic with API key only (no OAuth tokens) still allows API key editing via 'e' key
  #   12. NAPI bindings must be rebuilt (napi build) before this card can work — codelet/napi/src/claude_oauth.rs exists from PROV-024 but index.d.ts does not yet export the claude_oauth_* functions
  #
  # EXAMPLES:
  #   1. User expands Anthropic provider with no OAuth tokens and no API key: sees 'Login with Claude (browser)', 'Login with Claude (headless)', and the existing 'edit API key' option (via 'e' key on provider row)
  #   2. User selects 'Login with Claude (browser)': screen shows spinner with 'Waiting for authorization...' text, browser opens to claude.ai/oauth/authorize, local form appears for code paste, user pastes code#state, tokens exchange, screen shows '✓ Connected to Claude' success message, provider list refreshes showing Anthropic as configured with OAuth
  #   3. User selects 'Login with Claude (headless)': screen shows authorize URL as clickable link, text input for code#state, user navigates to URL, authorizes on claude.ai, copies code#state, pastes into input, presses Enter, tokens exchange, screen shows success, provider list refreshes
  #   4. Browser OAuth times out after 5 minutes: user sees error message 'OAuth login timed out' with options to retry (Enter) or go back (Esc)
  #   5. User presses Escape while 'Waiting for authorization...' spinner is showing: flow is cancelled, screen returns to provider list with no error
  #   6. User opens provider settings, Anthropic already has OAuth tokens from previous login: Anthropic provider shows green checkmark with '✓ OAuth [Claude]' source label, no login options needed when expanded, can use 'e' to reconnect or 'd' to disconnect
  #   7. User has Anthropic API key via ANTHROPIC_API_KEY env var but no OAuth tokens: Anthropic shows '✓ sk-ant-... [env]' status. User expands and sees OAuth login options alongside existing env-based config. After OAuth login, status changes to '✓ OAuth [Claude]' (OAuth takes precedence for display)
  #   8. User presses 'd' on Anthropic with OAuth tokens: tokens are cleared, if API key exists provider reverts to showing API key status, if no API key shows '(not configured)', and OAuth login options reappear when expanded
  #   9. Headless code entry with invalid code#state (CSRF mismatch): user pastes code with wrong state, claude_oauth_headless_complete rejects with CSRF error, error screen shows 'CSRF validation failed', user can retry or go back
  #
  # ASSUMPTIONS:
  #   1. The NAPI .node binary needs rebuilding (napi build) before this card can be implemented — the Rust code from PROV-024 is written but index.d.ts does not yet export the claude_oauth functions
  #   2. Changing Anthropic authType to 'oauth' does not break existing API key functionality — isOAuthProvider affects only TUI login options and 'e'/'d' key behavior, not the underlying credential resolution in Rust
  #
  # ========================================

  Background: User Story
    As a user with a Claude Max/Pro subscription
    I want to connect and disconnect my Anthropic subscription from the TUI provider settings
    So that use my subscription for API calls without manually setting environment variables

  Scenario: Anthropic provider shows OAuth login options when no tokens exist
    Given the Anthropic provider has no OAuth tokens
    And the Anthropic provider has no API key configured
    When the user expands the Anthropic provider in provider settings
    Then the expanded list shows "Login with Claude (browser)" option
    And the expanded list shows "Login with Claude (headless)" option

  Scenario: Non-OAuth providers do not show OAuth login options
    Given the OpenAI provider has no API key configured
    When the user expands the OpenAI provider in provider settings
    Then the expanded list does not show any OAuth login options
    And the expanded list shows "Create new profile" option

  Scenario: Successful browser OAuth login flow
    Given the Anthropic provider has no OAuth tokens
    And the user has expanded the Anthropic provider
    When the user selects "Login with Claude (browser)"
    Then the screen shows "Claude OAuth Login" title
    And a spinner displays "Waiting for authorization..."
    When the browser OAuth flow completes successfully
    Then the screen shows "✓ Connected to Claude" success message
    And the Anthropic provider shows "✓ OAuth [Claude]" status

  Scenario: Successful headless OAuth login flow
    Given the Anthropic provider has no OAuth tokens
    And the user has expanded the Anthropic provider
    When the user selects "Login with Claude (headless)"
    Then the screen shows the authorize URL as a clickable link
    And a text input for "code#state" is displayed
    When the user pastes a valid code#state and presses Enter
    Then the tokens are exchanged successfully
    And the screen shows "✓ Connected to Claude" success message
    And the Anthropic provider shows "✓ OAuth [Claude]" status

  Scenario: Browser OAuth login times out after 5 minutes
    Given the Anthropic provider has no OAuth tokens
    And the user has started a browser OAuth login flow
    When the browser OAuth flow times out
    Then the screen shows an error message containing "timed out"
    And the user can press Enter to retry or Esc to go back

  Scenario: Escape cancels browser OAuth waiting state
    Given the user is on the browser OAuth waiting screen for Anthropic
    When the user presses Escape
    Then the screen returns to the provider list
    And no error message is shown

  Scenario: Escape cancels headless code entry state
    Given the user is on the headless code entry screen for Anthropic
    When the user presses Escape
    Then the screen returns to the provider list
    And no error message is shown

  Scenario: Anthropic provider shows connected status when OAuth tokens exist
    Given the Anthropic provider has valid OAuth tokens from a previous login
    When the user opens provider settings
    Then the Anthropic provider row shows "✓ OAuth [Claude]" status
    When the user expands the Anthropic provider
    Then no OAuth login options are shown in the expanded list

  Scenario: OAuth status takes precedence over API key status
    Given the Anthropic provider has a configured API key via ANTHROPIC_API_KEY env var
    And the Anthropic provider has no OAuth tokens
    When the user opens provider settings
    Then the Anthropic provider shows the masked API key status
    When the user expands the Anthropic provider
    Then OAuth login options are shown alongside existing config
    When the user completes an OAuth login flow
    Then the Anthropic provider status changes to "✓ OAuth [Claude]"

  Scenario: Edit key on Anthropic provider with OAuth tokens starts OAuth flow
    Given the Anthropic provider has valid OAuth tokens
    When the user presses "e" on the Anthropic provider row
    Then the browser OAuth flow starts
    And the API key editor is not shown

  Scenario: Disconnect OAuth clears tokens and reverts status
    Given the Anthropic provider has valid OAuth tokens
    And the Anthropic provider has no API key configured
    When the user presses "d" on the Anthropic provider row
    Then the Claude OAuth tokens are cleared
    And the Anthropic provider shows "(not configured)" status
    When the user expands the Anthropic provider
    Then OAuth login options reappear in the expanded list

  Scenario: Disconnect OAuth with existing API key reverts to API key status
    Given the Anthropic provider has valid OAuth tokens
    And the Anthropic provider has an API key via ANTHROPIC_API_KEY env var
    When the user presses "d" on the Anthropic provider row
    Then the Claude OAuth tokens are cleared
    And the Anthropic provider reverts to showing the masked API key status

  Scenario: Headless code entry with invalid CSRF state shows error
    Given the user is on the headless code entry screen for Anthropic
    When the user pastes a code#state with a mismatched state value
    Then the screen shows an error containing "CSRF validation failed"
    And the user can press Enter to retry or Esc to go back

  Scenario: Retry browser OAuth after error
    Given the user is on the OAuth error screen for Anthropic
    When the user presses Enter to retry
    Then the browser OAuth flow restarts from scratch
    And the waiting screen is shown again

  Scenario: Go back to provider list after OAuth error
    Given the user is on the OAuth error screen for Anthropic
    When the user presses Escape
    Then the screen returns to the provider list
    And no OAuth flow is running
