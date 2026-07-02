@done
@provider-settings
@agent-view
@ts-parity
@tui
@RPC-105
Feature: Provider settings: header line shows total nav items not configured count
  """
  Depends on RPC-103: this card cannot ship until ProviderSettingsView has a `nav_items: Vec<NavItem>` field. Until then there is no `navItems.length` analog to count. Implementation removes `configured_count()` method (no other callers) and replaces title_text() to use `self.nav_items.len()`.
  Existing unit tests that assert the title string (search `"Provider Settings ("` in `codelet/fspec-tui/src/views/provider_settings/`) and any rendering snapshot goldens must be updated in the same commit to use the new `(N items)` format.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Header title in TS uses `navItems.length` (ProviderSettingsPanel.tsx ~558) — the count of all visible flat-list rows including expanded children, NOT a per-provider configured count
  #   2. Title literal must be 'Provider Settings (N items)' — the noun is 'items' not 'configured'; no singular/plural variant ('1 items' is acceptable matching TS verbatim)
  #   3. Count is reactive to expand/collapse: every toggle that mutates nav_items.len() must propagate to the title (ProviderSettingsPanel.tsx re-renders because navItems is a useMemo dependency on the header)
  #   4. Count is reactive to filter: typing a filter shrinks navItems (filter check at useProviderSettingsState.ts:141-147) and the title count shrinks correspondingly
  #
  # EXAMPLES:
  #   1. Fresh view with 17 canonical providers all collapsed → title reads 'Provider Settings (17 items)'
  #   2. User expands anthropic (no tokens) injecting 3 child rows → title reads 'Provider Settings (20 items)'
  #   3. User types filter '/openai' with no expansions → only openai row visible → title reads 'Provider Settings (1 items)'
  #   4. Filter '/anth' applied while anthropic expanded with 3 children → title reads 'Provider Settings (4 items)' (anthropic provider + 3 children)
  #   5. Title MUST NOT contain the substring 'configured' — a regression guard test asserts the literal absence
  #
  # ========================================
  Background: User Story
    As a provider settings user
    I want to see the header count match the actual number of rows I can navigate to
    So that the count reactively reflects expand/collapse and filter, telling me how many items I can move my cursor onto right now

  Scenario: Title shows nav item count for all collapsed providers
    Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    And no providers are expanded
    And no filter is applied
    When I read the title text
    Then the title text equals "Provider Settings (17 items)"

  Scenario: Title count grows when a provider is expanded
    Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    And no providers are expanded
    And no filter is applied
    When I toggle expansion of "anthropic" which has 3 children
    Then the title text equals "Provider Settings (20 items)"

  Scenario: Title count shrinks when a filter is applied
    Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    And no providers are expanded
    When the user types the filter character sequence "openai"
    Then the title text equals "Provider Settings (1 items)"

  Scenario: Title count reflects both filter and expansion
    Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    And "anthropic" is expanded with 3 children
    When the user types the filter character sequence "anth"
    Then the title text equals "Provider Settings (4 items)"

  Scenario: Title text never contains the substring "configured"
    Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    When I read the title text
    Then the title text does not contain "configured"

  Scenario: Title uses nav_items length not configured count
    Given a ProviderSettingsView with 5 providers loaded as raw credential infos where 2 are configured
    And 5 providers loaded as display infos
    When I read the title text
    Then the title text equals "Provider Settings (5 items)"
    And the title text does not contain "2 configured"

  Scenario: Backspacing a filter character grows the title count back
    Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    And the user has typed the filter "openai"
    When the user presses Backspace once
    Then the filter equals "openai" minus the last character
    And the title text equals "Provider Settings (N items)" where N matches the new filtered nav_items length

  Scenario: Clearing the filter via Esc restores the full count
    Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    And the user has typed the filter "openai"
    When the user presses Esc while in filter mode
    Then the filter is empty
    And the title text equals "Provider Settings (17 items)"

  Scenario: configured_count method is removed
    Given the ProviderSettingsView source file
    When I search for the method name "configured_count"
    Then the source file does not declare a "configured_count" method
