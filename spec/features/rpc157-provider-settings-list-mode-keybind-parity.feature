@done
@agent-view
@ts-parity
@provider-settings
@tui
@rust
@RPC-157
Feature: Provider settings: drop Rust-only list-mode keybinds (r/R, wrap-around, PageUp/Down, Home/End) to match TS

  """
  Implementation: surgery in codelet/fspec-tui/src/views/provider_settings/mod.rs (and any extracted list-mode handler module — see RPC-103). Remove the match arms for KeyCode::Char('r') | Char('R'), KeyCode::PageUp, KeyCode::PageDown, KeyCode::Home, KeyCode::End from the list-mode dispatcher. Replace wrap_index() usage on ↑/↓ with saturating clamp: down = (sel + 1).min(len - 1); up = sel.saturating_sub(1). Remove `wrap_index` import if no remaining callers in the file.
  
  Dependencies: builds on top of RPC-054 (the existing view scaffolding) and indirectly on RPC-103 (flat-tree nav model — when RPC-103 lands the list-mode dispatcher will already be reshaped to operate over Vec<NavItem>; this card surgically removes excess keybinds from whatever dispatcher exists).
  
  Critical requirements: zero regressions on the other list-mode keybinds (↑/↓ clamped, Enter, /, Tab, Esc, d) — those are owned by RPC-054/103/106/160/164 etc. Removed keybinds must fall through to a no-op (do NOT match-and-ignore explicitly; let the default arm absorb them). Source-shape test asserts the absence of the removed match arms via grep on the dispatcher file.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. List-mode arrow navigation MUST NOT wrap around: pressing ↑ on the first nav-item is a no-op (cursor stays at index 0); pressing ↓ on the last nav-item is a no-op (cursor stays at len-1). TS listModeHandler.ts uses clamped arithmetic (Math.min/Math.max), NOT modulo, so no wrap occurs.
  #   2. List-mode key dispatcher MUST NOT bind PageUp or PageDown — these key events must fall through to the default no-op arm. TS listModeHandler.ts handles only key.upArrow / key.downArrow / key.return / '/' / key.tab / key.escape / 'd'; PageUp and PageDown are never mentioned.
  #   3. List-mode key dispatcher MUST NOT bind Home or End — these key events must fall through to the default no-op arm. TS listModeHandler.ts has no Home/End keybinds.
  #   4. List-mode key dispatcher MUST NOT bind 'r' or 'R' (refresh models). The refresh-models action is reachable only by selecting a provider, opening it (Enter expansion in the flat tree per RPC-103), and using the per-provider edit/test flow — there is no global list-mode shortcut. TS listModeHandler.ts has no r/R keybind.
  #   5. The full TS-canonical list-mode keybind set is EXACTLY: ↑ (move up clamped), ↓ (move down clamped), Enter (expand provider or activate child row), / (enter filter mode), Tab (SwitchToModels — see RPC-160), Esc (clear filter OR close view per RPC-054 two-step cascade), d (delete on api-key/profile rows). Any other key MUST be a no-op.
  #   6. Removed keybinds are silently ignored: no scrollback notice, no error log, no tracing warning. The KeyEvent is simply not matched by any arm in the dispatcher and the view re-renders unchanged.
  #
  # EXAMPLES:
  #   1. User is in list mode with 17 providers, cursor at index 0 (OpenAI API). Pressing ↑ leaves cursor at index 0 — no wrap to index 16, no movement. ProviderSettingsView.selected_index() still returns 0 after the keystroke.
  #   2. User is in list mode with 17 providers, cursor at index 16 (GitHub Copilot). Pressing ↓ leaves cursor at index 16 — no wrap to index 0, no movement. ProviderSettingsView.selected_index() still returns 16 after the keystroke.
  #   3. User is in list mode with cursor at index 5 (Mistral AI). Pressing PageDown leaves cursor at index 5 — the key is silently ignored, no jump to bottom-of-page occurs. No backend RPC is fired.
  #   4. User is in list mode with cursor at index 5. Pressing PageUp leaves cursor at index 5 — silently ignored.
  #   5. User is in list mode with cursor at index 8. Pressing Home leaves cursor at index 8 — silently ignored (no jump to index 0).
  #   6. User is in list mode with cursor at index 8. Pressing End leaves cursor at index 8 — silently ignored (no jump to last row).
  #   7. User is in list mode focused on the 'OpenAI API' provider row. Pressing 'r' is a no-op — Action::RefreshProviderModels is NOT emitted, no tracing log fires, the view re-renders unchanged. Pressing 'R' (shifted) is also a no-op.
  #   8. User is in list mode. Pressing ↑/↓/Enter/// /Tab/Esc/d all continue to behave per the TS-canonical contract (RPC-054/103/160 etc.). Only the four Rust-only keybinds (r/R, wrap-around behavior on arrows, PageUp/Down, Home/End) are removed; the rest of the list-mode contract is untouched.
  #
  # ========================================

  Background: User Story
    As a Rust frontend user
    I want to have provider-settings list-mode keybinds exactly match the TS Ink reference
    So that muscle memory, footer hints, and documentation remain consistent across the TS and Rust frontends with no Rust-only surprises

  Scenario: Up arrow at the first nav item is a clamped no-op (no wrap-around)
    Given I have opened /provider with the 17 canonical providers loaded
    And the cursor is on the first nav item (OpenAI API, index 0)
    When I press the Up arrow key
    Then the cursor remains on the first nav item (index 0)
    And the cursor does NOT wrap to the last nav item (index 16)
    And no other view state changes

  Scenario: Down arrow at the last nav item is a clamped no-op (no wrap-around)
    Given I have opened /provider with the 17 canonical providers loaded
    And the cursor is on the last nav item (GitHub Copilot, index 16)
    When I press the Down arrow key
    Then the cursor remains on the last nav item (index 16)
    And the cursor does NOT wrap to the first nav item (index 0)
    And no other view state changes

  Scenario: PageDown is silently ignored in list mode
    Given I have opened /provider with the 17 canonical providers loaded
    And the cursor is on the Mistral AI nav item (index 5)
    When I press the PageDown key
    Then the cursor remains on index 5
    And no jump-by-page occurs
    And no backend RPC is fired
    And no scrollback notice, error log, or tracing warning is emitted

  Scenario: PageUp is silently ignored in list mode
    Given I have opened /provider with the 17 canonical providers loaded
    And the cursor is on index 5
    When I press the PageUp key
    Then the cursor remains on index 5
    And no jump-by-page occurs
    And no scrollback notice, error log, or tracing warning is emitted

  Scenario: Home is silently ignored in list mode
    Given I have opened /provider with the 17 canonical providers loaded
    And the cursor is on index 8
    When I press the Home key
    Then the cursor remains on index 8
    And the cursor does NOT jump to index 0
    And no scrollback notice, error log, or tracing warning is emitted

  Scenario: End is silently ignored in list mode
    Given I have opened /provider with the 17 canonical providers loaded
    And the cursor is on index 8
    When I press the End key
    Then the cursor remains on index 8
    And the cursor does NOT jump to the last nav item
    And no scrollback notice, error log, or tracing warning is emitted

  Scenario: Lowercase "r" is silently ignored in list mode (no RefreshProviderModels)
    Given I have opened /provider with the 17 canonical providers loaded
    And the cursor is on the OpenAI API provider row
    When I press the "r" key
    Then no Action::RefreshProviderModels is emitted
    And no backend RPC is fired
    And no tracing log fires
    And the view re-renders unchanged

  Scenario: Uppercase "R" is silently ignored in list mode (no RefreshProviderModels)
    Given I have opened /provider with the 17 canonical providers loaded
    And the cursor is on the OpenAI API provider row
    When I press the "R" key (shifted)
    Then no Action::RefreshProviderModels is emitted
    And no backend RPC is fired
    And no tracing log fires
    And the view re-renders unchanged

  Scenario: Preserved TS-canonical keybinds continue to behave per the existing contract
    Given I have opened /provider with the 17 canonical providers loaded
    When I press each of the keys ↑, ↓, Enter, "/", Tab, Esc, and "d" in turn
    Then each key continues to dispatch its TS-canonical action per RPC-054, RPC-103, RPC-106, RPC-160, and RPC-164
    And only the four Rust-only keybind groups are removed (r/R, arrow wrap-around, PageUp/Down, Home/End)
    And no other behaviour regresses
