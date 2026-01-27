@done
@input-components
@tui
@ui-enhancement
@TUI-055
Feature: File Search Popup Modal for @ Symbol Input

  """
  Integration point: MultiLineInput.tsx component needs @ symbol detection with regex @(\S*)$
  UI Constraint: Terminal UI using React + Ink framework requires different popup approach than web-based OpenCode
  File Search: Leverage existing ripgrep integration for file searching, need to identify current file search API/function
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Use immediate trigger like OpenCode - popup appears the moment @ is typed and updates in real-time as user types more characters
  #   2. Keep same as OpenCode - insert @filepath format (e.g., @src/components/Button.tsx) when file is selected
  #   3. Yes, existing ripgrep code is already wired up to Blob tool - reuse this existing file search infrastructure
  #   4. Yes, use the existing Glob tool for file pattern matching - this is the ripgrep integration already available in my context
  #   5. Center screen popup like slash command palette - copy its styling for consistency with existing UI patterns
  #   6. Yes, exact same keyboard navigation as slash command palette - Up/Down to highlight, Enter to select, Escape to close, continue typing to filter
  #
  # EXAMPLES:
  #   1. User types '@' and immediately sees popup with recent files, then types '@src' and sees files matching 'src' in real-time
  #   2. User selects 'src/components/Button.tsx' from popup and '@src/components/Button.tsx' is inserted into the text at cursor position
  #   3. User types '@comp' and sees files matching 'comp' like 'src/components/', 'lib/compiler.ts', 'test/compare.test.ts' using fuzzy matching
  #   4. User types '@but' and Glob tool searches '**/*but*' pattern, showing files like 'src/components/Button.tsx', 'lib/utils/debounce.ts', etc.
  #   5. User types '@comp' and center screen popup appears (like slash command palette), showing filtered file list that updates as user continues typing
  #   6. User types '@src', sees file list, uses Up/Down arrows to highlight 'src/components/Button.tsx', presses Enter and '@src/components/Button.tsx' is inserted
  #   7. User types '@nonexistent', popup shows 'No files found' message, user can press Escape to close or continue typing to search again
  #
  # QUESTIONS (ANSWERED):
  #   Q: When should the file search popup appear? Immediately when @ is typed, or after @ + space, or after @ + minimum characters?
  #   A: Use immediate trigger like OpenCode - popup appears the moment @ is typed and updates in real-time as user types more characters
  #
  #   Q: What format should be inserted when user selects a file? Options: @filepath, plain path, [file:path], or custom format?
  #   A: Keep same as OpenCode - insert @filepath format (e.g., @src/components/Button.tsx) when file is selected
  #
  #   Q: Do you have existing file search functionality with ripgrep that I can reuse, or should I research the codebase to find it?
  #   A: Yes, use the existing Glob tool for file pattern matching - this is the ripgrep integration already available in my context
  #
  #   Q: How should the file search popup appear in terminal UI? Overlay above/below current line, side panel, or inline expansion?
  #   A: Center screen popup like slash command palette - copy its styling for consistency with existing UI patterns
  #
  #   Q: Should keyboard navigation work like slash command palette? Up/Down to highlight, Enter to select, Escape to close?
  #   A: Yes, exact same keyboard navigation as slash command palette - Up/Down to highlight, Enter to select, Escape to close, continue typing to filter
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want to reference files quickly while typing
    So that I can insert file paths without interrupting my flow

  Scenario: Immediate popup trigger with real-time file filtering
    Given I am in the MultiLineInput component
    And the input has focus
    When I type "@"
    Then a popup should appear immediately in center screen
    And the popup should show recent files
    When I continue typing "src"
    Then the popup should update in real-time to show files matching "src"

  Scenario: File selection and @filepath insertion
    Given I am typing in the MultiLineInput component
    And the file search popup is open
    And "src/components/Button.tsx" is available in the file list
    When I select "src/components/Button.tsx" from the popup
    Then "@src/components/Button.tsx" should be inserted at the cursor position
    And the popup should close

  Scenario: Fuzzy file matching with partial queries
    Given I am in the MultiLineInput component
    When I type "@comp"
    Then the popup should appear with files matching "comp"
    And the results should include "src/components/" 
    And the results should include "lib/compiler.ts"
    And the results should include "test/compare.test.ts"
    And the matching should be fuzzy (non-contiguous character matching)

  Scenario: Backend file search integration using Glob tool
    Given I am in the MultiLineInput component
    When I type "@but"
    Then the system should use Glob tool with pattern "**/*but*"
    And the popup should show matching files like "src/components/Button.tsx"
    And the popup should show matching files like "lib/utils/debounce.ts"

  Scenario: Center screen popup appearance like slash command palette
    Given I am in the MultiLineInput component
    When I type "@comp"
    Then a popup should appear in the center of the screen
    And the popup should use the same styling as the slash command palette
    And the popup should show a filtered file list
    And the list should update as I continue typing

  Scenario: Keyboard navigation identical to slash command palette
    Given I am in the MultiLineInput component
    And the file search popup is open with multiple results
    When I type "@src"
    Then I should see a list of files containing "src"
    When I press the Down arrow key
    Then the next file should be highlighted
    When I press the Up arrow key  
    Then the previous file should be highlighted
    When I press Enter on "src/components/Button.tsx"
    Then "@src/components/Button.tsx" should be inserted
    And the popup should close

  Scenario: No results found with escape handling
    Given I am in the MultiLineInput component
    When I type "@nonexistent"
    Then the popup should appear
    And the popup should show "No files found" message
    When I press Escape
    Then the popup should close
    And I should return to normal input mode
    When I continue typing more characters
    Then the search should update again
