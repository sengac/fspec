@done
@rendering
@cli
@high
@CLI-006
Feature: Enhanced text styling with bash highlighting and diff rendering
  """
  Uses tree-sitter-highlight with tree-sitter-bash for bash syntax highlighting. Uses diffy crate for unified diff parsing. Bash highlighting only for command preview overlays (ApprovalRequest::Exec), not markdown. Diff rendering converts FileChange to ratatui Lines with green (Color::Green) for additions, red (Color::Red) for deletions.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Bash syntax highlighting must use tree-sitter-bash with minimal styling (dim for comments, operators, strings)
  #   2. Diff rendering must show additions in green and deletions in red
  #   3. Bash highlighting is only for command preview overlays, not for markdown code blocks
  #   4. Diff rendering must include line numbers and file paths
  #
  # EXAMPLES:
  #   1. User sees bash command 'echo "hi" && bar | qux' with "hi", &&, and | dimmed
  #   2. User sees diff with '+5 -2' showing 5 additions in green and 2 deletions in red
  #   3. User sees diff line '1 + new line' with green color for addition
  #   4. User sees bash command 'cat file.txt' with 'cat' in default style and 'file.txt' unmodified
  #
  # ========================================
  Background: User Story
    As a developer using codelet CLI
    I want to see bash commands with syntax highlighting and file changes with diff colors
    So that I can quickly understand what commands will execute and what changes were made to files

  Scenario: Highlight bash command with dimmed strings and operators
    Given the agent shows a bash command for approval
    When the command is 'echo "hi" && bar | qux'
    Then the string "hi" should be dimmed
    And the operator && should be dimmed
    And the operator | should be dimmed
    And the command 'echo' should be in default style
    And the text 'bar' and 'qux' should be in default style

  Scenario: Show diff summary with addition and deletion counts
    Given the agent has made changes to files
    When the diff summary shows '+5 -2'
    Then the text '+5' should be displayed in green
    And the text '-2' should be displayed in red
    And the summary should show 5 additions and 2 deletions

  Scenario: Show diff line with addition in green
    Given the agent has added a line to a file
    When the diff shows line '1 + new line'
    Then the line number '1' should be displayed
    And the '+' prefix should be displayed
    And the text 'new line' should be displayed in green

  Scenario: Show bash command with non-dimmed elements
    Given the agent shows a bash command for approval
    When the command is 'cat file.txt'
    Then the command 'cat' should be in default style
    And the argument 'file.txt' should be in default style
    And no dimming should be applied

  Scenario: Bash highlighting only for command previews not markdown
    Given the agent outputs a markdown code block with bash
    When the code block contains '```bash\necho "test"\n```'
    Then the code block should be rendered in cyan
    And NO bash syntax highlighting should be applied
    And the content should preserve the literal bash syntax

  Scenario: Diff rendering includes line numbers and file paths
    Given the agent has modified a file 'src/main.rs'
    When the diff is displayed
    Then the file path 'src/main.rs' should be shown
    And each changed line should have a line number
    And additions should show line numbers from the new file
    And deletions should show line numbers from the old file
