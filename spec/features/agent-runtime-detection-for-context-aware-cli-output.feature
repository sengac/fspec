@done
@runtime
@agent-detection
@cli
@high
@INIT-008
Feature: Agent runtime detection for context-aware CLI output
  """
  Output formats: Claude Code gets <system-reminder> tags, IDE agents (Cursor/Cline) get **⚠️ IMPORTANT:** with emoji, CLI-only agents (Aider/Gemini) get **IMPORTANT:** plain text.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Detection must use priority chain: FSPEC_AGENT env var > config file > auto-detect from project files > safe default
  #   2. System-reminders only emitted for Claude Code at runtime
  #   3. Cursor/Cline/IDE agents receive **⚠️ IMPORTANT:** with bold text and emoji
  #   4. CLI-only agents (Aider, Gemini) receive **IMPORTANT:** plain text without emoji
  #   5. Must support all 18 agents defined in existing agent registry
  #   6. Must fallback to safe default (plain text, no system-reminders) if detection fails
  #   7. Per-project config file at spec/fspec-config.json (consistent with other fspec config files like spec/fspec-hooks.json)
  #   8. No runtime auto-detection needed. Agent is detected during 'fspec init' (using existing detection logic) and written to spec/fspec-config.json. All subsequent commands read from config file.
  #   9. Agent registry already exists with all needed metadata. 'fspec init' detects agent and writes to config. Runtime commands read config to determine output format (system-reminder for Claude, bold+emoji for IDE agents, plain for CLI agents).
  #
  # EXAMPLES:
  #   1. User sets FSPEC_AGENT=claude, runs 'fspec validate', output contains <system-reminder> tags
  #   2. Cursor user (no env var) runs 'fspec validate' in project with .cursor/ directory, output contains **⚠️ IMPORTANT:** with emoji
  #   3. Aider user runs 'fspec validate' with no env var or config, auto-detects CLI-only agent, output contains **IMPORTANT:** plain text
  #   4. User has ~/.fspec/config.json with defaultAgent=cursor but FSPEC_AGENT=claude is set, Claude is used (env var takes priority)
  #   5. Unknown environment with no detection clues, system falls back to safe default (plain text output, no system-reminders)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should config file be global (~/.fspec/config.json) or per-project (.fspec/config.json) or both with priority?
  #   A: true
  #
  #   Q: What are the exact directory/file patterns for auto-detection (e.g., .claude/, .cursor/, .aider/config)?
  #   A: true
  #
  #   Q: Should the agent registry be extended with output format preferences (system-reminder vs bold vs plain)?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent (Claude Code, Cursor, Aider, etc.) running fspec commands
    I want to have CLI output adapted to my capabilities (system-reminders, bold text, plain text)
    So that I can understand and act on fspec guidance effectively without confusion or unsupported syntax

  Scenario: FSPEC_AGENT environment variable overrides config file
    Given a project with spec/fspec-config.json containing {"agent": "cursor"}
    And FSPEC_AGENT environment variable is set to "claude"
    When I run "fspec validate"
    Then the output should use Claude Code format with <system-reminder> tags
    And the config file agent setting should be ignored

  Scenario: Agent detected during init and written to config file
    Given I run "fspec init" in a new project
    And Claude Code is the detected agent
    When fspec init completes
    Then spec/fspec-config.json should be created
    And it should contain {"agent": "claude"}
    And subsequent commands should read this config for output formatting

  Scenario: Claude Code agent receives system-reminder output
    Given spec/fspec-config.json contains {"agent": "claude"}
    And FSPEC_AGENT is not set
    When I run "fspec update-work-unit-status WORK-001 testing"
    Then the output should contain <system-reminder> tags
    And guidance text should be wrapped in system-reminder format

  Scenario: Cursor IDE agent receives bold text with emoji output
    Given spec/fspec-config.json contains {"agent": "cursor"}
    And FSPEC_AGENT is not set
    When I run "fspec update-work-unit-status WORK-001 testing"
    Then the output should contain "**⚠️ IMPORTANT:**"
    And no <system-reminder> tags should appear

  Scenario: Aider CLI agent receives plain text output
    Given spec/fspec-config.json contains {"agent": "aider"}
    And FSPEC_AGENT is not set
    When I run "fspec update-work-unit-status WORK-001 testing"
    Then the output should contain "**IMPORTANT:**"
    And no emoji should appear in the output
    And no <system-reminder> tags should appear

  Scenario: Fallback to safe default when no config exists
    Given spec/fspec-config.json does not exist
    And FSPEC_AGENT is not set
    When I run "fspec validate"
    Then the output should use plain text format
    And no <system-reminder> tags should appear
    And no emoji should appear in the output
