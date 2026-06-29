@done
@agent-view
@scrollback
@tui-component
@rust
@RPC-381
Feature: Port AgentView Tab turn-selection (SELECT) mode to Rust

  """
  Mode toggle lives on AgentView.turn_select_mode (presentation state, mirrors TS component-level isTurnSelectMode). Selection cursor lives on ScrollbackList (SelectionMode { Scroll, Item } + selected index, keyed by stable chunk seq for stream-stability). A turn = one RenderedChunk (Rust chunks are already at message granularity; no TS-style separator-line group walking needed).
  Tab handler added in views/agent/dispatch.rs AFTER popup/mode-view routing (so popups consume Tab first). New actions: ToggleTurnSelectMode, TurnNavUp, TurnNavDown in components/mod.rs, reduced on the App task. Header badge wired by replacing the hardwired is_select_mode: false in chrome_paint.rs:60. Arrow bars rendered in scrollback_paint.rs (gray bg) via generate_arrow_bar (port of turnSelection.ts generateArrowBar).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing Tab toggles turn-selection mode on and off (when no popup or mode view is active)
  #   2. Entering turn-selection mode auto-selects the most-recent (last) turn in the scrollback
  #   3. While in turn-selection mode, Up/Down arrows move the selection to the previous/next turn and clamp at the first/last turn
  #   4. The selected turn is framed by a gray down-arrow bar above it and a gray up-arrow bar below it
  #   5. The session header shows a [SELECT] badge while turn-selection mode is active
  #   6. While in turn-selection mode, pressing Enter does not submit the input
  #   7. Pressing Esc while in turn-selection mode exits the mode and consumes the key (it does not trigger the normal Esc cascade)
  #   8. The selection stays pinned to the same turn (by stable seq) when new chunks stream into the scrollback
  #
  # EXAMPLES:
  #   1. User with a 3-turn conversation presses Tab; the third (last) turn becomes selected and the header shows [SELECT]
  #   2. In select mode on the last turn, user presses Up once; the second turn becomes selected with arrow bars framing it
  #   3. In select mode on the first turn, user presses Up; the selection stays on the first turn (clamped)
  #   4. In select mode, user types text and presses Enter; the text is not sent and remains in the input
  #   5. In select mode, user presses Esc; select mode turns off, the [SELECT] badge disappears, and the app does not start exiting
  #   6. User selects the second of three turns, then the agent streams a new reply; the selection stays on the originally-selected second turn
  #   7. While a slash-command popup is open, pressing Tab is handled by the popup and does NOT toggle turn-selection mode
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to press Tab to enter a turn-selection mode and navigate between conversation turns with arrow keys
    So that I can pick a specific past turn in the scrollback (matching the TypeScript reference TUI)

  Scenario: Pressing Tab enters turn-selection mode and selects the last turn
    Given an AgentView with a conversation of three turns
    And no popup or mode view is active
    When I press the Tab key
    Then turn-selection mode becomes active
    And the third turn is the selected turn
    And the session header shows the [SELECT] badge

  Scenario: Pressing Tab again exits turn-selection mode
    Given an AgentView in turn-selection mode
    When I press the Tab key
    Then turn-selection mode becomes inactive
    And the session header does not show the [SELECT] badge

  Scenario: Up arrow selects the previous turn
    Given an AgentView in turn-selection mode with the last of three turns selected
    When I press the Up arrow key
    Then the second turn is the selected turn
    And a gray down-arrow bar is rendered above the selected turn
    And a gray up-arrow bar is rendered below the selected turn

  Scenario: Up arrow on the first turn clamps the selection
    Given an AgentView in turn-selection mode with the first of three turns selected
    When I press the Up arrow key
    Then the first turn is still the selected turn

  Scenario: Down arrow on the last turn clamps the selection
    Given an AgentView in turn-selection mode with the last of three turns selected
    When I press the Down arrow key
    Then the last turn is still the selected turn

  Scenario: Enter does not submit input while in turn-selection mode
    Given an AgentView in turn-selection mode
    And the input contains the text "hello"
    When I press the Enter key
    Then no input is submitted
    And the input still contains the text "hello"

  Scenario: Esc exits turn-selection mode without triggering the exit cascade
    Given an AgentView in turn-selection mode
    When I press the Esc key
    Then turn-selection mode becomes inactive
    And the session header does not show the [SELECT] badge
    And the normal Esc exit cascade is not triggered

  Scenario: Selection stays pinned to the same turn when new content streams in
    Given an AgentView in turn-selection mode with the second of three turns selected
    When the agent streams a new turn into the scrollback
    Then the second turn is still the selected turn

  Scenario: Tab is consumed by an open slash-command popup
    Given an AgentView with the slash-command popup open
    When I press the Tab key
    Then the slash-command popup handles the key
    And turn-selection mode does not become active
