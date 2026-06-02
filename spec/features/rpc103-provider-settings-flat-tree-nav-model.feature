@done
@agent-view
@ts-parity
@provider-settings
@tui
@rust
@RPC-103
Feature: Provider settings: flat tree nav model with expand/collapse and child rows

  """
  Rust ProviderSettingsView replaces `ProviderSettingsMode::Detail { sub: DetailSub }` with a flat `Vec<NavItem>` where `NavItemKind` enum has six variants: Provider{expanded}, Profile{profile_name}, AddProfile, ApiKey, OAuthLogin{method,label}, OAuthStatus{label}. The old DetailSub::{Summary, EditApiKey, OAuthNotice} are removed — their UX is now inline child rows that dispatch Actions when Enter is pressed.
  Expansion is stored in a `HashSet<String> expanded` field on ProviderSettingsView (TS analog: useRef<Set<string>> expandedProviderIds). build_nav_items reads `expanded.contains(&p.id)` to decide whether to emit children. Toggle flow: Enter on Provider row → flip membership in `expanded` → rebuild nav_items → adjust scroll only if needed; selected_index is left untouched.
  PanelMode (modal overlay enum) shrinks to ONLY destructive/edit overlays: EditApiKey, DeleteApiKey, DeleteProfile, DisconnectOAuth, ProfileForm, OAuth waiting/success/error. List-mode navigation is no longer a PanelMode variant — list mode is the default and is always active when no overlay is showing.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. buildNavItems (useProviderSettingsState.ts:132-206) is a pure function of (providers, filter) — the Rust port must be a pure `fn build_nav_items(&[ProviderDisplayInfo], &str) -> Vec<NavItem>` with NO interior mutability
  #   2. The flat list contains six row variants (provider, profile, add-profile, api-key, oauth-login, oauth-status) per ProviderSettingsPanel.tsx:120-135 — Rust NavItemKind enum must have the same six variants and NO additional ones
  #   3. Expansion state persists across reload() per useProviderSettingsState.ts:243 and :340 — Rust must keep a HashSet<String> expanded field that is NOT cleared by set_providers(...) calls
  #   4. Enter on a provider row toggles expansion (listModeHandler.ts:119-121) and does NOT mutate selectedIndex — the cursor stays on the same provider row while children appear/disappear below it
  #   5. Child-row ordering when expanded is fixed (useProviderSettingsState.ts:158-201): [oauth-status?, oauth-login×N?, api-key?, profiles×N?, add-profile?] — Rust builder must emit children in exactly this order
  #   6. Filter is parent-anchored (useProviderSettingsState.ts:141-147): if provider.name and provider.id both fail the substring match, the provider AND all its children are removed from navItems
  #
  # EXAMPLES:
  #   1. Fresh view, 17 providers all collapsed → build_nav_items returns 17 NavItems all of NavItemKind::Provider { expanded: false }, in canonical registry order
  #   2. Cursor at index 3 (anthropic), press Enter → anthropic.expanded toggles to true, build_nav_items inserts [oauth-login(browser), oauth-login(headless), api-key] at indices 4,5,6; cursor STAYS at index 3
  #   3. openai expanded with 2 profiles → child rows are [profile(prof1), profile(prof2), add-profile] in that order; no api-key row (openai is profile-only); no oauth rows (openai is not OAuth)
  #   4. anthropic expanded with hasOAuthTokens=true → child rows are [oauth-status, oauth-login(browser), oauth-login(headless), api-key]; oauth-status appears FIRST in the child block
  #   5. User filters '/anth' while openai is also expanded → only the anthropic row (plus any anthropic children if anthropic is expanded) survives; openai children disappear because the openai PARENT was filtered out
  #   6. Reload triggered after destructive op: set_providers() rebuilds the providers Vec from disk; the `expanded: HashSet` is NOT cleared, so anthropic stays expanded across the reload (matches TS expandedProviderIds.current behavior)
  #
  # ========================================

  Background: User Story
    As a provider settings user
    I want to press Enter on a provider row to expand or collapse its children inline
    So that I see all interactable rows (api-key, oauth-login, profiles, add-profile) at known indices in a single flat list, matching the TS Ink reference

  Scenario: Fresh view with collapsed providers yields one NavItem per provider
    Given a fresh ProviderSettingsView with no expanded providers
    And the providers list contains 17 entries in canonical registry order
    And the filter is empty
    When build_nav_items is called
    Then the result contains exactly 17 NavItems
    And every NavItem has kind NavItemKind::Provider { expanded: false }
    And the NavItems appear in canonical registry order

  Scenario: Expanding anthropic without OAuth tokens injects oauth-login and api-key children
    Given a ProviderSettingsView containing the anthropic provider
    And the anthropic provider has no OAuth tokens (hasOAuthTokens = false)
    And anthropic is in the expanded set
    When build_nav_items is called
    Then the row immediately after anthropic is NavItemKind::OAuthLogin { method: Browser }
    And the next row is NavItemKind::OAuthLogin { method: Headless }
    And the next row is NavItemKind::ApiKey
    And no NavItemKind::OAuthStatus row appears in the anthropic child block

  Scenario: Expanding openai with profiles injects profile rows and a trailing add-profile pseudo-row
    Given a ProviderSettingsView containing the openai provider
    And openai has 2 profiles named "prof1" and "prof2"
    And openai is in the expanded set
    When build_nav_items is called
    Then the rows immediately after openai are NavItemKind::Profile { profile_name: "prof1" } then NavItemKind::Profile { profile_name: "prof2" }
    And the next row is NavItemKind::AddProfile
    And no NavItemKind::ApiKey row appears in the openai child block
    And no NavItemKind::OAuthLogin or NavItemKind::OAuthStatus row appears in the openai child block

  Scenario: Expanding anthropic with OAuth tokens prepends an oauth-status row before oauth-login rows
    Given a ProviderSettingsView containing the anthropic provider
    And the anthropic provider has OAuth tokens (hasOAuthTokens = true)
    And anthropic is in the expanded set
    When build_nav_items is called
    Then the row immediately after anthropic is NavItemKind::OAuthStatus
    And the next two rows are NavItemKind::OAuthLogin { method: Browser } then NavItemKind::OAuthLogin { method: Headless }
    And the next row is NavItemKind::ApiKey

  Scenario: Filter is parent-anchored — children of filtered-out providers disappear
    Given a ProviderSettingsView with anthropic and openai both expanded
    And openai has 2 profiles
    When the filter is set to "anth"
    And build_nav_items is called
    Then the result contains the anthropic NavItem and its child rows
    And the result contains NO openai NavItem
    And the result contains NO child rows belonging to openai (no profile rows, no add-profile row)

  Scenario: Expansion state survives set_providers reload
    Given a ProviderSettingsView with the anthropic provider expanded
    When set_providers is called with a freshly rebuilt providers Vec from disk
    And build_nav_items is called
    Then anthropic remains in the expanded set
    And the anthropic NavItem has kind NavItemKind::Provider { expanded: true }
    And anthropic's child rows are present immediately after it

  Scenario: Enter on a provider row toggles expansion without mutating selected_index
    Given a ProviderSettingsView with selected_index pointing at the anthropic provider row
    And anthropic is currently collapsed (not in the expanded set)
    When the user presses Enter
    Then anthropic is added to the expanded set
    And selected_index still points at the anthropic provider row
    And the anthropic child rows now appear immediately below selected_index in nav_items
    When the user presses Enter again
    Then anthropic is removed from the expanded set
    And selected_index still points at the anthropic provider row

  Scenario: Enter on an api-key child row transitions directly to the EditApiKey state
    Given a ProviderSettingsView with anthropic expanded
    And selected_index points at the NavItemKind::ApiKey child row under anthropic
    When the user presses Enter
    Then the view's mode becomes ProviderSettingsMode::Detail with provider_id "anthropic" and sub DetailSub::EditApiKey with an empty draft
    And the keystroke is reported as ProviderSettingsEvent::Consumed
    And the view does not first land on a Summary sub-view — Enter routes directly to the edit form
