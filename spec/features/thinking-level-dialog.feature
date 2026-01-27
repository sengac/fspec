@done
@thinking-detection
@dialog
@tui
@TUI-054
Feature: ThinkingLevelDialog for /thinking command
  """
  Effective level calculation: Math.max(baseLevel, detectedLevel), except disable keywords always force Off
  /thinking command accepts optional level argument: off, low, med/medium, high (case insensitive)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ThinkingLevelDialog must display 4 options: Off (0), Low (1), Medium (2), High (3)
  #   2. Navigation must use up/down arrow keys to move selection
  #   3. Enter key must confirm selection and close dialog, updating base thinking level
  #   4. Escape key must close dialog without changing the thinking level
  #   5. The selected base level persists for all subsequent requests in the session
  #   6. Effective thinking level is max(baseLevel, detectedLevelFromText) - text keywords can only INCREASE the level
  #   7. Disable keywords (quickly, briefly, short, fast, rapid) always force level to Off regardless of base level
  #   8. Dialog must use CRITICAL input priority via useInputCompat to capture all keyboard input
  #   9. Slash command palette must be hidden/disabled when ThinkingLevelDialog is open
  #   10. Current thinking level must be visually indicated (highlighted/selected) when dialog opens
  #   11. Base thinking level stored in Rust BackgroundSession as AtomicU8, synced via useRustSessionState hook
  #   12. SessionHeader shows thinking level badge only when level > Off, shows effective level (max of base and detected) during loading
  #   13. /thinking command requires an active session - shows error message if no session
  #   14. /thinking accepts optional level argument: off, low, med/medium, high (case insensitive)
  #   15. /thinking with no argument opens the dialog, /thinking with argument sets level directly
  #
  # EXAMPLES:
  #   1. User types /thinking at start of input, ThinkingLevelDialog appears with current level highlighted
  #   2. Dialog shows Off first, user presses down twice to reach Medium, presses Enter - base level is now Medium
  #   3. User presses up arrow on Off - selection wraps to High (last option)
  #   4. User presses Escape while dialog is open - dialog closes, base level remains unchanged
  #   5. Base level is High, user types 'explain this code' with no keywords - effective level is High
  #   6. Base level is Medium, user types 'ultrathink about architecture' - effective level is High (ultrathink > Medium)
  #   7. Base level is High, user types 'quickly list files' - effective level is Off (disable keywords override base)
  #   8. User opens /thinking dialog while typing '/thi' - slash command palette closes, ThinkingLevelDialog opens
  #   9. ThinkingLevelDialog renders with accessible labels: Off (no thinking), Low (~4K tokens), Medium (~10K), High (~32K)
  #   10. Base level is Off (default), SessionHeader shows no thinking badge
  #   11. User sets base level to Medium via /thinking, SessionHeader shows [T:Med] badge even when idle
  #   12. User types '/thinking high' - base level set to High without opening dialog
  #   13. User types '/thinking MED' - base level set to Medium (case insensitive)
  #   14. User types '/thinking invalid' - error message shown, level unchanged
  #   15. User types '/thinking' without a session - error message prompts to start session first
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the dialog show the current base level when opened, or always start at Off?
  #   A: Show current level when opened. State stored in Rust BackgroundSession as AtomicU8, synced via useRustSessionState hook. Follow pause_state pattern: add base_thinking_level field, getter/setter methods, NAPI functions (session_get_base_thinking_level, session_set_base_thinking_level), extend RustStateSource/RustSessionSnapshot.
  #
  #   Q: Should arrow key navigation wrap around (High→Off and Off→High), or stop at boundaries?
  #   A: Yes, wrap around. Down on High wraps to Off, Up on Off wraps to High.
  #
  #   Q: Should the current base thinking level be displayed in the SessionHeader, similar to how API key status is shown?
  #   A: Yes. Extend SessionHeader to show base level only when > Off. During loading, show effective level (max of base and detected). Base level stored in Rust BackgroundSession as AtomicU8, synced via useRustSessionState. Add baseThinkingLevel to RustSessionSnapshot.
  #
  #   Q: Should the /thinking command be available at any point in the input, or only at the start (like other slash commands)?
  #   A: Only at start of input, like other slash commands. Follow existing slash command pattern - detected when input starts with '/' and matched against SLASH_COMMANDS registry.
  #
  # ========================================
  Background: User Story
    As a developer using the AI chat TUI
    I want to select a thinking level using the /thinking command
    So that I can control how much thinking time the AI uses for my requests without typing keywords like ultrathink

  Scenario: Open thinking level dialog with slash command
    Given the user has a chat session open
    When the user types '/thinking' at the start of input
    Then the ThinkingLevelDialog appears
    And the current level Off is highlighted
    And the dialog shows 4 options: Off, Low, Medium, High

  Scenario: Navigate and select thinking level
    Given the ThinkingLevelDialog is open with Off selected
    When the user presses down arrow twice
    Then Medium is highlighted
    When the user presses Enter
    Then the dialog closes
    And the base thinking level is set to Medium

  Scenario: Navigation wraps around at boundaries
    Given the ThinkingLevelDialog is open with Off selected
    When the user presses up arrow
    Then High is highlighted (wraps from Off to High)

  Scenario: Cancel dialog with Escape
    Given the ThinkingLevelDialog is open
    When the user presses Escape
    Then the dialog closes
    And the base thinking level remains Medium

  Scenario: Base level persists and applies to requests without keywords
    Given the base thinking level is set to High
    When the user submits 'explain this code'
    Then the effective thinking level is High

  Scenario: Text keywords can increase but not decrease effective level
    Given the base thinking level is set to Medium
    When the user submits 'ultrathink about architecture'
    Then the effective thinking level is High (ultrathink overrides Medium)

  Scenario: Disable keywords override base level
    Given the base thinking level is set to High
    When the user submits 'quickly list files'
    Then the effective thinking level is Off (disable keywords always win)

  Scenario: Slash command palette closes when thinking dialog opens
    Given the user is typing '/thi' in the input
    When the user selects the /thinking command
    Then the slash command palette closes
    And the input is cleared
    And the ThinkingLevelDialog appears

  Scenario: Dialog displays accessible labels with token budgets
    Given the ThinkingLevelDialog is open
    When the user views the dialog options
    Then the dialog shows 'Off' with description 'No extended thinking'
    And the dialog shows 'Low' with description '~4K tokens'
    And the dialog shows 'Medium' with description '~10K tokens'
    And the dialog shows 'High' with description '~32K tokens'

  Scenario: SessionHeader shows base level when idle
    Given the base thinking level is Off
    When the user sets the base level to Medium via /thinking
    Then the SessionHeader shows '[T:Med]' badge even while idle

  Scenario: Dialog opens with current base level highlighted
    Given the base thinking level is set to High
    When the user opens the ThinkingLevelDialog via /thinking
    Then High is highlighted (not Off)

  Scenario: Set thinking level directly with argument
    Given the user has a chat session open
    When the user types '/thinking high'
    Then the base thinking level is set to High
    And a status message confirms 'Thinking level set to High.'

  Scenario: Arguments are case insensitive
    Given the user has a chat session open
    When the user types '/thinking MED'
    Then the base thinking level is set to Medium

  Scenario: Invalid argument shows error
    Given the user has a chat session open
    When the user types '/thinking invalid'
    Then an error message shows 'Invalid thinking level "invalid". Use: off, low, med, medium, or high.'
    And the base thinking level remains unchanged

  Scenario: Command requires active session
    Given no session is active
    When the user types '/thinking'
    Then an error message shows 'Start a session first to set the thinking level.'

  Scenario: SessionHeader hides badge when level is Off
    Given the base thinking level is Off
    Then the SessionHeader does not show a thinking level badge
