@done
@security
@destructive-action
@provider-settings
@tui
@PROV-029
Feature: Provider Settings TUI — OAuth profile guards, dead code cleanup, keybind simplification, provider list cleanup
  """
  The Provider Settings TUI needs several fixes:
  1. Profiles restricted to OpenAI API provider only (local models: vLLM, Ollama)
  2. Dead code deletion (ProviderSettingsView.tsx, useProviderProfiles.ts)
  3. Keybind simplification: only Enter and 'd' — no 'e', 'n', 't'
  4. Uniform delete confirmations for API key, OAuth disconnect, profile delete
  5. Context-sensitive footer showing available keybinds per item type
  6. Remove providers without tool calling support (Perplexity, Hyperbolic, Mira, Voyage AI)
  7. Remove Ollama as distinct provider (use OpenAI API profiles instead)
  8. Rename OpenAI to 'OpenAI API' (local model compatible API format)
  9. Add codex to PROVIDER_ENV_VARS in credentials.ts
  10. Footer Tab hint: 'Switch to models' / 'Switch to providers'
  """

  Background: 
    Given the Provider Settings TUI is open

  # --- Provider list composition ---
  Scenario: Provider list contains only providers with tool calling support
    Then the provider list contains exactly 17 providers
    And the following providers are NOT in the list:
      | provider   | reason                  |
      | Ollama     | Use OpenAI API profiles |
      | Perplexity | No tool calling support |
      | Hyperbolic | No tool calling support |
      | Mira       | No tool calling support |
      | Voyage AI  | Embedding-only provider |
    And "OpenAI" is displayed as "OpenAI API"

  # --- OAuth provider expansion ---
  Scenario: Expanding an OAuth provider shows OAuth items and API key but no profiles
    Given Anthropic is configured with OAuth connected and an API key from env
    When I expand the Anthropic provider
    Then I see the following nav items:
      | item                               |
      | ✓ OAuth [Claude]                   |
      | 🔑 Login with Claude (browser)     |
      | 🔑 Login with Claude (headless)    |
      | 🔑 API key ✓ sk-ant-••••Qr7K [env] |
    And I do NOT see any profile rows
    And I do NOT see a "Create new profile" button
    And the header does NOT show a profile count

  Scenario: Expanding an OAuth provider with stale profiles in config ignores them
    Given Anthropic has a stale profile in user config from OAuth development
    When I expand the Anthropic provider
    Then the stale profile row is NOT displayed
    And the header does NOT show "(1 profile)"
    And the application does not crash or show an error

  # --- Cloud API-key provider expansion ---
  Scenario: Expanding a cloud API-key provider shows only the API key row
    Given Google Gemini has an API key configured from env
    When I expand the Google Gemini provider
    Then I see only the "🔑 API key" nav item
    And I do NOT see any profile rows
    And I do NOT see any OAuth items
    And I do NOT see a "Create new profile" button

  # --- OpenAI API (profile-only) expansion ---
  Scenario: Expanding OpenAI API with profiles shows profile rows and create button
    Given OpenAI API has 2 profiles configured
    When I expand the OpenAI API provider
    Then I see the following nav items:
      | item                                    |
      | 📁 work-vllm → http://10.0.1.5:8080     |
      | 📁 home-ollama → http://localhost:11434 |
      | + Create new profile                    |
    And I do NOT see a "🔑 API key" row
    And the header shows "(2 profiles)"

  Scenario: Expanding OpenAI API with no profiles shows only create button
    Given OpenAI API has no profiles configured
    When I expand the OpenAI API provider
    Then I see only the "+ Create new profile" nav item
    And I do NOT see a "🔑 API key" row

  # --- saveProfile guard ---
  Scenario: saveProfile rejects non-OpenAI-API providers
    When saveProfile is called with providerId "gemini"
    Then an error is thrown: "Profiles are only supported for OpenAI API provider"

  Scenario: saveProfile rejects OAuth providers
    When saveProfile is called with providerId "anthropic"
    Then an error is thrown: "Profiles are only supported for OpenAI API provider"

  Scenario: saveProfile accepts OpenAI API provider
    When saveProfile is called with providerId "openai" and valid profile data
    Then the profile is saved successfully

  # --- Keybind behavior: Enter ---
  Scenario: Enter on a provider row toggles expansion
    Given I have the cursor on a collapsed provider row
    When I press Enter
    Then the provider expands to show its nav items

  Scenario: Enter on a login item starts the OAuth flow
    Given I have the cursor on "🔑 Login with Claude (browser)"
    When I press Enter
    Then the browser OAuth flow starts

  Scenario: Enter on an API key item opens the key editor
    Given I have the cursor on "🔑 API key" for Google Gemini
    When I press Enter
    Then the API key editor opens

  Scenario: Enter on a profile item opens the profile editor
    Given I have the cursor on "📁 work-vllm" under OpenAI API
    When I press Enter
    Then the profile editor opens

  Scenario: Enter on create new profile starts profile creation
    Given I have the cursor on "+ Create new profile" under OpenAI API
    When I press Enter
    Then the new profile form opens

  # --- Keybind behavior: 'd' with confirmation ---
  Scenario: Pressing 'd' on an API key item shows delete confirmation
    Given I have the cursor on "🔑 API key" for Google Gemini
    When I press "d"
    Then a confirmation dialog appears: "Delete API key for Google Gemini? (y/n)"

  Scenario: Declining API key delete confirmation preserves the key
    Given a "Delete API key" confirmation dialog is shown
    When I press "n"
    Then the API key is preserved
    And I return to list mode

  Scenario: Confirming API key delete removes the key
    Given a "Delete API key" confirmation dialog is shown
    When I press "y"
    Then the API key is deleted
    And the provider status updates

  Scenario: Pressing 'd' on an OAuth status item shows disconnect confirmation
    Given I have the cursor on "✓ OAuth [Claude]"
    When I press "d"
    Then a confirmation dialog appears: "Disconnect Claude OAuth? (y/n)"

  Scenario: Confirming OAuth disconnect clears tokens
    Given a "Disconnect OAuth" confirmation dialog is shown
    When I press "y"
    Then the OAuth tokens are cleared
    And the OAuth status updates

  Scenario: Pressing 'd' on a profile item shows delete confirmation
    Given I have the cursor on "📁 work-vllm" under OpenAI API
    When I press "d"
    Then a confirmation dialog appears: "Delete profile work-vllm? (y/n)"

  # --- Removed keybinds ---
  Scenario: Pressing 'e' does nothing on any item
    Given I have the cursor on any nav item
    When I press "e"
    Then nothing happens

  Scenario: Pressing 'n' does nothing on any item
    Given I have the cursor on any nav item
    When I press "n"
    Then nothing happens

  Scenario: Pressing 't' does nothing on any item
    Given I have the cursor on any nav item
    When I press "t"
    Then nothing happens

  # --- Context-sensitive footer ---
  Scenario: Footer updates based on selected item type
    When I navigate to different item types the footer shows:
      | item type          | footer                                                                  |
      | provider row       | Enter: expand · / filter · Tab: Switch to models · Esc: close           |
      | oauth status       | d: disconnect · / filter · Tab: Switch to models · Esc: close           |
      | login item         | Enter: start login · / filter · Tab: Switch to models · Esc: close      |
      | api key            | Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close |
      | profile            | Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close |
      | create new profile | Enter: create · / filter · Tab: Switch to models · Esc: close           |

  Scenario: Tab hint says "Switch to models" on provider settings panel
    Given I am on the provider settings panel
    Then the footer includes "Tab: Switch to models"

  # --- PROVIDER_ENV_VARS ---
  Scenario: PROVIDER_ENV_VARS includes codex entry
    Then the PROVIDER_ENV_VARS map in credentials.ts includes "codex" with value "CODEX_API_KEY"

  # --- Dead code cleanup ---
  Scenario: Dead code files are removed
    Then "src/tui/components/ProviderSettingsView.tsx" does not exist
    And "src/tui/hooks/useProviderProfiles.ts" does not exist
    And the project builds successfully with no broken imports

  Scenario: Dead types in provider.ts are cleaned up
    Then "src/tui/types/provider.ts" does not contain types only used by dead code
    And the following types are removed if unused elsewhere:
      | type                 |
      | ProviderWithProfiles |
      | ProfileDisplay       |
      | ProviderStatus       |
      | SettingsViewMode     |
      | ConnectionTestResult |
