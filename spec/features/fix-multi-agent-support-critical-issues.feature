@phase1
@cli
@multi-agent-support
@bug-fix
@security
@initialization
@INIT-005
Feature: Fix multi-agent support critical issues
  """
  Architecture notes:
  - Fixes 25 bugs/anti-patterns discovered during INIT-004 code review
  - Critical security fixes: path traversal prevention, non-destructive file operations
  - Robustness improvements: regex edge cases, word boundary handling, duplicate detection
  - User experience: helpful error messages, clear validation feedback

  Critical implementation requirements:
  - Path validation must use Node.js path.normalize() and check for ../ patterns
  - File deletion must be surgical (specific files only, never entire directories)
  - Agent registry must validate for duplicate paths at load time
  - Regex transformations must be tested against edge cases (nested tags, multiline)
  - Word boundary regex must use \b for meta-cognitive prompt removal
  - Error messages must include actionable suggestions

  Dependencies:
  - Node.js path module for path validation and normalization
  - Existing agent registry (src/utils/agentRegistry.ts)
  - Template generator (src/utils/templateGenerator.ts)
  - Init command (src/commands/init.ts)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Agent switching must ONLY delete fspec-specific files (AGENT.md, spec/AGENT.md, .agent/commands/fspec.md), never entire directories or user's custom files
  #   2. Agent registry slashCommandPath must be validated to prevent path traversal attacks (no ../, no absolute paths, must be relative to project root)
  #   3. Each agent must have unique slashCommandPath and rootStubFile to avoid conflicts (e.g., codex and codex-cli cannot both use .codex/commands/)
  #   4. System-reminder regex transformation must handle nested tags, multiline content, and edge cases without breaking content
  #   5. Meta-cognitive prompt removal must use word boundaries to avoid corrupting valid words containing 'think' (e.g., 'rethink', 'thinking')
  #   6. Invalid agent ID errors must include list of valid agent IDs to help users discover available options
  #
  # EXAMPLES:
  #   1. Critical: User runs 'fspec init --agent=cursor' after 'fspec init --agent=claude', system deletes .claude/commands/fspec.md only (not entire .claude/commands/ directory with user's custom commands)
  #   2. Critical: Agent registry contains malicious slashCommandPath '../../../etc/passwd', validation rejects it before creating files
  #   3. Critical: codex and codex-cli agents both try to use .codex/commands/, installation detects conflict and shows clear error message
  #   4. High: Template contains nested system-reminders, transformation correctly handles both levels without breaking content structure
  #   5. High: Template contains 'rethinking' and 'thinking', meta-cognitive removal only strips 'ultrathink' and 'deeply consider', preserves valid words
  #   6. High: User runs 'fspec init --agent=invalid-agent', error message lists all 18 valid agent IDs with descriptions
  #
  # ========================================
  Background: User Story
    As a developer using fspec with multiple AI agents
    I want to have critical bugs and anti-patterns from INIT-004 implementation fixed
    So that multi-agent support is production-ready, secure, and follows best practices

  Scenario: Non-destructive agent switching
    Given I have installed fspec for Claude with "fspec init --agent=claude"
    And the .claude/commands/ directory contains custom user files
    When I run "fspec init --agent=cursor" to switch agents
    Then the system should delete only .claude/commands/fspec.md
    And the system should preserve all other files in .claude/commands/
    And the system should install Cursor-specific files

  Scenario: Path traversal attack prevention
    Given the agent registry contains an agent with slashCommandPath "../../../etc/passwd"
    When the installation attempts to validate the path
    Then the validation should reject the path
    And the error message should indicate path traversal attempt
    And no files should be created outside the project directory

  Scenario: Duplicate agent path detection
    Given the agent registry has "codex" with slashCommandPath ".codex/commands/"
    And the agent registry has "codex-cli" with slashCommandPath ".codex/commands/"
    When the registry is loaded
    Then the system should detect the duplicate paths
    And the system should show a clear error message
    And the error message should list the conflicting agents

  Scenario: Nested system-reminder transformation
    Given a template contains nested system-reminder tags
    When the template generator transforms the content for a non-Claude agent
    Then both outer and inner system-reminders should be transformed
    And the content structure should remain intact
    And no closing tags should be left unmatched

  Scenario: Safe meta-cognitive prompt removal
    Given a template contains the words "rethinking" and "thinking"
    And the template contains "ultrathink your next steps"
    When the template generator removes meta-cognitive prompts
    Then "ultrathink your next steps" should be removed
    And "rethinking" should be preserved
    And "thinking" should be preserved

  Scenario: Helpful invalid agent error
    Given I run "fspec init --agent=invalid-agent"
    When the system detects the invalid agent ID
    Then the error message should list all 18 valid agent IDs
    And each agent should include a brief description
    And the exit code should be 1
