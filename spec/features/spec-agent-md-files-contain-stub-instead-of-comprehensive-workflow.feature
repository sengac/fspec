@done
@multi-agent-support
@templates
@critical
@BUG-028
Feature: spec/AGENT.md files contain stub instead of comprehensive workflow
  """
  Architecture notes:
  - Create src/utils/projectManagementTemplate.ts following same pattern as slashCommandTemplate.ts
  - Break spec/CLAUDE.md (2069 lines) into section files in src/utils/projectManagementSections/ directory
  - Update generateAgentDoc() in src/utils/templateGenerator.ts to use getProjectManagementTemplate()
  - generateSlashCommandContent() is ALREADY CORRECT - uses getSlashCommandTemplate() for .claude/commands/fspec.md
  - After fix: spec/CLAUDE.md ~2069 lines (Project Management), .claude/commands/fspec.md remains ~1019 lines (ACDD workflow)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. spec/AGENT.md files must contain full Project Management Guidelines (2069 lines), not a 17-line stub
  #   2. generateAgentDoc() must use getProjectManagementTemplate() as base content, not BASE_AGENT_TEMPLATE
  #   3. projectManagementTemplate.ts should follow same pattern as slashCommandTemplate.ts with section files
  #   4. Agent-specific transformations (stripSystemReminders, removeMetaCognitivePrompts, replacePlaceholders) must be applied
  #   5. Slash command generation (generateSlashCommandContent) should NOT be changed - it's already correct
  #
  # EXAMPLES:
  #   1. Current spec/CLAUDE.md has 17 lines with circular reference: 'For using fspec commands and ACDD workflow: See spec/CLAUDE.md'
  #   2. Expected spec/CLAUDE.md should have ~2069 lines titled "Project Management and Specification Guidelines for fspec"
  #   3. Restored spec/CLAUDE.md (from GitHub) has 2069 lines - this content should be templated
  #   4. Slash command .claude/commands/fspec.md has 1019 lines ACDD workflow - DIFFERENT from spec/CLAUDE.md, should NOT change
  #   5. projectManagementTemplate.ts follows pattern: getIntroSection(), getProjectManagementSection(), etc.
  #
  # ========================================
  Background: User Story
    As an AI agent (Cursor, Aider, etc.) reading spec/AGENT.md
    I want to read comprehensive Project Management and Specification Guidelines
    So that I understand the full fspec workflow including work units, ACDD, coverage, hooks, and checkpoints

  Scenario: spec/AGENT.md contains comprehensive Project Management Guidelines
    Given generateAgentDoc() is modified to use getProjectManagementTemplate() as base
    When fspec init --agent=claude is run
    Then spec/CLAUDE.md should contain approximately 2069 lines
    And spec/CLAUDE.md should be titled "Project Management and Specification Guidelines for fspec"
    And spec/CLAUDE.md should include work unit management documentation
    And spec/CLAUDE.md should include Reverse ACDD documentation
    And spec/CLAUDE.md should include coverage tracking system documentation
    And spec/CLAUDE.md should include lifecycle hooks documentation
    And spec/CLAUDE.md should include git checkpoints documentation

  Scenario: Agent-specific transformations are applied to Project Management template
    Given generateAgentDoc() uses getProjectManagementTemplate() as base content
    When generating spec/CURSOR.md for Cursor agent
    Then system-reminder tags should be transformed to "**⚠️ IMPORTANT:**"
    And agent name placeholders should be replaced with "Cursor"
    And slash command path placeholders should be replaced with ".cursor/commands/"
    And the file should contain the full Project Management Guidelines with transformations applied

  Scenario: Root stub files remain concise
    Given installRootStub() generates inline stub content (not BASE_AGENT_TEMPLATE)
    When fspec init --agent=cursor is run
    Then CURSOR.md (root stub) should contain approximately 17 lines
    And CURSOR.md should point to spec/CURSOR.md for full documentation
    And spec/CURSOR.md should contain approximately 2069 lines with Project Management Guidelines

  Scenario: Fix eliminates circular reference in spec/CLAUDE.md
    Given the current spec/CLAUDE.md has 17 lines with circular reference
    When generateAgentDoc() is fixed to use getProjectManagementTemplate()
    And fspec init --agent=claude is run
    Then spec/CLAUDE.md should not contain the text "See spec/CLAUDE.md"
    And spec/CLAUDE.md should contain comprehensive standalone Project Management Guidelines
    And spec/CLAUDE.md should have approximately 2069 lines
