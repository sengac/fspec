@done
@multi-agent
@work-management
@cli
@medium
@REV-002
Feature: Add agent-agnostic ultrathink to review command and update documentation
  """
  Review command uses getAgentConfig() from src/utils/agentRuntimeConfig.ts to detect current agent. Agent detection priority: FSPEC_AGENT env var > spec/fspec-config.json > safe default. Output formatting uses formatAgentOutput() to wrap messages appropriately for each agent type.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Review command must use shared agent detection code (getAgentConfig) from src/utils/agentRuntimeConfig.ts
  #   2. Agent-specific formatting must use formatAgentOutput() for system-reminders (Claude) or bold text (other agents)
  #   3. Documentation must be updated in: src/help.ts, src/commands/review-help.ts (if exists), autogenerator templates, README.md, and docs/ directory
  #   4. Obsolete .claude/commands/review.md file must be removed (no longer used)
  #   5. Yes - Review command help should detect agent using getAgentConfig() and show agent-specific examples (e.g., <system-reminder> for Claude, **⚠️ IMPORTANT:** for Cursor). If no agent configured, show generic examples.
  #
  # EXAMPLES:
  #   1. Claude Code agent runs 'fspec review AUTH-001' and gets output with <system-reminder> tags for ULTRATHINK guidance
  #   2. Cursor agent runs 'fspec review API-005' and gets output with **⚠️ IMPORTANT:** bold text formatting
  #   3. Aider CLI agent runs 'fspec review DASH-003' and gets output with **IMPORTANT:** plain bold text
  #   4. Review command detects agent from FSPEC_AGENT env var or spec/fspec-config.json or defaults to safe plain text
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the review command help include examples for different agent types (Claude, Cursor, Aider)?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent (any type: Claude, Cursor, Aider, etc.)
    I want to use the review command with agent-specific metacognition formatting
    So that I get properly formatted output that works with my capabilities

  Scenario: Review command outputs agent-specific formatting for Claude Code
    Given I am using Claude Code as my AI agent
    And spec/fspec-config.json contains agent: 'claude'
    When I run 'fspec review AUTH-001'
    Then the output should contain <system-reminder> tags
    And the system-reminder should contain ACDD compliance guidance

  Scenario: Review command outputs agent-specific formatting for Cursor IDE
    Given I am using Cursor IDE as my AI agent
    And spec/fspec-config.json contains agent: 'cursor'
    When I run 'fspec review API-005'
    Then the output should contain **⚠️ IMPORTANT:** prefix
    And the message should contain ACDD compliance guidance

  Scenario: Review command outputs agent-specific formatting for Aider CLI
    Given I am using Aider CLI as my AI agent
    And spec/fspec-config.json contains agent: 'aider'
    When I run 'fspec review DASH-003'
    Then the output should contain **IMPORTANT:** prefix
    And the message should contain ACDD compliance guidance

  Scenario: Review command uses agent detection with fallback to safe default
    Given no agent is configured in spec/fspec-config.json
    And FSPEC_AGENT environment variable is not set
    When I run 'fspec review TEST-001'
    Then the output should use safe default formatting
    And the output should NOT contain <system-reminder> tags
    And the output should contain plain text ACDD compliance guidance
