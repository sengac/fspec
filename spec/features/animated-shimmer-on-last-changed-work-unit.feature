@done
@board-visualization
@interactive-cli
@tui
@high
@BOARD-009
Feature: Animated shimmer on last changed work unit
  """
  Architecture notes:
  - Implement character-by-character shimmer wave that moves left-to-right across work unit text
  - Shimmer effect uses 3-level brightness gradient: dim (gray) → base color → bright
  - At any frame, one character is at peak brightness with gradient falloff on both sides
  - Animation moves one character position per frame at ~100ms interval for smooth visual effect
  - Compute lastChangedWorkUnit using useMemo by finding max(updated) timestamp
  - Use setInterval to advance shimmer position (not toggle entire string)
  - Apply chalk color variations based on work unit type: white/red/blue → whiteBright/redBright/blueBright
  - For selected+last-changed: apply gradient to background (bgGreen → bgGreenBright) with black text
  - Loop continuously: when shimmer reaches end of string, restart from beginning
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
  #   6. Shimmer starts on TUI startup for the most recently changed work unit from history
  #   7. Shimmer continues indefinitely until another work unit changes status
  #   8. Zustand store WorkUnit interface must include updated and estimate fields to pass data from work-units.json to components
  #   9. Shimmer effect animates character-by-character from left to right across the entire string
  #   10. At any given frame, one character is at peak brightness (whiteBright/redBright/blueBright), with darker gradient on both sides
  #   11. Gradient uses 3 brightness levels: dim (gray), base color (white/red/blue), bright (whiteBright/redBright/blueBright)
  #   12. Shimmer wave moves one character position per frame at smooth animation speed
  #   13. When shimmer reaches end of string, it loops back to the beginning continuously
  #
  # EXAMPLES:
  #   1. Story TECH-001 with most recent timestamp displays with shimmering white text (white→whiteBright every 5s)
  #   2. Bug BUG-007 with most recent timestamp displays with shimmering red text (red→redBright every 5s)
  #   3. Task TASK-003 with most recent timestamp displays with shimmering blue text (blue→blueBright every 5s)
  #   4. Selected work unit AUTH-001 that is also last-changed displays with shimmering green background (bgGreen→bgGreenBright) and black text
  #   5. On TUI startup, BOARD-008 with updated='2025-10-27T21:30:00Z' is most recent, starts shimmering immediately
  #   6. User moves FEAT-002 from testing to implementing, shimmer stops on BOARD-008 and starts on FEAT-002
  #   7. Zustand store loads work unit with updated='2025-10-27T22:00:00Z' from work-units.json and passes it to UnifiedBoardLayout
  #   8. String 'FEAT-001' with shimmer at position 0: [F]EAT-001 where F is whiteBright, E is white, A is gray
  #   9. String 'FEAT-001' with shimmer at position 3: FEA[T]-001 where T is whiteBright, A and - are white, E and 0 are gray
  #   10. Bug work unit 'BUG-007' uses red gradient: gray → red → redBright → red → gray
  #   11. Task work unit 'TASK-003' uses blue gradient: gray → blue → blueBright → blue → gray
  #   12. Animation speed: shimmer moves 1 character position every 100ms for smooth visual wave effect
  #   13. For selected work unit that is also last-changed: apply shimmer gradient to background color (bgGreen → bgGreenBright → bgGreen) with black text
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

  Scenario: Story work unit with most recent timestamp displays with character-by-character shimmer wave
    Given UnifiedBoardLayout renders with work units
    And story work unit TECH-001 has updated timestamp '2025-10-27T22:00:00Z'
    And all other work units have earlier timestamps
    When the board is rendered
    Then TECH-001 should display with a left-to-right shimmer wave
    And at any frame one character should be at peak brightness (whiteBright)
    And adjacent characters should use gradient: gray → white → whiteBright → white → gray
    And the shimmer should advance one character position every 100ms
    And the shimmer should loop continuously from start when reaching the end

  Scenario: Bug work unit with most recent timestamp displays with red gradient shimmer wave
    Given UnifiedBoardLayout renders with work units
    And bug work unit BUG-007 has updated timestamp '2025-10-27T22:00:00Z'
    And all other work units have earlier timestamps
    When the board is rendered
    Then BUG-007 should display with a left-to-right shimmer wave
    And at any frame one character should be at peak brightness (redBright)
    And adjacent characters should use red gradient: gray → red → redBright → red → gray
    And the shimmer should advance one character position every 100ms
    And the shimmer should loop continuously from start when reaching the end

  Scenario: Task work unit with most recent timestamp displays with blue gradient shimmer wave
    Given UnifiedBoardLayout renders with work units
    And task work unit TASK-003 has updated timestamp '2025-10-27T22:00:00Z'
    And all other work units have earlier timestamps
    When the board is rendered
    Then TASK-003 should display with a left-to-right shimmer wave
    And at any frame one character should be at peak brightness (blueBright)
    And adjacent characters should use blue gradient: gray → blue → blueBright → blue → gray
    And the shimmer should advance one character position every 100ms
    And the shimmer should loop continuously from start when reaching the end

  Scenario: Selected work unit that is also last-changed displays with green background gradient shimmer
    Given UnifiedBoardLayout renders with work units
    And work unit AUTH-001 has updated timestamp '2025-10-27T22:00:00Z'
    And AUTH-001 is the currently selected work unit
    And all other work units have earlier timestamps
    When the board is rendered
    Then AUTH-001 should display with a left-to-right shimmer wave on the background
    And at any frame one character should have peak brightness background (bgGreenBright)
    And adjacent characters should use green background gradient: bgGreen → bgGreenBright → bgGreen
    And all characters should display with black text color
    And the shimmer should advance one character position every 100ms
    And the shimmer should loop continuously from start when reaching the end

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
