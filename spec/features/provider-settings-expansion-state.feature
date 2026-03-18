@PROV-036
Feature: Provider settings tree collapses after OAuth logout confirmation
  """
  Changes confined to useProviderSettingsState.ts. Add expandedProviderIds ref that tracks which providers are expanded, surviving reload(). Add navigateToProviderRef that signals post-reload cursor repositioning. toggleProviderExpansion updates both state and ref. disconnectOauth, removeApiKey, removeProfile set navigateToProviderRef before reload().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. reload() must preserve which providers are expanded — expansion state tracked via a ref
  #   2. After disconnectOauth, removeApiKey, or removeProfile, selectedIndex must point to the parent provider row
  #   3. toggleProviderExpansion must update the expansion ref so reload reads from it
  #   4. Changes are confined to useProviderSettingsState.ts — no new types, components, or mode changes
  #
  # EXAMPLES:
  #   1. User expands Anthropic, selects Logout, confirms disconnect → tree stays expanded and cursor moves to the Anthropic provider row
  #   2. User expands Anthropic, selects Logout, cancels with 'n' → tree stays expanded and cursor stays on the Logout row
  #   3. User deletes API key from expanded provider → tree stays expanded and cursor moves to the provider row
  #
  # ========================================
  Background: User Story
    As a user
    I want to disconnect OAuth and remain on the same provider row with the tree still expanded
    So that I can see what happened and continue navigating without losing context

  Scenario: Expansion state preserved after OAuth disconnect confirmation
    Given Anthropic provider is expanded with an OAuth logout row visible
    When the user confirms the OAuth disconnect
    Then the Anthropic provider tree remains expanded
    Then the selected index points to the Anthropic provider row

  Scenario: Cancel disconnect keeps cursor on logout row
    Given Anthropic provider is expanded with an OAuth logout row visible
    When the user cancels the OAuth disconnect confirmation
    Then the Anthropic provider tree remains expanded
    Then the cursor remains on the OAuth logout row

  Scenario: Expansion state preserved after API key deletion
    Given a provider is expanded with an API key configured
    When the user confirms the API key deletion
    Then the provider tree remains expanded
    Then the selected index points to the provider row
