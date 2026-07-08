@done
@agent-view
@tui-component
@rust
@tui
@BLOCK-009
Feature: BlocklistView: adopt full-screen shell scaffold + reference-parity framing (header count, pane divider, scroll indicator)
  """
  BlocklistView::render adopts the shared full_screen_shell scaffold (render_full_screen_scaffold with count-title 'Blocklist Rules'/'rules'), a [Percentage(50), Length(1), Percentage(50)] body split with diff_common::render_vertical_divider in the middle gutter, and a footer hint. (BLOCK-010 superseded the footer wording; it now reads '↑↓ Navigate | PgUp/PgDn/Home/End: Scroll | Enter/Space: Toggle Rule | Esc: Close'.) Reuses shared helpers (DRY). Preserves all RPC-056 + BLOCK-008 behaviour (categories, glyphs, session-disabled, windowed scrolling, Showing indicator, empty-state, handle_key). Files under 300 lines, clippy clean, no unwrap/expect/todo in non-test code.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The view uses the shared full_screen_shell scaffold (render_full_screen_scaffold / _raw_title) instead of a hand-rolled bordered Block, matching model_selector/changed_files/provider_settings
  #   2. The header title includes the rules count, rendered as 'Blocklist Rules (N rules)' (parity with the TS reference and the count-title scaffold used by sibling views)
  #   3. A vertical divider is drawn between the left list pane and the right details pane using the shared diff_common::render_vertical_divider helper
  #   4. The footer hint reads '↑↓ Navigate | PgUp/PgDn/Home/End: Scroll | Enter/Space: Toggle Rule | Esc: Close' (BLOCK-010 superseded the earlier '↑↓/jk' wording by removing the vim bindings)
  #   5. The category system, store-backed session-disabled persistence, and event-dispatcher architecture from RPC-056 are preserved and MUST NOT be reverted; all existing RPC-056 render assertions still hold
  #
  # EXAMPLES:
  #   1. A BlocklistView with 2 rules is rendered; the header row contains 'Blocklist Rules (2 rules)'
  #   2. A BlocklistView with rules is rendered; a vertical divider column separates the list pane from the details pane
  #   3. A BlocklistView is rendered; the footer row contains 'Enter/Space: Toggle Rule' and 'Esc: Close'
  #   4. After the framing change, the RPC-056 render still shows rule ids, 'system'/'project' source tags, 'file_path'/'bash' categories, ○/● glyphs, '(disabled)' suffix, and the empty-state 'No blocklist rules configured' text
  #
  # ========================================
  Background: User Story
    As a fspec TUI user opening the /blocklist view
    I want to see the same framing/chrome the reference and sibling views use (rules-count header, a divider between panes, a scroll indicator, and a clear footer)
    So that the blocklist view feels consistent with the rest of the app and matches the TypeScript reference

  Scenario: Header shows the rules count
    Given a BlocklistView seeded with 2 rules
    When the view is rendered into a 120x24 buffer
    Then the rendered header contains "Blocklist Rules (2 rules)"

  Scenario: A vertical divider separates the list and details panes
    Given a BlocklistView seeded with rules
    When the view is rendered into a 120x24 buffer
    Then a vertical divider column separates the list pane from the details pane

  Scenario: Footer shows the reference-parity hint
    Given a BlocklistView seeded with rules
    When the view is rendered into a 120x24 buffer
    Then the rendered footer contains "Enter/Space: Toggle Rule"
    And the rendered footer contains "Esc: Close"

  Scenario: RPC-056 rendering behaviour is preserved after the framing change
    Given a BlocklistView seeded with rules [git-checkout-block(system, block), cat-block(project, block)]
    And the focused session's blocklist_disabled set contains "git-checkout-block"
    When the view is rendered into a 120x24 buffer
    Then the rendered text contains "git-checkout-block"
    And the rendered text contains "system"
    And the rendered text contains "project"
    And the rendered text contains "○ git-checkout-block"
    And the rendered text contains "(disabled)"
    When the view is re-seeded with an empty rule list
    And the view is rendered again
    Then the rendered text contains "No blocklist rules configured"
