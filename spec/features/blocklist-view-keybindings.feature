@done
@tui-component
@rust
@agent-view
@tui
@BLOCK-010
Feature: BlocklistView: keyboard parity — remove vim j/k, add PageUp/PageDown/Home/End (arrows + page/home/end only)

  """
  BlocklistView::handle_key (views/blocklist/mod.rs) removes the vim KeyCode::Char('j'/'k') arms and adds KeyCode::PageDown/PageUp/Home/End arms. Paging helpers step the selection by visible_rows.max(1) then call adjust_scroll(), which delegates to the shared components::scroll_viewport::ensure_visible primitive (same as model_selector/provider_settings). All rules are selectable, so no header-skipping is needed. Space/Enter continue to emit Action::ToggleBlocklistRule. FOOTER_HINT in views/blocklist/render.rs is updated to drop /jk and advertise the arrow + Page/Home/End keys. All changes stay within the blocklist view; mouse-wheel support is tracked separately in BLOCK-011.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing j, J, k, or K does not move the selection (vim bindings are removed)
  #   2. Up and Down arrow keys move the selection one rule at a time
  #   3. PageDown advances the selection by up to one viewport (visible_rows) clamped to the last rule, and PageUp retreats by up to one viewport clamped to the first rule
  #   4. Home selects the first rule and End selects the last rule, with scroll_offset reconciled so the selection stays visible
  #   5. Space and Enter still toggle the focused rule's session-disabled status
  #   6. The footer hint no longer advertises jk and instead lists the arrow and Page/Home/End keys
  #
  # EXAMPLES:
  #   1. With 20 rules and visible_rows 8, selection at index 0, pressing PageDown moves selection to index 8 and the window scrolls to keep it visible
  #   2. With the selection at the last rule (index 19), pressing PageDown leaves the selection at index 19 (clamped)
  #   3. With the selection anywhere in the list, pressing End selects the last rule and pressing Home selects the first rule
  #   4. With the selection at index 5, pressing j or k leaves the selection at index 5 unchanged
  #   5. With a rule focused, pressing Space emits a toggle action for that rule's id
  #
  # ========================================

  Background: User Story
    As a keyboard user of the Rust TUI /blocklist view
    I want to navigate the rules list using only arrow keys plus PageUp/PageDown/Home/End
    So that the blocklist view behaves consistently with the model and provider views and does not silently swallow letters as navigation

  Scenario: Vim keys do not move the selection
    Given the blocklist view has 20 rules loaded
    And the selection is at index 5
    When I press the "j" key
    Then the selection stays at index 5
    When I press the "k" key
    Then the selection stays at index 5

  Scenario: Arrow keys move the selection one rule at a time
    Given the blocklist view has 20 rules loaded
    And the selection is at index 5
    When I press the Down arrow key
    Then the selection moves to index 6
    When I press the Up arrow key
    Then the selection moves to index 5

  Scenario: PageDown advances the selection by one viewport
    Given the blocklist view has 20 rules loaded
    And the visible window shows 8 rows
    And the selection is at index 0
    When I press the PageDown key
    Then the selection moves to index 8
    And the visible window scrolls so the selection stays visible

  Scenario: PageDown clamps at the last rule
    Given the blocklist view has 20 rules loaded
    And the visible window shows 8 rows
    And the selection is at the last rule index 19
    When I press the PageDown key
    Then the selection stays at index 19

  Scenario: PageUp retreats the selection by one viewport
    Given the blocklist view has 20 rules loaded
    And the visible window shows 8 rows
    And the selection is at index 8
    When I press the PageUp key
    Then the selection moves to index 0

  Scenario: End selects the last rule and Home selects the first rule
    Given the blocklist view has 20 rules loaded
    And the visible window shows 8 rows
    And the selection is at index 5
    When I press the End key
    Then the selection moves to the last rule index 19
    And the visible window scrolls so the selection stays visible
    When I press the Home key
    Then the selection moves to the first rule index 0

  Scenario: Space toggles the focused rule
    Given the blocklist view has 20 rules loaded
    And the selection is at index 3
    When I press the Space key
    Then a toggle action is emitted for the focused rule's id

  Scenario: Footer hint no longer advertises vim keys
    Given the blocklist view has rules loaded
    When the view is rendered
    Then the footer hint does not contain "jk"
    And the footer hint lists the arrow and Page/Home/End keys
