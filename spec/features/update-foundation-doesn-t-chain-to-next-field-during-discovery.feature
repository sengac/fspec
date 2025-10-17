@done
@validation
@foundation
@cli
@phase1
@BUG-017
Feature: update-foundation doesn't chain to next field during discovery

  """
  Architecture notes:
  - update-foundation command must call discoverFoundation({ scanOnly: true }) after updating draft
  - scanOnly mode scans draft and returns next field guidance without modifying files
  - System-reminder must be emitted to stdout after success message
  - If no next field, emit "all fields complete" reminder with finalize command
  - Chaining happens in updateFoundationCommand() after successful draft update
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. After updating draft field, MUST automatically scan draft for next field
  #   2. MUST emit system-reminder with guidance for next unfilled field
  #   3. If all fields complete, MUST tell AI to run finalize
  #   4. Chaining MUST be automatic, no manual intervention required
  #
  # EXAMPLES:
  #   1. AI runs 'fspec update-foundation projectName "MyProject"', sees success message PLUS system-reminder for Field 2/8: project.vision
  #   2. AI runs 'fspec update-foundation projectVision "My vision"', sees success message PLUS system-reminder for Field 3/8: project.projectType
  #   3. AI fills last field, sees success message PLUS system-reminder saying 'All fields complete. Run: fspec discover-foundation --finalize'
  #
  # ========================================

  Background: User Story
    As a AI agent using draft-driven discovery
    I want to automatically see next field guidance after updating a field
    So that I don't have to manually re-run discover-foundation or guess what comes next

  Scenario: Chain to next field after updating first field
    Given I have a foundation.json.draft with placeholder fields
    When I run `fspec update-foundation projectName "MyProject"`
    Then the command should exit with code 0
    And the output should contain "Updated \"projectName\" in foundation.json.draft"
    And the output should contain a system-reminder for Field 2/8
    And the system-reminder should mention "project.vision"
    And the system-reminder should include the command to run next

  Scenario: Chain to next field after updating middle field
    Given I have a foundation.json.draft with first field filled
    When I run `fspec update-foundation projectVision "My vision"`
    Then the command should exit with code 0
    And the output should contain "Updated \"projectVision\" in foundation.json.draft"
    And the output should contain a system-reminder for Field 3/8
    And the system-reminder should mention "project.projectType"
    And the system-reminder should include the command to run next

  Scenario: Show finalize reminder when all fields complete
    Given I have a foundation.json.draft with all fields except last filled
    When I run `fspec update-foundation solutionOverview "My solution"`
    Then the command should exit with code 0
    And the output should contain "Updated \"solutionOverview\" in foundation.json.draft"
    And the output should contain a system-reminder saying all fields complete
    And the system-reminder should include "fspec discover-foundation --finalize"
