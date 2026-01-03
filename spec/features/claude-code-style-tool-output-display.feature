@done
@tool-display
@tui
@TUI-037
Feature: Claude Code Style Tool Output Display

  """
  Architecture notes:
  - Changes are in AgentModal.tsx only (TUI component)
  - Tool output formatting changes from [Tool result preview] to ● ToolName(command) format
  - Streaming output uses fixed 10-line scrolling window
  - Collapsed state shows ~3-4 lines with expand indicator
  - Bug fix: Tool header must persist (currently disappears for Bash)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tool header must display as bullet point followed by ToolName(command/args) - e.g., '● Bash(fspec bootstrap)'
  #   2. Tool output content must use tree connectors (L character with vertical lines) for nested/indented content display
  #   3. Long tool output must be collapsible with '... +N lines (ctrl+o to expand)' indicator showing remaining line count
  #   4. Initial display shows first few lines of output before collapse indicator, user can press ctrl+o to expand full content
  #   5. During Bash streaming, output is displayed in a fixed-height window (e.g., 10 lines) that scrolls - new lines push old lines up and out of view
  #   6. Tool call header (e.g., '● Bash(command)') must ALWAYS remain visible - before, during, and after execution - it should never disappear
  #   7. This display format applies to ALL tool types, not just Bash
  #   8. Changes are TUI only (AgentModal.tsx) - CLI output.rs is not in scope
  #
  # EXAMPLES:
  #   1. Bash tool shows: '● Bash(fspec --sync-version 0.9.3)' with output tree-structured below using L connectors
  #   2. Long output (753 lines) shows first 3-4 lines then '... +753 lines (ctrl+o to expand)' indicator
  #   3. system-reminder content displayed as nested tree node with L connector: 'L <system-reminder>' followed by indented content
  #   4. While fspec bootstrap runs, user sees last 10 lines of output scrolling in real-time, not the full 753+ lines flooding the screen
  #   5. After 'fspec bootstrap' completes, user still sees '● Bash(fspec bootstrap)' header above the collapsed output - it never vanishes
  #
  # QUESTIONS (ANSWERED):
  #   Q: How many lines of tool output should be shown before collapsing? (Claude Code appears to show ~3-4 lines)
  #   A: About 10 lines for streaming window, and ~3-4 lines shown in collapsed state after completion
  #
  #   Q: Should this apply to all tool types (Bash, Read, Edit, Write, Grep, Glob) or just Bash?
  #   A: All tools (Bash, Read, Edit, Write, Grep, Glob, etc.) should use this display format
  #
  #   Q: Should the expand/collapse be toggleable (press ctrl+o to expand, then ctrl+o again to collapse)?
  #   A: No toggle in this story - just expand with ctrl+o, no collapse back
  #
  #   Q: This change affects both AgentModal.tsx (TUI) and output.rs (CLI). Should both be updated, or just the TUI?
  #   A: TUI only (AgentModal.tsx) - CLI output.rs is not used
  #
  #   Q: For the streaming window - should it be exactly 10 lines, or configurable? And should it show a visual indicator that more content exists above (e.g., scroll bar or '... N lines above')?
  #   A: Fixed 10 lines for streaming window is fine
  #
  #   Q: Is this a bug fix (tool header currently disappearing) combined with the new display format? Or is the disappearing header a separate issue to track?
  #   A: It's a bug - fix the disappearing tool header as part of this story
  #
  # ========================================

  Background: User Story
    As a developer using fspec/codelet agent
    I want to see tool outputs displayed in Claude Code's visual style
    So that I have a familiar, consistent experience matching Claude Code's polished UI

  Scenario: Tool header displays in Claude Code format
    Given the agent executes a Bash tool with command "fspec --sync-version 0.9.3"
    When the tool output is displayed in the TUI
    Then the header shows "● Bash(fspec --sync-version 0.9.3)"
    And the output content uses tree connectors with L character for nesting

  Scenario: Long output is collapsed with expand indicator
    Given the agent executes a tool that produces 753 lines of output
    When the tool completes execution
    Then the first 3-4 lines of output are visible
    And an indicator shows "... +749 lines (ctrl+o to expand)"
    And pressing ctrl+o expands to show full output

  Scenario: System-reminder content displays as nested tree node
    Given the agent executes a tool that returns content with system-reminder tags
    When the tool output is displayed
    Then the system-reminder is shown as a nested tree node
    And it displays with L connector followed by indented content

  Scenario: Streaming output uses fixed-height scrolling window
    Given the agent executes a long-running Bash command like "fspec bootstrap"
    When output streams in real-time
    Then only the last 10 lines of output are visible
    And new lines push older lines up and out of view
    And the full output is not flooding the screen

  Scenario: Tool header remains visible after completion
    Given the agent executes "fspec bootstrap"
    When the command completes
    Then the header "● Bash(fspec bootstrap)" is still visible
    And it appears above the collapsed output
    And it never disappears

  Scenario: All tool types use the same display format
    Given the agent executes tools of different types
    When Read, Edit, Write, Grep, or Glob tools complete
    Then each shows header in format "● ToolName(arguments)"
    And each uses tree connectors for nested output
    And each collapses long output with expand indicator
