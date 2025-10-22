@done
@foundation
@cli
@BUG-015
Feature: Foundation discovery workflow cannot update nested fields (capabilities, personas)
  """
  Root Cause: src/commands/discover-foundation.ts emits system-reminders with '--field <path> --value <value>' syntax, but src/commands/update-foundation.ts implements '<section> <content>' syntax with no --field option
  Impact: Breaks field-by-field discovery feedback loop documented in CLAUDE.md 'Foundation Document Discovery' section. AI cannot complete foundation without manual file editing.
  Current Workaround: AI must manually edit foundation.json or foundation.json.draft using Write tool, which violates the automation principle and bypasses validation
  Affected Fields: capabilities[] (array of {name, description} objects), personas[] (array of {name, description, goals[]} objects), architectureDiagrams[] (array of Mermaid diagrams)
  Schema Reference: src/schemas/generic-foundation.schema.json defines required structure - capabilities[] requires minItems:1, personas[] is optional but must follow persona definition structure
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. discover-foundation system-reminders MUST use syntax that matches actual update-foundation command implementation
  #   2. update-foundation command MUST support updating nested array fields (capabilities[], personas[]) through CLI without manual file editing
  #   3. Foundation discovery workflow MUST be fully automatable by AI without requiring manual JSON editing
  #   4. All fields in foundation.json.draft MUST have corresponding CLI commands to update them programmatically
  #
  # EXAMPLES:
  #   1. AI runs 'fspec discover-foundation' → system-reminder says 'Run: fspec update-foundation --field project.name --value <name>' → AI tries command → Error: 'unknown option --field'
  #   2. AI checks help with 'fspec update-foundation --help' → discovers actual syntax is '<section> <content>' not '--field <path> --value <value>'
  #   3. AI successfully uses 'fspec update-foundation projectName "fspec"' for simple fields → works correctly
  #   4. AI encounters foundation.json.draft with capabilities[] array placeholder → no CLI command exists to update array fields → workflow breaks
  #   5. AI encounters personas[] array with nested structure → update-foundation command cannot handle nested JSON objects → must manually edit file (violates automation principle)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should update-foundation support JSON path syntax (e.g., 'capabilities[0].name') or add separate commands (e.g., 'add-capability', 'add-persona')?
  #   A: true
  #
  #   Q: Should the fix update discover-foundation to emit correct syntax, or extend update-foundation to support --field flag?
  #   A: true
  #
  #   Q: How should AI update array elements - append to array, or replace entire array?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Add separate commands for better UX: 'fspec add-capability <name> <description>' and 'fspec add-persona <name> <description> --goal <goal>'. This follows existing command patterns (add-scenario, add-step) and is more intuitive than JSON path syntax.
  #   2. Create new commands (add-capability, add-persona) that update foundation.json programmatically. Update discover-foundation to emit system-reminders with correct command syntax. This is cleaner than retrofitting --field flag to update-foundation.
  #   3. Append to arrays for add-capability and add-persona commands. This allows building up the foundation iteratively during discovery. Provide separate clear-capabilities and clear-personas commands if replacement is needed.
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec discover-foundation workflow
    I want to update nested array fields in foundation.json through CLI commands
    So that I can complete foundation discovery without manual file editing

  Scenario: Add capability to foundation.json through CLI
    Given I have a foundation.json file with an empty capabilities array
    When I run "fspec add-capability 'Kanban Workflow' 'Enforces ACDD phases with visual board'"
    Then the foundation.json file should contain the new capability
    And the capability should have name "Kanban Workflow"
    And the capability should have description "Enforces ACDD phases with visual board"
    And the foundation.json should pass schema validation

  Scenario: Add persona to foundation.json through CLI
    Given I have a foundation.json file
    When I run "fspec add-persona 'AI Agent' 'Uses fspec to manage specifications' --goal 'Complete foundation discovery without manual editing'"
    Then the foundation.json file should contain the new persona
    And the persona should have name "AI Agent"
    And the persona should have description "Uses fspec to manage specifications"
    And the persona should have goal "Complete foundation discovery without manual editing"
    And the foundation.json should pass schema validation

  Scenario: Add multiple capabilities iteratively
    Given I have a foundation.json file with one existing capability
    When I run "fspec add-capability 'Example Mapping' 'Collaborative discovery with rules and examples'"
    And I run "fspec add-capability 'Coverage Tracking' 'Link scenarios to tests and implementation'"
    Then the foundation.json file should contain 3 capabilities total
    And all capabilities should be preserved
    And the foundation.json should pass schema validation

  Scenario: Discover foundation workflow uses correct command syntax
    Given I have a project without foundation.json
    When I run "fspec discover-foundation"
    Then a foundation.json.draft file should be created
    And any system-reminders should reference valid CLI commands
    And system-reminders should NOT reference "--field" flag
    And system-reminders should suggest "fspec add-capability" for capabilities
    And system-reminders should suggest "fspec add-persona" for personas

  Scenario: Complete foundation discovery without manual file editing
    Given I have a foundation.json.draft with placeholders
    When I use only CLI commands to fill all fields
    And I run "fspec add-capability 'User Authentication' 'Secure access control'"
    And I run "fspec add-persona 'Developer' 'Builds features with AI agents' --goal 'Ship quality code faster'"
    And I run "fspec discover-foundation --finalize"
    Then the foundation.json should be created successfully
    And the foundation.json should contain all required fields
    And the foundation.json should pass schema validation
    And no manual file editing should have been required
