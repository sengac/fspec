@done
@rust
@ts-parity
@oauth
@provider-settings
@tui
@PROV-114
Feature: GitHub Copilot OAuth device flow (deployment/enterprise preamble)

  """
  Builds on PROV-112 (OAuth boundary) and PROV-113 (shared oauth-device-waiting / oauth-success / oauth-error modes + generation stale-cancel). Adds two copilot-only preamble modes: OAuthDeploymentTypeSelect{provider_id,selected_index} and OAuthEnterpriseUrlEntry{provider_id,url_input,validation_error} (TS settingsMode.ts:48-52,59-63), with sub-handlers + renderers (CopilotOauthRender.tsx parity). Enter on the github-copilot login row → deployment-type-select. Backend (codelet-providers-direct via embedded): copilot::oauth_device_code device-start (Option<enterprise_url>) + copilot::oauth_polling device-poll; copilot::oauth_device_code::normalize_enterprise_domain (sync, pure) for host normalization. github.com → device-start(None); enterprise → enterprise-url-entry → normalize → device-start(Some(host)); both converge on PROV-113's device-waiting → success/error. Dispatch via dispatch_provider_settings_oauth.rs. list_actions.rs Enter on the copilot OauthLogin row routes here (checked by provider_id == github-copilot FIRST, before method, per dossier §2.1). Offline tests: MockBackend scripted Ok/Err + call counters (assert enterprise host passed through); view/key tests drive handle_key; no real network/~/.fspec mutation. Files <300 LoC; clippy -D warnings + fmt clean; NO git; do not touch user WIP. Reference: spec/attachments/PROV-105/oauth-parity-spec.md §4.3, §6, §9 copilot_oauth; napi/src/copilot_oauth.rs for exact semantics.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Enter on 'Login with GitHub Copilot (device flow)' enters oauth-deployment-type-select showing 'GitHub Copilot Login — Select deployment type' with two options: 'GitHub.com' (Public, index 0) and 'GitHub Enterprise' (Self-hosted / data residency, index 1); ↑ selects index 0, ↓ selects index 1; Esc cancels
  #   2. On deployment-type-select Enter: index 0 (github.com) begins device polling directly with null enterprise host; index 1 (enterprise) enters oauth-enterprise-url-entry
  #   3. In enterprise-url-entry: printable ASCII 32-126 append to urlInput; Backspace/Delete pop last char and clear any validationError; Enter with empty input sets validationError 'URL or domain is required'; Enter with non-empty input normalizes the host (strip scheme/trailing slash) then begins device polling; Esc cancels
  #   4. beginCopilotDevicePolling: copilot device-start (with optional enterprise host) yields userCode+verificationUrl → oauth-device-waiting (providerId github-copilot, deviceWaitingTitle 'GitHub Copilot Device Login'); device-poll runs fire-and-forget → success=oauth-success ('✓ Connected to GitHub Copilot') + reload, error=oauth-error; a start failure also routes to oauth-error
  #   5. The copilot device-waiting/success/error screens and their keybindings are the SAME shared modes introduced in PROV-113; PROV-114 only adds the deployment-type-select and enterprise-url-entry preamble modes and the copilot device start/poll backend calls
  #
  # EXAMPLES:
  #   1. User on the copilot login row presses Enter, leaves 'GitHub.com' selected (index 0), presses Enter → device-waiting with code + URL; authorizes on github.com → '✓ Connected to GitHub Copilot'
  #   2. User selects 'GitHub Enterprise' (↓ then Enter) → enterprise-url-entry; types 'https://company.ghe.com/'; presses Enter → host normalized to 'company.ghe.com', device polling begins against the enterprise host → '✓ Connected to GitHub Copilot'
  #   3. User selects 'GitHub Enterprise' then presses Enter with the URL field empty → red validationError 'URL or domain is required' shown, stays on enterprise-url-entry; typing a char clears the error
  #   4. User presses Esc on deployment-type-select → returns to list, no backend call; likewise Esc on enterprise-url-entry returns to list
  #   5. Copilot device-start fails (napi error) → oauth-error 'OAuth Login error' + message; Enter retries; Esc returns to list
  #
  # ========================================

  Background: User Story
    As a fspec-tui user with GitHub Copilot configured
    I want to log in with the GitHub Copilot device flow, choosing GitHub.com or a GitHub Enterprise host
    So that I can authenticate Copilot (including enterprise/data-residency hosts) from the Rust TUI exactly like the TypeScript TUI

  @tui @provider-settings @oauth
  Scenario: GitHub.com device flow goes straight to device-waiting and connects
    Given the "github-copilot" provider is expanded
    And the cursor is on the "Login with GitHub Copilot (device flow)" row
    When the user presses Enter
    Then the mode becomes oauth-deployment-type-select for provider "github-copilot"
    And the screen shows "GitHub Copilot Login — Select deployment type"
    And "GitHub.com" is selected at index 0
    When the user presses Enter
    Then the backend copilot device-start is called with no enterprise host
    And the mode becomes oauth-device-waiting for provider "github-copilot"
    And the screen shows the user code and verification URL
    When the backend device-poll resolves with a credential
    Then the mode becomes oauth-success for provider "github-copilot"
    And the screen shows "✓ Connected to GitHub Copilot"

  @tui @provider-settings @oauth @enterprise
  Scenario: GitHub Enterprise prompts for a host, normalizes it, and polls against it
    Given the "github-copilot" provider is in oauth-deployment-type-select
    When the user presses Down
    Then "GitHub Enterprise" is selected at index 1
    When the user presses Enter
    Then the mode becomes oauth-enterprise-url-entry for provider "github-copilot"
    When the user types "https://company.ghe.com/"
    And the user presses Enter
    Then the host is normalized to "company.ghe.com"
    And the backend copilot device-start is called with enterprise host "company.ghe.com"
    And the mode becomes oauth-device-waiting for provider "github-copilot"

  @tui @provider-settings @oauth @enterprise @error
  Scenario: Submitting an empty enterprise URL shows a validation error
    Given the "github-copilot" provider is in oauth-enterprise-url-entry with an empty URL input
    When the user presses Enter
    Then the screen shows the validation error "URL or domain is required"
    And the mode is still oauth-enterprise-url-entry
    And no backend device-start is called
    When the user types "c"
    Then the validation error is cleared

  @tui @provider-settings @oauth
  Scenario: Esc cancels the copilot preamble modes back to list
    Given the "github-copilot" provider is in oauth-deployment-type-select
    When the user presses Esc
    Then the mode returns to list
    And no backend call is made
    Given the "github-copilot" provider is in oauth-enterprise-url-entry
    When the user presses Esc
    Then the mode returns to list
    And no backend call is made

  @tui @provider-settings @oauth @error
  Scenario: A failed copilot device-start shows the error screen and retries on Enter
    Given the "github-copilot" provider is in oauth-deployment-type-select
    When the user presses Enter
    And the backend copilot device-start resolves with an error
    Then the mode becomes oauth-error for provider "github-copilot"
    And the screen shows "OAuth Login error"
    And the screen shows the error message
    When the user presses Enter
    Then the copilot login flow is retried
    When the user presses Esc
    Then the mode returns to list
