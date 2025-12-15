Feature: CLI-016: Context Gathering with CLAUDE.md Discovery
  As a CLI user
  I want the CLI to automatically discover and inject project context (CLAUDE.md)
  So that the LLM has relevant project information without manual setup

  Background:
    Given the codelet CLI application is running
    And the system_reminders module provides injection infrastructure

  # Discovery of CLAUDE.md/AGENTS.md files

  Scenario: Discover CLAUDE.md in current directory
    Given a CLAUDE.md file exists in the current working directory
    When the CLI starts an interactive session
    Then the CLAUDE.md content should be injected as a system reminder
    And the system reminder type should be "claudeMd"

  Scenario: Discover CLAUDE.md in parent directory
    Given no CLAUDE.md file exists in the current working directory
    And a CLAUDE.md file exists in a parent directory
    When the CLI starts an interactive session
    Then the CLAUDE.md content from the parent directory should be injected
    And the system reminder type should be "claudeMd"

  Scenario: Discover AGENTS.md as fallback
    Given no CLAUDE.md file exists in any parent directory
    And an AGENTS.md file exists in the current working directory
    When the CLI starts an interactive session
    Then the AGENTS.md content should be injected as a system reminder
    And the system reminder type should be "claudeMd"

  Scenario: No context file found
    Given no CLAUDE.md or AGENTS.md file exists in any parent directory
    When the CLI starts an interactive session
    Then no claudeMd system reminder should be injected
    And the session should start normally without error

  # Environment information gathering

  Scenario: Inject environment information
    When the CLI starts an interactive session
    Then an environment system reminder should be injected
    And it should contain the platform (OS)
    And it should contain the architecture
    And it should contain the shell (if available)
    And it should contain the username (if available)
    And it should contain the current working directory

  # Deduplication and prompt cache preservation

  Scenario: System reminders are deduplicated
    Given the CLI has injected initial context reminders
    When context is re-injected during the session
    Then the new reminders should have supersession markers
    And the old reminders should be preserved for prompt cache stability

  Scenario: Context preserved across multi-turn conversation
    Given an interactive session with CLAUDE.md context injected
    When I have a multi-turn conversation
    Then the CLAUDE.md context should remain available to the LLM
    And token tracking should account for context size

  # Integration with compaction

  Scenario: System reminders persist through compaction
    Given an interactive session with context reminders injected
    And the conversation triggers context compaction
    When compaction completes
    Then the latest system reminders should be preserved
    And older superseded reminders should be removed

  # File reading behavior

  Scenario: CLAUDE.md content is read completely
    Given a CLAUDE.md file with multiple sections
    When the CLI discovers and reads it
    Then the entire file content should be included
    And no truncation should occur

  Scenario: Handle unreadable CLAUDE.md gracefully
    Given a CLAUDE.md file exists but is not readable (permissions)
    When the CLI attempts to discover context
    Then a warning should be logged
    And the session should start normally without that context

  # Directory traversal limits

  Scenario: Stop at filesystem root
    Given no CLAUDE.md exists in any directory up to the filesystem root
    When the CLI searches for context files
    Then the search should stop at the filesystem root
    And no claudeMd system reminder should be injected
