@done
@foundation-management
@bug
@cli
@discovery
@BUG-057
Feature: Fix incorrect command name in discover-foundation system reminder for problemSpace.primaryProblem.title
  """
  Bug fix in src/commands/discover-foundation.ts line 132. The system reminder for problemSpace.primaryProblem.title field incorrectly instructs 'fspec update-foundation problemDefinition' but should instruct 'fspec update-foundation problemTitle'. The command handlers in src/commands/update-foundation.ts show: problemTitle updates title field (line 151-156), problemDefinition updates description field (line 158-164).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System reminder for problemSpace.primaryProblem.title must instruct to use 'problemTitle' command, not 'problemDefinition'
  #   2. Command name 'problemTitle' updates problemSpace.primaryProblem.title field
  #   3. Command name 'problemDefinition' updates problemSpace.primaryProblem.description field
  #
  # EXAMPLES:
  #   1. AI agent receives system reminder for 'problemSpace.primaryProblem.title' field with instruction: 'Run: fspec update-foundation problemTitle "Problem Title"'
  #   2. AI agent runs 'fspec update-foundation problemTitle "User Authentication Issues"' and foundation.json.draft shows problemSpace.primaryProblem.title updated correctly
  #   3. Before fix: System reminder says 'fspec update-foundation problemDefinition' for title field (incorrect)
  #   4. After fix: System reminder says 'fspec update-foundation problemTitle' for title field (correct)
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec discover-foundation
    I want to receive correct command name in system reminder for problemSpace.primaryProblem.title field
    So that I can successfully update the correct field without confusion

  Scenario: System reminder instructs to use problemTitle command for title field
    Given I am an AI agent running fspec discover-foundation
    And the foundation.json.draft has placeholder for problemSpace.primaryProblem.title
    When the discover-foundation command emits system reminder for problemSpace.primaryProblem.title field
    Then the system reminder should instruct "fspec update-foundation problemTitle"
    And the system reminder should NOT instruct "fspec update-foundation problemDefinition"

  Scenario: Using problemTitle command updates the title field correctly
    Given I have a foundation.json.draft with empty problemSpace.primaryProblem.title
    When I run "fspec update-foundation problemTitle 'User Authentication Issues'"
    Then the problemSpace.primaryProblem.title field should contain "User Authentication Issues"
    And the problemSpace.primaryProblem.description field should remain unchanged

  Scenario: Using problemDefinition command updates the description field, not title
    Given I have a foundation.json.draft with empty problemSpace.primaryProblem fields
    When I run "fspec update-foundation problemDefinition 'Users lack structured auth workflow'"
    Then the problemSpace.primaryProblem.description field should contain "Users lack structured auth workflow"
    And the problemSpace.primaryProblem.title field should remain empty
