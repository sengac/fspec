@multi-agent-support
@agent-compatibility
@generator
@multi-agent
@init
@critical
@INIT-013
Feature: Replace hardcoded CLAUDE.md references with agent-agnostic placeholders

  """
  Template system uses placeholders ({{DOC_TEMPLATE}}, {{AGENT_NAME}}, {{AGENT_ID}}) that are replaced during generation by templateGenerator.ts replacePlaceholders() function. Three template files currently contain hardcoded 'CLAUDE.md' strings that break agent-agnostic design.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All template files must use {{DOC_TEMPLATE}} placeholder instead of hardcoded 'CLAUDE.md'
  #   2. Template placeholders are replaced during generation by templateGenerator.ts replacePlaceholders() function
  #   3. References to FOUNDATION.md and TAGS.md should remain hardcoded as they are universal across all agents
  #
  # EXAMPLES:
  #   1. File: src/utils/slashCommandSections/bootstrapFoundation.ts:62 contains 'spec/CLAUDE.md' → should be 'spec/{{DOC_TEMPLATE}}'
  #   2. File: src/utils/projectManagementSections/fileStructure.ts:10 contains '├── CLAUDE.md' → should be '├── {{DOC_TEMPLATE}}'
  #   3. File: src/utils/projectManagementSections/enforcement.ts:9 contains 'CLAUDE.md' → should be '{{DOC_TEMPLATE}}'
  #   4. When Cursor user runs 'fspec init --agent=cursor', generated spec/CURSOR.md should reference 'spec/CURSOR.md' not 'spec/CLAUDE.md'
  #   5. When Aider user runs 'fspec init --agent=aider', generated spec/AIDER.md should reference 'spec/AIDER.md' not 'spec/CLAUDE.md'
  #
  # ========================================

  Background: User Story
    As a developer using any of the 18 supported AI agents
    I want to see correct references to my agent's documentation file in generated templates
    So that I'm not confused by incorrect references to CLAUDE.md when using Cursor, Aider, or other agents

  Scenario: Replace hardcoded CLAUDE.md in bootstrapFoundation.ts with placeholder
    Given the file "src/utils/slashCommandSections/bootstrapFoundation.ts" contains hardcoded "spec/CLAUDE.md" at line 62
    When I replace the hardcoded string with the placeholder "spec/{{DOC_TEMPLATE}}"
    Then the placeholder should be replaced with the correct agent doc name during template generation
    And Cursor users should see "spec/CURSOR.md" not "spec/CLAUDE.md"
    And Aider users should see "spec/AIDER.md" not "spec/CLAUDE.md"

  Scenario: Replace hardcoded CLAUDE.md in fileStructure.ts with placeholder
    Given the file "src/utils/projectManagementSections/fileStructure.ts" contains "├── CLAUDE.md" at line 10
    When I replace the hardcoded string with "├── {{DOC_TEMPLATE}}"
    Then the file structure example should show the correct agent doc name for each agent
    And the placeholder should be replaced during template generation by templateGenerator.ts

  Scenario: Replace hardcoded CLAUDE.md in enforcement.ts with placeholder
    Given the file "src/utils/projectManagementSections/enforcement.ts" contains "CLAUDE.md" at line 9
    And the line mentions CLAUDE.md as an exception in the documentation rules
    When I replace the hardcoded reference with "{{DOC_TEMPLATE}}" placeholder
    Then all agents should see their correct doc name in the exception list
    And FOUNDATION.md and TAGS.md should remain hardcoded as they are universal

  Scenario: Verify Cursor agent sees correct references after fix
    Given all three template files have been updated to use {{DOC_TEMPLATE}} placeholder
    When a Cursor user runs "fspec init --agent=cursor"
    Then the generated "spec/CURSOR.md" file should contain "spec/CURSOR.md" references
    And the generated file should NOT contain any "spec/CLAUDE.md" references
    And the file structure example should show "├── CURSOR.md"

  Scenario: Verify Aider agent sees correct references after fix
    Given all three template files have been updated to use {{DOC_TEMPLATE}} placeholder
    When an Aider user runs "fspec init --agent=aider"
    Then the generated "spec/AIDER.md" file should contain "spec/AIDER.md" references
    And the generated file should NOT contain any "spec/CLAUDE.md" references
    And the file structure example should show "├── AIDER.md"
