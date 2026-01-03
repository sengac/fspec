@tool-display
@tui
@TUI-038
Feature: Diff View for Write/Edit Tool Output

  """
  Changes are in AgentModal.tsx only (TUI component). Reuses diff formatting from diff-parser.ts and color scheme from FileDiffViewer.tsx. Edit tool receives old_string and new_string which can be directly diffed. Write tool receives file content; for new files all lines are additions, for existing files would need full file diff (not in scope for initial implementation). Uses Ink's Text component with backgroundColor prop for colored output.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Edit tool shows old_string lines as removed (red background #8B0000) and new_string lines as added (green background #006400)
  #   2. Write tool shows all content as additions (green) for new files, or full diff for existing file overwrites
  #   3. Diff colors match FileDiffViewer style: white text on dark red/green backgrounds
  #   4. Diff view uses tree connector pattern: L prefix on first line, indent on subsequent lines
  #   5. Long diffs are collapsed with '... +N lines (ctrl+o to expand)' indicator
  #
  # EXAMPLES:
  #   1. Edit replaces 'const x = 1' with 'const x = 2': shows '-const x = 1' in red, '+const x = 2' in green
  #   2. Write creates new file with 3 lines: all 3 lines shown with '+' prefix in green
  #   3. Edit with multi-line replacement: consecutive removed lines grouped together, followed by consecutive added lines
  #   4. Edit with 100+ line diff: first 4 lines visible, then '... +96 lines (ctrl+o to expand)' indicator
  #
  # ========================================

  Background: User Story
    As a developer using AI agents
    I want to see colored diff output when Write/Edit tools modify files
    So that I can quickly understand what changes were made with visual red/green highlighting

  Scenario: Edit tool shows single line replacement with diff colors
    Given the agent executes an Edit tool replacing 'const x = 1' with 'const x = 2'
    When the tool result is displayed in the TUI
    Then the removed line shows '-const x = 1' with red background and the added line shows '+const x = 2' with green background


  Scenario: Write tool shows new file content as additions
    Given the agent executes a Write tool creating a new file with 3 lines of content
    When the tool result is displayed in the TUI
    Then all 3 lines are shown with '+' prefix and green background


  Scenario: Edit tool shows multi-line replacement with grouped changes
    Given the agent executes an Edit tool replacing 3 lines with 2 new lines
    When the tool result is displayed in the TUI
    Then the 3 removed lines are grouped together with red background followed by the 2 added lines grouped together with green background


  Scenario: Long diff output is collapsed with expand indicator
    Given the agent executes an Edit tool with a 100+ line diff
    When the tool result is displayed in the TUI
    Then the first 4 lines of the diff are visible followed by '... +96 lines (ctrl+o to expand)' indicator


  Scenario: Diff output uses tree connector pattern
    Given the agent executes an Edit tool with a multi-line diff
    When the tool result is displayed in the TUI
    Then the first diff line has 'L ' prefix and subsequent lines have two-space indent

