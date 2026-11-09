@foundation-management
@agent-detection
@foundation
@cli
@multi-agent
@discovery
@BUG-031
@BUG-048
Feature: No support for ULTRATHINK in discovery-driven feedback loop
  """
  Uses existing INIT-008 agent detection infrastructure (getAgentConfig from agentRuntimeConfig.ts). Implements conditional messaging based on agent.supportsMetaCognition flag. Only modifies 2 locations in discover-foundation.ts: initial draft system-reminder (line ~422) and project.vision field guidance (line ~95). Uses formatAgentOutput() for proper output formatting per agent type (system-reminder for Claude, bold+emoji for IDE, plain for CLI).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Claude Code is the only agent that supports 'ULTRATHINK' terminology (supportsMetaCognition flag)
  #   2. Non-Claude agents must receive generic 'analyze' or 'examine' language instead of ULTRATHINK
  #   3. Agent detection must use existing INIT-008 infrastructure (getAgentConfig from agentRuntimeConfig.ts)
  #   4. Output format must match agent type: system-reminder for Claude, bold+emoji for IDE agents, plain bold for CLI agents
  #   5. ULTRATHINK appears in exactly 2 locations: initial draft reminder and project.vision field guidance
  #   6. Focus only on locations with ULTRATHINK (initial draft + project.vision). Other fields don't have Claude-specific language. Keep scope minimal for quick fix (Option 1).
  #
  # EXAMPLES:
  #   1. Claude Code agent runs discover-foundation, sees 'ULTRATHINK: Read ALL code, understand deeply' in system-reminder
  #   2. Cursor agent runs discover-foundation, sees 'Think a lot about the entire codebase' with **⚠️ IMPORTANT:** format
  #   3. Aider CLI agent runs discover-foundation, sees 'Think a lot about the entire codebase' with **IMPORTANT:** plain text format
  #   4. Unknown/default agent runs discover-foundation, sees 'Think a lot about the entire codebase' without system-reminder tags
  #   5. discover-foundation initial draft creation uses agent.supportsMetaCognition to conditionally include ULTRATHINK
  #   6. project.vision field guidance checks agent capabilities before using ULTRATHINK terminology
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we add agent-specific variations for ALL field guidance prompts, or just focus on project.vision which currently has ULTRATHINK?
  #   A: true
  #
  #   Q: Should help documentation (discover-foundation-help.ts) also be updated to remove ULTRATHINK references, or can we keep it as an example of Claude-specific guidance?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Update help docs to mention this is Claude-specific and explain agent-aware messaging. Add note: 'Non-Claude agents will see generic analysis language.' Documentation should reflect multi-agent reality.
  #
  # ========================================
  Background: User Story
    As a AI agent (non-Claude) using fspec discover-foundation
    I want to receive guidance in language my agent understands
    So that I can successfully complete foundation discovery without confusion

  Scenario: Claude Code agent receives ULTRATHINK guidance in system-reminder
    Given I am using Claude Code (FSPEC_AGENT=claude)
    And Claude Code has supportsMetaCognition flag set to true
    When I run fspec discover-foundation
    Then I should see "you must ULTRATHINK the entire codebase" in the output
    And the output should contain system-reminder tags
    And the guidance should emphasize deep codebase analysis

  Scenario: Cursor IDE agent receives 'think a lot' guidance
    Given I am using Cursor (FSPEC_AGENT=cursor)
    And Cursor has supportsMetaCognition flag set to false
    When I run fspec discover-foundation
    Then I should see "you must think a lot about the entire codebase" in the output
    And I should NOT see "ULTRATHINK" in the output

  Scenario: Aider CLI agent receives 'think a lot' guidance
    Given I am using Aider (FSPEC_AGENT=aider)
    And Aider has supportsMetaCognition flag set to false
    When I run fspec discover-foundation
    Then I should see "you must think a lot about the entire codebase" in the output
    And I should NOT see "ULTRATHINK" in the output

  Scenario: Unknown agent receives 'think a lot' guidance
    Given no agent is configured (no FSPEC_AGENT env var or config file)
    When I run fspec discover-foundation
    Then I should see "you must think a lot about the entire codebase" in the output
    And I should NOT see "ULTRATHINK" in the output

  Scenario: Project vision field guidance follows the agent's meta-cognition support
    Given a foundation draft whose next unfilled field after project.name is project.vision
    When I run fspec update-foundation projectName once with FSPEC_AGENT=claude and once with FSPEC_AGENT=cursor
    Then the claude reminder for Field 2/8 should include "ULTRATHINK: Read ALL code, understand the system deeply."
    And the cursor reminder for Field 2/8 should include "Think a lot about the entire codebase."
