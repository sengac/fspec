@PROV-028
Feature: OAuth TUI broken flows — Claude browser auth stuck, Codex empty expansion, parent card housekeeping
  """
  BUG 1 fix: claude_oauth_server.rs:118 — change open::that(&auth_url) to open::that(format!("http://localhost:{port}/")). BUG 2 fix: useProviderSettingsState.ts buildNavItems() — when OAuth provider has tokens, show oauth-status + re-login items. BUG 3 fix: ProviderSettingsPanel.tsx — ensure status text renders inside wrap=truncate when expanded. BUG 7 fix: ProviderSettingsPanel.tsx:403-408 — wrap code input in width-constrained Box. BUG 8 fix: oauthModeHandler.ts — add 'c' for clipboard copy and 'o' for browser open keybinds.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Claude browser OAuth must open the local form page (http://localhost:{port}/) not the Anthropic authorize URL directly
  #   2. Expanded OAuth provider with tokens must show sub-items: OAuth status info and re-login options
  #   3. Provider status indicator (checkmark + masked key + source) must remain visible when provider is expanded
  #   4. Headless code input must be width-constrained to prevent layout overflow with long OAuth codes (80-200+ chars)
  #   5. Headless mode must provide a way to copy or open the authorize URL (keybind c to copy, o to open browser)
  #   6. Parent cards PROV-011 and PROV-012 must be advanced to done when all children are complete
  #   7. User-Agent version in PROV-020 spec must match code constant (2.1.3 not 2.1.2)
  #
  # EXAMPLES:
  #   1. User clicks 'Login with Claude (browser)' → browser opens http://localhost:{port}/ showing the paste form → user clicks auth link → authorizes → pastes code#state → tokens exchanged
  #   2. User expands Codex (ChatGPT) with OAuth tokens → sees 'OAuth [ChatGPT]' status info item + 'Login with ChatGPT (browser)' + 'Login with ChatGPT (headless)' re-login options
  #   3. User expands Codex (ChatGPT) with OAuth tokens and selects the provider row → status indicator '✓ OAuth [ChatGPT]' remains visible on the provider row
  #   4. User pastes 150-char OAuth code into headless code entry field → text is width-constrained and does not overflow the terminal width
  #   5. User presses 'c' in headless code entry screen → authorize URL is copied to clipboard. User presses 'o' → browser opens the authorize URL.
  #   6. PROV-012 parent with all 8 children done → advance through workflow to done status
  #
  # ========================================
  Background: User Story
    As a user
    I want to complete OAuth login flows for both Claude and Codex providers without encountering broken UI or dead-end states
    So that I can authenticate and use my subscription without frustration

  @critical
  Scenario: Expanded OAuth provider with tokens shows status and re-login options
    Given the Codex (ChatGPT) provider has valid OAuth tokens
    And the provider list is rendered
    When I expand the Codex provider
    Then I should see an OAuth status info item showing "✓ OAuth [ChatGPT]"
    And I should see a "Login with ChatGPT (browser)" re-login option
    And I should see a "Login with ChatGPT (headless)" re-login option

  Scenario: Provider status indicator remains visible when expanded
    Given the Codex (ChatGPT) provider has valid OAuth tokens
    And the provider list is rendered showing status "✓ OAuth [ChatGPT]"
    When I expand the Codex provider
    Then the provider row should still display "✓ OAuth [ChatGPT]" status text
    And the status text should be visible regardless of selection state

  Scenario: Headless code input is width-constrained for long OAuth codes
    Given I am on the Claude headless login screen
    And the terminal width is 80 columns
    When I paste an OAuth code that is 150 characters long
    Then the code input rendering should use a width-constrained container

  Scenario: Headless mode provides keybinds to copy and open the authorize URL
    Given I am on the Claude headless login screen
    And the authorize URL is displayed
    When I press "c"
    Then the authorize URL should be copied to the system clipboard
    When I press "o"
    Then the authorize URL should be opened in the default browser
    And the hint text should show "c: copy URL" and "o: open URL" keybinds
