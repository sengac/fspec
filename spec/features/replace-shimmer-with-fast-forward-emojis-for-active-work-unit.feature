@high
@tui
@ui-enhancement
@TUI-017
Feature: Replace shimmer with fast-forward emojis for active work unit

  """
  Architecture notes:
  - Replace shimmer animation with static emoji indicators (⏩) in checkpoint panel
  - Modify WorkUnitRow component in dev-tui.tsx to conditionally render emojis around active work unit
  - Remove gradient and shimmer effect logic from Ink Box component styling
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Active work unit must display fast-forward emoji (⏩) on both left and right sides
  #   2. Shimmer effect must be completely removed from active work unit display
  #   3. Format must be: ⏩ WORK-UNIT-ID [points] ⏩ with single spaces between elements
  #   4. Non-active work units must not display the fast-forward emojis
  #
  # EXAMPLES:
  #   1. When TUI-015 [3] is active, display shows: ⏩ TUI-015 [3] ⏩
  #   2. When TUI-016 [5] is not active, display shows: TUI-016 [5] (no emojis)
  #   3. When switching from TUI-015 to TUI-016, emojis move from TUI-015 to TUI-016
  #   4. Shimmer animation that previously highlighted active work unit is no longer visible
  #
  # ========================================

  Background: User Story
    As a developer using the TUI
    I want to see which work unit is currently active with fast-forward emojis
    So that I can quickly identify the active work without visual distraction from shimmer effects

  Scenario: Display fast-forward emojis around active work unit
    Given the TUI is displaying the checkpoint panel
    And work unit "TUI-015" with 3 story points exists
    And "TUI-015" is the currently active work unit
    When I view the work unit row for "TUI-015"
    Then I should see "⏩ TUI-015 [3] ⏩"
    And the emojis should be separated by single spaces

  Scenario: Display non-active work unit without emojis
    Given the TUI is displaying the checkpoint panel
    And work unit "TUI-016" with 5 story points exists
    And "TUI-016" is not the currently active work unit
    When I view the work unit row for "TUI-016"
    Then I should see "TUI-016 [5]"
    And I should not see any fast-forward emojis around the work unit

  Scenario: Emojis move when active work unit changes
    Given the TUI is displaying the checkpoint panel
    And work unit "TUI-015" with 3 story points exists
    And work unit "TUI-016" with 5 story points exists
    And "TUI-015" is the currently active work unit
    When I switch the active work unit to "TUI-016"
    Then "TUI-015" should be displayed as "TUI-015 [3]" without emojis
    And "TUI-016" should be displayed as "⏩ TUI-016 [5] ⏩" with emojis

  Scenario: Shimmer animation is removed from active work unit
    Given the TUI is displaying the checkpoint panel
    And work unit "TUI-015" exists
    And "TUI-015" is the currently active work unit
    When I view the work unit row for "TUI-015"
    Then I should not see any shimmer or gradient animation effects
    And I should only see the fast-forward emojis as the active indicator
