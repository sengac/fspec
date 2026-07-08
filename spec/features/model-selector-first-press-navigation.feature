@done
@navigation
@model-selector
@tui
@PROV-124
Feature: /model selector swallows the first arrow press — cursor needs two presses to move when opened with no matched current model
  """
  has_selection gates Enter ONLY, never movement. move_up/move_down/page_up/page_down (navigation.rs) no longer early-return to anchor_first_selectable() when has_selection is false; instead each sets has_selection=true and performs the clamped no-wrap move on the same press. This restores TS parity with useModelSelectorState.ts navigateUp (Math.max(idx-1,0)) and navigateDown (Math.min(idx+1,len-1)) — no has_selection gate on movement, no anchor-to-first-selectable side effect. set_providers still leaves has_selection=false when no current model matches (PROV-101), so Enter on a model row remains a no-op until the user navigates; on a fresh collapse-by-default open every row is a header and Enter on a header toggles expansion. anchor_first_selectable() is retained for Home and filter-change paths only. PROV-124 fix; introduced-by PROV-101.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The first explicit navigation (Up/Down/PageUp/PageDown) sets has_selection=true AND performs the clamped move on the same press — the cursor moves visibly on press one, never on press two
  #   2. has_selection gates Enter only, not movement; movement is always a clamped no-wrap move matching the TS navigateUp/navigateDown reference
  #   3. Pressing Up while the cursor is on the top row (row 0) is a clamped no-move (cursor stays on row 0) but still activates the selection (has_active_selection becomes true)
  #   4. Opening on a matched current model still seeds the cursor on that model's row with has_selection=true, so Enter selects it immediately without any navigation (RPC-341 regression guard)
  #   5. Enter before any navigation on a fresh no-match open remains a no-op on a model row (has_selection starts false, so Enter emits no ModelSelected) — PROV-101 regression guard
  #
  # EXAMPLES:
  #   1. Open with no matched current model (all providers collapsed, every row a header); pressing Down once moves the cursor from row 0 to row 1 and has_active_selection() is true
  #   2. Open with no matched current model; pressing Up once at the top row leaves the cursor on row 0 (clamped no-move) but has_active_selection() is true
  #   3. Open with no matched current model on a list taller than the viewport; pressing PageDown once moves the selection by one viewport step on the first press and has_active_selection() is true
  #   4. Open with no matched current model; pressing Enter before any navigation on a model row is a consumed no-op and emits no ModelSelected action (PROV-101 guard)
  #   5. Open with current model 'claude-sonnet' matched; the cursor is seeded on the claude-sonnet row and pressing Enter immediately emits ModelSelected for it without any navigation (RPC-341 guard)
  #
  # ========================================
  Background: User Story
    As a fspec TUI user opening the /model selector with no matched current model
    I want to have the very first arrow-key press move the cursor
    So that I can navigate the model list on the first press instead of pressing twice

  Scenario: First Down press moves the cursor one row when opened with no matched current model
    Given the model selector is opened with no matched current model so every provider is collapsed and every row is a header
    When I press Down once
    Then the cursor moves from row 0 to row 1
    And the selection is now active

  Scenario: First Up press at the top row is a clamped no-move but activates the selection
    Given the model selector is opened with no matched current model so every provider is collapsed and every row is a header
    When I press Up once
    Then the cursor stays on row 0
    And the selection is now active

  Scenario: First PageDown press moves the selection by a viewport step
    Given the model selector is opened with no matched current model on a list taller than the viewport
    When I press PageDown once
    Then the cursor moves down by one viewport step
    And the selection is now active

  Scenario: Enter before any navigation is a no-op on a model row
    Given the model selector is opened with no matched current model and the cursor rests on a model row
    When I press Enter before pressing any arrow key
    Then the key is consumed and no model-selected action is emitted
    And no selection is active

  Scenario: Opening on a matched current model seeds the cursor and Enter selects immediately
    Given my current model is "claude-sonnet"
    When the model selector loads the providers
    Then the cursor is seeded on the selectable row for "claude-sonnet" and the selection is active
    When I press Enter before pressing any arrow key
    Then a model-selected action is emitted for "claude-sonnet"
