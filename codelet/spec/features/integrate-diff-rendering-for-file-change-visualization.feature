@done
@interactive
@tool-display
@rendering
@cli
@high
@CLI-007
Feature: Integrate diff rendering for file change visualization
  """
  Integrates existing diff rendering functions from src/cli/diff.rs into interactive mode tool result display. Uses ANSI color codes (green \x1b[32m for additions, red \x1b[31m for deletions). Intercepts Edit and Write tool results in src/cli/interactive.rs and applies render_diff_line formatting before display.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Diff rendering must show additions in green and deletions in red using ANSI codes
  #   2. Diff rendering must include line numbers for each change
  #   3. Diff rendering must show file path at the top of each diff
  #   4. Edit tool results must show before/after diff using render_diff_line function
  #   5. Write tool results must show additions when creating new files
  #
  # EXAMPLES:
  #   1. Agent edits src/main.rs changing 'let x = 3' to 'let x = 5', user sees red line '- let x = 3' and green line '+ let x = 5'
  #   2. Agent writes new file src/utils.rs, user sees all lines in green with '+' prefix
  #   3. Agent edits file at line 42, user sees line number '42' displayed with the change
  #   4. Agent edits src/cli/mod.rs, user sees 'File: src/cli/mod.rs' header before diff lines
  #
  # ========================================
  Background: User Story
    As a developer using codelet in interactive mode
    I want to see visual diffs of file changes when Edit and Write tools execute
    So that I can quickly understand what code was modified without reading entire files

  Scenario: Display diff with color coding for Edit tool changes
    Given the agent is running in interactive mode
    When the agent edits src/main.rs changing 'let x = 3' to 'let x = 5'
    Then the tool result should display a diff
    And the deletion 'let x = 3' should be shown in red with '-' prefix
    And the addition 'let x = 5' should be shown in green with '+' prefix

  Scenario: Display all additions in green for Write tool on new file
    Given the agent is running in interactive mode
    When the agent writes a new file src/utils.rs
    Then the tool result should display all lines in green
    And each line should have a '+' prefix
    And the diff should show the file path 'src/utils.rs'

  Scenario: Display line numbers with diff changes
    Given the agent is running in interactive mode
    When the agent edits a file at line 42
    Then the tool result should display the line number '42'
    And the line number should appear before the change indicator
    And the diff should follow the format '<line_number> <prefix> <content>'

  Scenario: Display file path header in diff output
    Given the agent is running in interactive mode
    When the agent edits src/cli/mod.rs
    Then the tool result should display 'File: src/cli/mod.rs' at the top
    And the file path header should appear before any diff lines
    And the diff lines should follow below the header
