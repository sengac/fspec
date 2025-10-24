@foundation-management
@agent-detection
@foundation
@cli
@multi-agent
@discovery
@BUG-031
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
  #   2. Cursor agent runs discover-foundation, sees 'Carefully analyze the entire codebase' with **⚠️ IMPORTANT:** format
  #   3. Aider CLI agent runs discover-foundation, sees 'Thoroughly examine the codebase' with **IMPORTANT:** plain text format
  #   4. Unknown/default agent runs discover-foundation, sees safe generic language without system-reminder tags
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
    Then I should see "ULTRATHINK: Read ALL code, understand deeply" in the output
    And the output should contain system-reminder tags
    And the guidance should emphasize deep codebase analysis

  Scenario: Cursor IDE agent receives generic analysis guidance with emoji warning
    Given I am using Cursor (FSPEC_AGENT=cursor)
    And Cursor has supportsMetaCognition flag set to false
    When I run fspec discover-foundation
    Then I should see "Carefully analyze the entire codebase" in the output
    And I should NOT see "ULTRATHINK" in the output
    And the output should contain "**⚠️ IMPORTANT:**" format
    And the output should NOT contain system-reminder tags

  Scenario: Aider CLI agent receives plain text analysis guidance
    Given I am using Aider (FSPEC_AGENT=aider)
    And Aider has supportsMetaCognition flag set to false
    When I run fspec discover-foundation
    Then I should see "Thoroughly examine the codebase" in the output
    And I should NOT see "ULTRATHINK" in the output
    And the output should contain "**IMPORTANT:**" format without emoji
    And the output should NOT contain system-reminder tags

  Scenario: Unknown agent receives safe default guidance without system-reminders
    Given no agent is configured (no FSPEC_AGENT env var or config file)
    When I run fspec discover-foundation
    Then I should see generic analysis language in the output
    And I should NOT see "ULTRATHINK" in the output
    And I should NOT see system-reminder tags
    And the guidance should use safe default plain text format

  Scenario: Initial draft creation uses agent capability detection for ULTRATHINK
    Given I am implementing the discover-foundation command
    When the initial draft system-reminder is generated (line ~422)
    Then the code should call getAgentConfig(cwd) to detect the current agent
    And the code should check agent.supportsMetaCognition flag
    And if true, the reminder should include "you must ULTRATHINK the entire codebase"
    And if false, the reminder should include "you must thoroughly analyze the entire codebase"

  Scenario: Project vision field guidance checks agent capabilities for ULTRATHINK
    Given I am implementing the project.vision field guidance
    When the field-specific system-reminder is generated (line ~95)
    Then the code should call getAgentConfig(cwd) to detect the current agent
    And the code should check agent.supportsMetaCognition flag
    And if true, the guidance should include "ULTRATHINK: Read ALL code, understand deeply"
    And if false, the guidance should include "Carefully analyze the codebase to understand its purpose"
