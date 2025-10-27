@done
@board-visualization
@interactive-cli
@tui
@high
@BOARD-009
Feature: Animated shimmer on last changed work unit

  """
  Modify UnifiedBoardLayout.tsx to add shimmer effect. Compute lastChangedWorkUnit using useMemo by finding max(updated) timestamp. Use setInterval with 5-second cycle to toggle shimmer state. Apply chalk color variations based on work unit type and selection state.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Last changed work unit is determined by status/column change, not other field updates
  #   2. Find last changed work unit by computing max(updated) timestamp across all work units
  #   3. Shimmer cycles every 5 seconds with pauses between cycles
  #   4. When multiple work units have same timestamp, pick the one with most recent updated field
  #   5. When last-changed work unit is NOT selected: shimmer text color (white→whiteBright, red→redBright, blue→blueBright)
  #   6. When last-changed work unit IS selected: shimmer background color (bgGreen→bgGreenBright) with black text
  #   7. Shimmer starts on TUI startup for the most recently changed work unit from history
  #   8. Shimmer continues indefinitely until another work unit changes status
  #
  # EXAMPLES:
  #   1. Story TECH-001 with most recent timestamp displays with shimmering white text (white→whiteBright every 5s)
  #   2. Bug BUG-007 with most recent timestamp displays with shimmering red text (red→redBright every 5s)
  #   3. Task TASK-003 with most recent timestamp displays with shimmering blue text (blue→blueBright every 5s)
  #   4. Selected work unit AUTH-001 that is also last-changed displays with shimmering green background (bgGreen→bgGreenBright) and black text
  #   5. On TUI startup, BOARD-008 with updated='2025-10-27T21:30:00Z' is most recent, starts shimmering immediately
  #   6. User moves FEAT-002 from testing to implementing, shimmer stops on BOARD-008 and starts on FEAT-002
  #
  # QUESTIONS (ANSWERED):
  #   Q: What constitutes a 'change' to a work unit?
  #   A: true
  #
  #   Q: How should the shimmer animation work exactly?
  #   A: true
  #
  #   Q: What happens if multiple work units changed columns at the same time?
  #   A: true
  #
  #   Q: What happens when the last changed work unit is also the currently selected work unit?
  #   A: true
  #
  #   Q: Should the shimmer override the work unit type color (white/red/blue) or shimmer the existing type color?
  #   A: true
  #
  #   Q: When does the shimmer start and stop?
  #   A: true
  #
  #   Q: How should Zustand track status changes?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a developer viewing TUI board
    I want to see which work unit was most recently changed
    So that I can quickly identify where recent activity occurred

  Scenario: Story work unit with most recent timestamp displays with shimmering white text
    Given UnifiedBoardLayout renders with work units
    And story work unit TECH-001 has updated timestamp '2025-10-27T22:00:00Z'
    And all other work units have earlier timestamps
    When the board is rendered
    Then TECH-001 should display with white text color
    And TECH-001 text color should shimmer between white and whiteBright
    And the shimmer should cycle every 5 seconds

  Scenario: Bug work unit with most recent timestamp displays with shimmering red text
    Given UnifiedBoardLayout renders with work units
    And bug work unit BUG-007 has updated timestamp '2025-10-27T22:00:00Z'
    And all other work units have earlier timestamps
    When the board is rendered
    Then BUG-007 should display with red text color
    And BUG-007 text color should shimmer between red and redBright
    And the shimmer should cycle every 5 seconds

  Scenario: Task work unit with most recent timestamp displays with shimmering blue text
    Given UnifiedBoardLayout renders with work units
    And task work unit TASK-003 has updated timestamp '2025-10-27T22:00:00Z'
    And all other work units have earlier timestamps
    When the board is rendered
    Then TASK-003 should display with blue text color
    And TASK-003 text color should shimmer between blue and blueBright
    And the shimmer should cycle every 5 seconds

  Scenario: Selected work unit that is also last-changed displays with shimmering green background
    Given UnifiedBoardLayout renders with work units
    And work unit AUTH-001 has updated timestamp '2025-10-27T22:00:00Z'
    And AUTH-001 is the currently selected work unit
    And all other work units have earlier timestamps
    When the board is rendered
    Then AUTH-001 should display with green background color
    And AUTH-001 background color should shimmer between bgGreen and bgGreenBright
    And AUTH-001 should display with black text color
    And the shimmer should cycle every 5 seconds

  Scenario: TUI startup identifies and shimmers most recently changed work unit from history
    Given work-units.json contains work unit BOARD-008 with updated '2025-10-27T21:30:00Z'
    And all other work units have earlier updated timestamps
    When the TUI starts up and loads the board
    Then BOARD-008 should be identified as the last changed work unit
    And BOARD-008 should start shimmering immediately
    And the shimmer should cycle every 5 seconds

  Scenario: Shimmer transfers to newly changed work unit
    Given UnifiedBoardLayout renders with work units
    And work unit BOARD-008 is currently shimmering (most recent)
    When user moves FEAT-002 from testing to implementing status
    And FEAT-002 updated timestamp becomes '2025-10-27T22:05:00Z'
    Then shimmer should stop on BOARD-008
    And shimmer should start on FEAT-002
    And FEAT-002 should shimmer its type color (white/red/blue)


  Scenario: Zustand store passes updated timestamp from work-units.json to components
    Given work-units.json contains work unit with updated timestamp
    When Zustand store loads data from work-units.json
    Then WorkUnit interface must include updated and estimate fields
    And UnifiedBoardLayout receives work units with updated timestamps

