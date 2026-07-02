@done
@rust
@ts-parity
@oauth
@provider-settings
@tui
@PROV-113
Feature: Anthropic + Codex OAuth login wiring (browser, headless, device)
  """
  Builds on PROV-112's OAuth boundary (FspecBackend OAuth trait methods, codelet-providers-direct via embedded; websocket no-op defaults). Adds ProviderSettingsMode variants OAuthBrowserWaiting{provider_id}, OAuthDeviceWaiting{provider_id,user_code,verification_url}, OAuthHeadlessCodeEntry{provider_id,authorize_url,pkce_verifier,code_input}, OAuthSuccess{provider_id}, OAuthError{provider_id,error} (replacing DetailSub::OAuthNotice for login rows), plus their sub-handlers + renderers. Backend methods forward to codelet-providers: claude_oauth_server::claude_browser_oauth_login, claude_oauth headless start (sync) + complete (async), codex codex_oauth_server::browser_oauth_login, codex_device_auth start + poll. Dispatch via dispatch_provider_settings_oauth.rs (spawn→backend→status→list_provider_credentials refresh). list_actions.rs Enter on OauthLogin routes by method (browser/headless) and provider; success reload returns cursor to provider row. Generation counter (per-flow monotonically increasing) invalidates stale results after Esc/cancel. Browser rows gated to embedded transport. Offline tests: MockBackend with scripted Ok/Err + call counters; view/key tests drive handle_key; no real OAuth network or ~/.fspec mutation. Files <300 LoC; clippy -D warnings + fmt clean; NO git; do not touch user WIP. Reference: spec/attachments/PROV-105/oauth-parity-spec.md §3,§4.1,§4.2,§6; napi/src/{claude,codex}_oauth.rs for exact flow semantics.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Enter on a browser login row (anthropic or codex) enters oauth-browser-waiting showing the provider's browserWaitingTitle + 'Waiting for authorization...' + 'Press Esc to cancel'; the napi browser login runs fire-and-forget
  #   2. Enter on anthropic headless row enters oauth-headless-code-entry: synchronous claude headless-start yields authorizeUrl+pkceVerifier shown immediately with a 'Code:' input; when codeInput empty 'c' copies URL and 'o' opens URL; once non-empty, c/o append as normal chars; Enter submits only when codeInput non-empty; Backspace deletes; Esc cancels
  #   3. Enter on codex headless row enters oauth-device-waiting showing deviceWaitingTitle + 'Your code: <userCode>' + verification URL + spinner + 'Press Esc to cancel'; device-start then device-poll run async; Esc cancels
  #   4. On any login success the mode becomes oauth-success showing the provider successLabel ('✓ Connected to Claude'/'✓ Connected to ChatGPT') + 'Press Enter or Esc to continue', then nav reloads; Enter or Esc returns to list
  #   5. On any login error the mode becomes oauth-error showing 'OAuth Login error' + the error message + 'Press Enter to retry | Esc to go back'; Enter retries the last method (browser→startBrowserLogin, device→startDeviceLogin), Esc cancels to list; all other keys absorbed
  #   6. A generation counter invalidates in-flight logins: pressing Esc during waiting/entry increments the generation; a success or error result whose generation no longer matches is dropped and does not change the mode away from list
  #   7. Browser login rows are only available on the embedded transport (the napi local HTTP server must run on the user's machine); on non-embedded transport the browser rows are gated/disabled while headless/device rows remain available
  #
  # EXAMPLES:
  #   1. User on 'Login with Claude (browser)' presses Enter → 'Claude OAuth Login' + 'Waiting for authorization...'; napi returns tokens → '✓ Connected to Claude'; Enter → back to list with a 'Logout from OAuth [Claude]' row
  #   2. User on 'Login with Claude (headless)' presses Enter → sees 'Visit: <authorizeUrl>' and 'Code:'; presses 'o' (input empty) → URL opens in browser; types/pastes 'abc#xyz'; presses Enter → claude headless-complete runs → '✓ Connected to Claude'
  #   3. User in headless-code-entry with empty input presses 'c' → authorize URL copied to clipboard, input stays empty; then types 'x' → input becomes 'x'; then presses 'c' again → input becomes 'xc' (c now a literal char, not copy)
  #   4. User on 'Login with ChatGPT (headless)' presses Enter → 'Codex Device Login' + 'Your code: ABCD-1234' + 'Visit: <url>' + spinner; device authorized elsewhere → '✓ Connected to ChatGPT'
  #   5. Codex browser login fails (napi error) → 'OAuth Login error' + message + 'Press Enter to retry | Esc to go back'; Enter restarts codex browser login; Esc returns to list
  #   6. User presses Esc during codex device-waiting → returns to list; a poll result (success OR error) that arrives after cancel is dropped and does not change the screen
  #   7. On a non-embedded (websocket) transport the 'Login with Claude (browser)' / 'Login with ChatGPT (browser)' rows are disabled/hidden, while the headless/device rows remain selectable
  #
  # ========================================
  Background: User Story
    As a fspec-tui user with Anthropic or Codex configured
    I want to log in via the browser or headless/device OAuth rows from Provider Settings
    So that I can actually authenticate Claude or ChatGPT from the Rust TUI with the same screens, keys and outcomes as the TypeScript TUI

  @tui
  @provider-settings
  @oauth
  Scenario: Anthropic browser login shows the waiting screen and connects on success
    Given the embedded transport is in use
    And the "anthropic" provider is expanded
    And the cursor is on the "Login with Claude (browser)" row
    When the user presses Enter
    Then the mode becomes oauth-browser-waiting for provider "anthropic"
    And the screen shows "Claude OAuth Login"
    And the screen shows "Waiting for authorization..."
    And the screen shows "Press Esc to cancel"
    When the backend browser login resolves with tokens
    Then the mode becomes oauth-success for provider "anthropic"
    And the screen shows "✓ Connected to Claude"
    When the user presses Enter
    Then the mode returns to list
    And a "Logout from OAuth [Claude]" row is present

  @tui
  @provider-settings
  @oauth
  Scenario: Anthropic headless code entry submits the pasted code and connects
    Given the "anthropic" provider is expanded
    And the cursor is on the "Login with Claude (headless)" row
    When the user presses Enter
    Then the mode becomes oauth-headless-code-entry for provider "anthropic"
    And the screen shows the authorize URL
    And the screen shows a "Code:" input
    When the user presses "o" while the code input is empty
    Then the authorize URL is opened in the browser
    And the code input remains empty
    When the user types "abc#xyz"
    And the user presses Enter
    Then the backend headless-complete is called with "abc#xyz" and the pkce verifier
    And on success the mode becomes oauth-success for provider "anthropic"
    And the screen shows "✓ Connected to Claude"

  @tui
  @provider-settings
  @oauth
  Scenario: In headless code entry c copies the URL only while the input is empty
    Given the "anthropic" provider is in oauth-headless-code-entry with an empty code input
    When the user presses "c"
    Then the authorize URL is copied to the clipboard
    And the code input remains empty
    When the user types "x"
    Then the code input is "x"
    When the user presses "c"
    Then the code input is "xc"
    And the clipboard is not copied again

  @tui
  @provider-settings
  @oauth
  Scenario: Codex headless enters device-waiting and connects when authorized elsewhere
    Given the "codex" provider is expanded
    And the cursor is on the "Login with ChatGPT (headless)" row
    When the user presses Enter
    Then the backend codex device-start is called
    And the mode becomes oauth-device-waiting for provider "codex"
    And the screen shows "Codex Device Login"
    And the screen shows the user code "ABCD-1234"
    And the screen shows the verification URL
    And the screen shows "Press Esc to cancel"
    When the backend device-poll resolves with tokens
    Then the mode becomes oauth-success for provider "codex"
    And the screen shows "✓ Connected to ChatGPT"

  @tui
  @provider-settings
  @oauth
  @error
  Scenario: A failed codex browser login shows the error screen and retries on Enter
    Given the embedded transport is in use
    And the "codex" provider is expanded
    And the cursor is on the "Login with ChatGPT (browser)" row
    When the user presses Enter
    And the backend browser login resolves with an error
    Then the mode becomes oauth-error for provider "codex"
    And the screen shows "OAuth Login error"
    And the screen shows the error message
    And the screen shows "Press Enter to retry | Esc to go back"
    When the user presses Enter
    Then codex browser login is started again
    When the user presses Esc
    Then the mode returns to list

  @tui
  @provider-settings
  @oauth
  Scenario: Cancelling codex device-waiting drops a late poll result
    Given the "codex" provider is in oauth-device-waiting
    When the user presses Esc
    Then the mode returns to list
    When a device-poll result arrives for the cancelled generation
    Then the result is dropped
    And the mode is still list

  @tui
  @provider-settings
  @oauth
  @integration
  Scenario: Browser login rows are gated to the embedded transport
    Given the websocket transport is in use
    And the "anthropic" provider is expanded
    Then the "Login with Claude (browser)" row is disabled or hidden
    And the "Login with Claude (headless)" row is selectable
    Given the "codex" provider is expanded
    Then the "Login with ChatGPT (browser)" row is disabled or hidden
    And the "Login with ChatGPT (headless)" row is selectable
