@done
@dialog
@persistence
@thinking-detection
@tui
@TUI-058
Feature: Default Thinking Level Persistence
  """
  Implementation:
  - Follow TUI-035 (persist-last-used-model-selection) pattern - use loadConfig/writeConfig from src/utils/config.ts to store tui.defaultThinkingLevel. Create separate defaultThinkingLevelConfig.ts for DRY/SOLID separation. Apply default during session creation in AgentView.tsx similar to lastUsedModel restoration.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ThinkingLevelDialog shows 'D' key option in footer to set current selection as default
  #   2. Default thinking level persisted to ~/.fspec/fspec-config.json under tui.defaultThinkingLevel key
  #   3. When creating new session, base thinking level initialized from persisted default if available
  #   4. If no default is set, new sessions start with Off (0) thinking level (current behavior)
  #   5. Pressing 'D' key in dialog shows status message confirming default was set
  #   6. Setting default does NOT close the dialog - user can still select a different level for current session
  #   7. Dialog displays a '(default)' indicator next to the level that is currently set as default
  #   8. Dialog loads the current default from config on open and displays the indicator
  #   9. When D is pressed, the '(default)' indicator moves to the currently selected level
  #
  # EXAMPLES:
  #   1. User opens /thinking dialog, navigates to High, presses D - status shows 'High set as default' and dialog stays open
  #   2. User starts new session with default set to High - session opens with base thinking level High
  #   3. User starts new session without any default set - session starts with Off (0) thinking level
  #   4. User sets default to Medium via D key, then presses Enter on High - session uses High but Medium remains default for next session
  #   5. Dialog footer shows '↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close'
  #   6. Config file ~/.fspec/fspec-config.json with corrupt JSON - session starts with Off, no error shown
  #   7. Dialog opens with Medium as current default - 'Medium - ~10K tokens, balanced (default)' shown in options
  #   8. Dialog opens with no default set - no '(default)' indicator shown on any level
  #   9. User presses D on High when Medium was default - '(default)' moves from Medium to High
  #
  # ========================================
  Background: User Story
    As a developer using the AI agent TUI
    I want to set a default thinking level for new sessions
    So that have my preferred thinking level automatically applied when starting new agent sessions

  # ----------------------------------------
  # DIALOG UI - Setting Default
  # ----------------------------------------
  Scenario: Set default thinking level via D key
    Given the user has a chat session open
    And the ThinkingLevelDialog is open with High selected
    When the user presses the 'D' key
    Then a status message shows "High set as default for new sessions"
    And the dialog remains open
    And the user can still navigate and select a different level

  Scenario: Dialog footer shows D key option
    Given the user has a chat session open
    When the ThinkingLevelDialog is opened via /thinking command
    Then the dialog footer shows "↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close"

  # ----------------------------------------
  # VISUAL INDICATOR - Default Level Display
  # ----------------------------------------
  Scenario: Dialog shows default indicator when default is set
    Given ~/.fspec/fspec-config.json contains "tui.defaultThinkingLevel": 2
    And the user has a chat session open
    When the ThinkingLevelDialog is opened via /thinking command
    Then the Medium option shows "(default)" indicator
    And no other option shows the "(default)" indicator

  Scenario: Dialog shows no indicator when no default is set
    Given ~/.fspec/fspec-config.json does not contain tui.defaultThinkingLevel
    And the user has a chat session open
    When the ThinkingLevelDialog is opened via /thinking command
    Then no option shows the "(default)" indicator

  Scenario: Default indicator moves when D key is pressed
    Given the user has a chat session open
    And the default thinking level is Medium
    And the ThinkingLevelDialog is open with High selected
    When the user presses the 'D' key
    Then the High option now shows "(default)" indicator
    And the Medium option no longer shows "(default)" indicator

  # ----------------------------------------
  # SESSION INITIALIZATION - Restoring Default
  # ----------------------------------------
  Scenario: Restore default thinking level on new session
    Given ~/.fspec/fspec-config.json contains "tui.defaultThinkingLevel": 3
    When the user starts a new agent session
    Then the session starts with base thinking level High
    And the SessionHeader shows the thinking level indicator

  Scenario: Use Off when no default is set
    Given ~/.fspec/fspec-config.json does not contain tui.defaultThinkingLevel
    When the user starts a new agent session
    Then the session starts with base thinking level Off
    And the SessionHeader does not show a thinking level indicator

  # ----------------------------------------
  # SEPARATION OF CURRENT VS DEFAULT
  # ----------------------------------------
  Scenario: Current session selection is independent of default
    Given the user has set a default thinking level of Medium via D key
    And the ThinkingLevelDialog is open with High selected
    When the user presses Enter to select High
    Then the current session uses High thinking level
    And the default remains Medium for future sessions

  # ----------------------------------------
  # ERROR HANDLING
  # ----------------------------------------
  Scenario: Handle corrupt config gracefully
    Given ~/.fspec/fspec-config.json contains invalid JSON
    When the user starts a new agent session
    Then the session starts with base thinking level Off
    And no error is shown to the user
    And the session is fully functional
