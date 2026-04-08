@done
@provider-settings
@providers
@tui
@validation
@PROV-054
Feature: GitHub Copilot TUI provider integration
  """
  Uses new HookMode variants oauth-deployment-type-select and oauth-enterprise-url-entry. GitHub Copilot is registered in SUPPORTED_PROVIDERS with authType 'oauth' and requiresApiKey false. NAPI bridge (codelet/napi/src/copilot_oauth.rs) exposes copilotOauthDeviceLoginStart/Poll/GetCredential/ClearCredential/NormalizeEnterpriseDomain. Login flow is orchestrated by src/tui/utils/copilotLoginFlow.ts and driven by src/tui/inputHandlers/copilotOauthModeHandler.ts. OAuth labels come from src/tui/utils/oauthProviderLabels.ts (single source of truth, no more binary ternaries). Login items come from src/tui/utils/oauthLoginLabels.ts (registry-driven, no hard-coded isAnthropic).
  """

  Background: User Story
    As a fspec user with a GitHub Copilot subscription
    I want to see and interact with GitHub Copilot in the codelet TUI providers screen like any other provider
    So that I can sign in via the device flow and use my Copilot entitlement without leaving the TUI

  Scenario: GitHub Copilot appears in the TUI providers list after provider registration
    Given the provider registry contains a 'github-copilot' entry with authType 'oauth' and requiresApiKey false
    When the user opens the provider settings screen in the codelet TUI
    Then a row labelled 'GitHub Copilot' is displayed in the provider list
    And no copilot_auth.json credential file exists
    And the row shows the status '(not configured)' because no credential exists

  Scenario: Expanding GitHub Copilot row reveals the device-flow login option only
    Given the GitHub Copilot row is visible in the provider list
    When the user presses Enter on the GitHub Copilot row
    Then exactly one login item appears labelled 'Login with GitHub Copilot (device flow)'
    And no browser-login item is shown for GitHub Copilot
    And no API-key row is shown for GitHub Copilot

  Scenario: Starting login transitions the TUI into deployment-type selection mode
    Given the 'Login with GitHub Copilot (device flow)' row is highlighted
    When the user presses Enter on the login row
    Then the TUI mode becomes 'oauth-deployment-type-select' with providerId 'github-copilot'
    And a prompt is shown with two options 'GitHub.com (Public)' and 'GitHub Enterprise (self-hosted)'

  Scenario: Selecting github.com launches device-code flow without prompting for URL
    Given the TUI is in the deployment-type selection mode for github-copilot
    When the user selects 'GitHub.com' and presses Enter
    Then the TUI calls copilotOauthDeviceLoginStart with enterpriseUrl omitted
    And the TUI mode transitions to 'oauth-device-waiting' showing the user code and verification URL
    And no enterprise URL prompt is shown

  Scenario: Selecting enterprise prompts for the enterprise URL before the device flow
    Given the TUI is in the deployment-type selection mode for github-copilot
    When the user selects 'GitHub Enterprise' and presses Enter
    Then the TUI mode becomes 'oauth-enterprise-url-entry' with an empty urlInput
    And a text input is shown with placeholder 'company.ghe.com'

  Scenario: Submitting a valid enterprise URL normalizes it and launches the device flow
    Given the TUI is in the enterprise URL entry mode for github-copilot
    When the user types 'https://ghe.example.com/' and presses Enter
    Then the URL is normalized to 'ghe.example.com' (scheme and trailing slash stripped)
    And the TUI calls copilotOauthDeviceLoginStart with enterpriseUrl 'ghe.example.com'
    And the TUI mode transitions to 'oauth-device-waiting' showing the user code and verification URL

  Scenario: After successful authorization the GitHub Copilot row shows OAuth status
    Given the TUI has completed a successful Copilot device-flow login
    When the provider list is reloaded
    Then copilotOauthGetCredential returns the persisted credential
    And the GitHub Copilot row displays '✓ OAuth [GitHub Copilot]' for github.com deployments
    And a 'Logout from OAuth' row becomes visible under the expanded GitHub Copilot provider

  Scenario: Disconnecting OAuth deletes the credential file via the NAPI bridge
    Given the GitHub Copilot row shows '✓ OAuth [GitHub Copilot]'
    When the user selects 'Logout from OAuth' and confirms with 'y'
    Then copilotOauthClearCredential is called
    And the copilot_auth.json file is deleted from the fspec credentials directory
    And the GitHub Copilot row updates to '(not configured)'
