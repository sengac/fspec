@foundation-management
@cli
@phase1
@FOUND-010
Feature: Project type detection incorrectly identifies CLI tools as web-apps

  """
  discover-foundation reads draft, prompts AI for each placeholder, AI uses fspec update-foundation commands (NOT manual editing)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. foundation.json.draft IS the guidance file - it defines structure and what needs to be filled
  #   2. Draft contains [QUESTION: text] placeholders for fields requiring human/AI input
  #   3. Draft contains [DETECTED: value] for auto-detected fields that AI should verify with human
  #   4. discover-foundation reads draft and identifies unfilled/placeholder fields
  #   5. For each unfilled field, command emits system-reminder prompting AI to gather information
  #   6. AI must analyze codebase AND ask human to gather accurate information
  #   7. AI must use fspec update-foundation command to set values (NOT manual editing)
  #   8. After each field is set, command re-reads draft and chains to next unfilled field
  #   9. When all [QUESTION:] placeholders resolved, command validates against JSON schema
  #   10. Final step: command creates foundation.json and deletes draft
  #
  # EXAMPLES:
  #   1. Draft has projectType:[QUESTION: What type?] → Command prompts AI → AI analyzes commander.js usage → AI asks human 'Is this cli-tool, web-app, or library?' → Human: 'cli-tool' → AI runs: fspec update-foundation --field project.projectType --value cli-tool → Command re-reads draft and moves to next field
  #   2. Draft has personas with [QUESTION: Describe persona] → Command prompts AI to gather persona info → AI analyzes CLI commands, no web UI → AI asks human 'Who uses this and their goals?' → Human: 'Developers in terminal managing specs' → AI runs: fspec update-foundation --field personas[0].description --value 'Developer using fspec to manage Gherkin specs' → Command chains to next field
  #   3. Draft has all [QUESTION:] resolved → Command validates against schema → If valid: creates foundation.json, deletes draft → If invalid: shows validation errors, prompts AI to fix
  #   4. AI tries to manually edit foundation.json.draft → Command detects file change outside fspec → Emits system-reminder: 'You must use fspec update-foundation commands, not manual editing'
  #   5. Draft has [DETECTED: web-app] for projectType → Command prompts AI to verify → AI asks human 'Detected web-app, is this correct?' → Human: 'No, it's cli-tool' → AI runs: fspec update-foundation --field project.projectType --value cli-tool
  #
  # ========================================

  Background: User Story
    As a AI agent running discover-foundation
    I want to correctly detect CLI tool projects
    So that foundation.json has accurate project type and personas

  Scenario: Fill project type field using draft guidance
    Given foundation.json.draft contains "projectType": "[QUESTION: What type of project is this?]"
    When discover-foundation command identifies this unfilled field
    Then command should emit system-reminder prompting AI to gather information
    And AI should analyze codebase for project type indicators
    And AI should ask human "Is this a cli-tool, web-app, or library?"
    And human responds "cli-tool"
    And AI should run "fspec update-foundation --field project.projectType --value cli-tool"
    And command should re-read draft and identify next unfilled field

  Scenario: Fill persona field using draft guidance
    Given foundation.json.draft contains persona with "[QUESTION: Describe this persona]"
    When discover-foundation command identifies this unfilled field
    Then command should emit system-reminder to gather persona information
    And AI should analyze codebase to identify user types
    And AI should ask human "Who uses this tool and what are their goals?"
    And human responds "Developers using it in terminal to manage specs"
    And AI should run command to update persona description
    And command should chain to next unfilled field

  Scenario: Finalize foundation when all fields are filled
    Given foundation.json.draft has all [QUESTION:] placeholders resolved
    When discover-foundation command validates the draft
    Then command should validate draft against JSON schema
    And if validation passes, command should create foundation.json
    And command should delete foundation.json.draft
    And if validation fails, command should show validation errors
    And prompt AI to fix validation issues

  Scenario: Prevent manual editing of draft file
    Given AI attempts to manually edit foundation.json.draft with Write/Edit tools
    When discover-foundation command detects file change outside fspec commands
    Then command should emit system-reminder "You must use fspec update-foundation commands, not manual editing"
    And command should guide AI to use proper fspec commands

  Scenario: Verify detected values with human
    Given foundation.json.draft contains "projectType": "[DETECTED: web-app]"
    When discover-foundation command processes this field
    Then command should prompt AI to verify detected value
    And AI should ask human "Detected web-app, is this correct?"
    And human responds "No, it's cli-tool"
    And AI should run "fspec update-foundation --field project.projectType --value cli-tool"
    And command should update draft with correct value
