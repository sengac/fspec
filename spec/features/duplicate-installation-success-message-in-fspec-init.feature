@done
@multi-agent-support
@bug
@initialization
@cli
@high
@BUG-040
Feature: Duplicate installation success message in fspec init
  """
  Uses Commander.js for CLI output and success message formatting. The init command installs fspec for various AI agents (Claude, Cursor, Cline, etc.). Success messages are currently displayed in two places: once with detailed file list and next steps, and a duplicate final message that needs removal. The fix involves removing the redundant final console.log statement that outputs the duplicate message.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Only one success message should be displayed after installation completes
  #   2. The success message should include agent name, installed files, and next steps
  #   3. The duplicate final message ("✓ Installed fspec for Claude Code") should be removed
  #
  # EXAMPLES:
  #   1. Running 'fspec init' for Claude Code shows: '✓ Installed fspec for claude' with file list and next steps, then '✓ Installed fspec for Claude Code' (CURRENT BEHAVIOR - duplicate)
  #   2. Running 'fspec init' for Claude Code should show: '✓ Installed fspec for claude' with file list and next steps, WITHOUT the final duplicate message (EXPECTED BEHAVIOR)
  #   3. Other agents (Cursor, Cline, Windsurf, etc.) should also show only one success message without duplicates
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to see clear non-duplicate success messages after initialization
    So that I understand installation status without confusion

  Scenario: Initialize fspec for Claude Code without duplicate message
    Given I am in a project directory without fspec initialized
    When I run 'fspec init' and select Claude Code
    Then the output should display '✓ Installed fspec for claude'
    And the output should include the list of installed files
    And the output should include 'Next steps: Run /fspec in Claude Code to activate'
    And the output should NOT contain a duplicate '✓ Installed fspec for Claude Code' message

  Scenario: Initialize fspec for other agents without duplicate message
    Given I am in a project directory without fspec initialized
    When I run 'fspec init' and select an agent (Cursor, Cline, or Windsurf)
    Then the output should display exactly one success message for the selected agent
    And the output should NOT contain any duplicate success messages
