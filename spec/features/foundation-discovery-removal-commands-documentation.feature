@done
@high
@documentation
@help-system
@cli
@REMIND-010
Feature: Foundation discovery removal commands documentation
  """
  Help text should show examples of removing BOTH actual personas/capabilities AND placeholder text like '[QUESTION: Who uses this?]'
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The 'fspec help setup' section must include remove-persona command documentation
  #   2. discover-foundation placeholder warnings must explain how to remove unwanted items
  #   3. Both remove-persona and remove-capability must be documented in help setup section
  #   4. discover-foundation error message must include instructions for both filling AND removing placeholder fields
  #
  # EXAMPLES:
  #   1. Developer runs 'fspec help setup' and sees remove-persona command listed alongside add-persona
  #   2. discover-foundation detects placeholder personas and outputs system-reminder explaining to use 'fspec remove-persona <name>' to remove them
  #   3. Developer runs 'fspec help setup' and sees remove-capability command with usage, options, and examples
  #   4. Help text for remove-persona and remove-capability follows same format as add-persona/add-capability with usage, description, and examples
  #   5. When discover-foundation detects placeholder personas like '[QUESTION: Who uses this?]', the error message should explain: 'To remove unwanted placeholders: fspec remove-persona "[QUESTION: Who uses this?]"'
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should remove-capability also be included in the help setup section, or just remove-persona?
  #   A: true
  #
  #   Q: Where exactly in the discover-foundation flow should the removal guidance appear - when placeholders are detected, when showing field-by-field prompts, or both?
  #   A: true
  #
  #   Q: Should the help text follow the same format as other commands in the setup section (showing usage, options, examples)?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to understand how to remove placeholder items during foundation discovery
    So that I can easily clean up unwanted personas/capabilities without manual JSON editing

  Scenario: Help setup section includes remove-persona command
    Given I am using fspec
    When I run "fspec help setup"
    Then the output should include "fspec remove-persona <name>"
    And the output should include "Description: Remove persona from foundation.json"
    And the output should include example usage for removing personas
    And the remove-persona section should appear alongside add-persona

  Scenario: Help setup section includes remove-capability command
    Given I am using fspec
    When I run "fspec help setup"
    Then the output should include "fspec remove-capability <name>"
    And the output should include "Description: Remove capability from foundation.json"
    And the output should include example usage for removing capabilities
    And the remove-capability section should appear alongside add-capability

  Scenario: Removal commands show placeholder removal examples
    Given I am using fspec
    When I run "fspec help setup"
    Then the remove-persona examples should include removing placeholder text
    And the output should show 'fspec remove-persona "[QUESTION: Who uses this?]"'
    And the remove-capability examples should include removing placeholder text
    And the output should show 'fspec remove-capability "[QUESTION: What can users DO?]"'

  Scenario: discover-foundation error includes removal guidance for placeholders
    Given I have a foundation draft with placeholder personas
    And the draft contains '[QUESTION: Who uses this?]' placeholder
    When I run "fspec discover-foundation --finalize"
    Then the command should fail with exit code 1
    And the error message should explain how to fill placeholders
    And the error message should explain how to remove unwanted placeholders
    And the error message should include "fspec remove-persona" command example
    And the error message should include "fspec remove-capability" command example
