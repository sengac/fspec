@done
@rust
@ts-parity
@bug
@provider-settings
@tui
@PROV-103
Feature: Up/Down and Enter dead in provider settings nav tree

  """
  TS-parity research (deepsearch of listModeHandler.ts/useProviderSettingsState.ts/ProviderSettingsPanel.tsx): every NavItem variant (provider, profile, add-profile, api-key, oauth-login, oauth-status) is an Enter-actionable landing target. TS Up/Down is a plain ±1 clamp to [0, navItems.length-1] with NO skip-non-selectable loop. Therefore the correct parity fix bounds move_clamped/adjust_scroll by nav_items.len() WITHOUT a header-skip — the model_selector skip logic exists only because that view has non-selectable provider HEADER rows; the provider_settings nav tree has none.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Up/Down navigation and scroll are bounded by the rendered nav_items length (the full flat tree) whenever nav_items is populated
  #   2. When nav_items is empty (legacy set_providers-only callers), navigation falls back to bounding by visible_providers().len()
  #   3. Every nav_items row is a valid landing target (TS parity: no non-selectable header skip logic); Up/Down is a plain clamped step within [0, nav_items.len()-1]
  #   4. scroll_offset is reconciled against nav_items.len() so the highlighted row stays visible across the full tree, not pinned at 0
  #   5. No new silent selection fallback is introduced (PROV-101 mandate): when nav_items is empty navigation is a clamped no-op, never an implicit row-0 selection
  #
  # EXAMPLES:
  #   1. Given an expanded openai provider with profiles, pressing Down from the provider row lands on the first child (a profile/api-key/add-profile row) instead of being trapped at the top-level provider count
  #   2. Given an expanded provider deep in a long nav tree, repeatedly pressing Down moves the cursor onto every rendered child row and the highlighted row scrolls into view (scroll_offset advances past 0)
  #   3. Given the cursor is on an api-key child row, pressing Enter transitions to the EditApiKey detail (Enter is no longer dead on child rows)
  #   4. Given the cursor is at the last nav_items row, pressing Down is a clamped no-op (no wrap, no out-of-bounds); pressing Up at row 0 is likewise a no-op
  #   5. Given nav_items is empty (legacy set_providers-only state), Down with two providers can still step to index 1 but never beyond providers.len()-1, and never silently snaps to a selection
  #
  # ========================================

  Background: User Story
    As a fspec user navigating the provider settings nav tree
    I want to move the cursor with Up/Down and press Enter on any row including expanded child rows
    So that I can reach and act on profiles, API-key and OAuth rows, not just the top-level providers

  Scenario: Down from an expanded provider lands on its first child row
    Given a provider settings view with an expanded openai provider that has profiles
    And the nav tree has more rows than the single top-level provider
    And the cursor is on the openai provider row
    When the user presses Down
    Then the cursor moves to the first child row beneath the provider
    And the focused nav item belongs to the openai provider child block

  Scenario: Repeated Down reaches every child row and scrolls the highlighted row into view
    Given a provider settings view with an expanded provider in a nav tree taller than the viewport
    And the viewport shows fewer rows than the nav tree contains
    When the user presses Down enough times to move past the viewport height
    Then the cursor reaches a child row beyond the first viewport
    And the scroll offset advances past zero so the highlighted row stays visible

  Scenario: Enter on an api-key child row opens the edit-api-key detail
    Given a provider settings view with an expanded provider exposing an api-key child row
    And the cursor is on the api-key child row
    When the user presses Enter
    Then the view transitions to the edit-api-key detail for that provider

  Scenario: Navigation is clamped with no wrap at the tree boundaries
    Given a provider settings view with an expanded provider producing several nav rows
    And the cursor is on the last nav row
    When the user presses Down
    Then the cursor stays on the last nav row
    When the cursor is moved to the first nav row
    And the user presses Up
    Then the cursor stays on the first nav row

  Scenario: Empty nav tree falls back to provider-count bounds without silent selection
    Given a provider settings view populated only via the legacy provider list with two providers
    And the nav items list is empty
    And the cursor is on the first provider
    When the user presses Down
    Then the cursor moves to the second provider
    When the user presses Down again
    Then the cursor stays on the second provider and no selection is silently snapped
