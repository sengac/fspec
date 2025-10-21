@foundation-management
@error-handling
@cli
@phase1
@FOUND-009
Feature: Foundation missing error message is not imperative enough
  """
  Architecture notes:
  - Error message located in src/utils/foundation-check.ts buildFoundationMissingError()
  - Must rewrite error message to be imperative and forbid manual JSON creation
  - Must include detailed 3-step workflow instructions in error output
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Error message must FORBID manual creation of foundation.json files
  #   2. Error message must explain foundation.json is a detailed PRD, not a quick summary
  #   3. Error message must show exact command sequence: discover-foundation creates draft, AI fills placeholders, discover-foundation --finalize creates final file
  #
  # EXAMPLES:
  #   1. AI tries to create work unit, gets error saying 'NEVER manually create foundation.json - use discover-foundation workflow'
  #   2. Error message shows: Step 1: fspec discover-foundation (creates draft), Step 2: Fill [QUESTION:] placeholders, Step 3: fspec discover-foundation --finalize
  #
  # ========================================
  Background: User Story
    As a AI agent without foundation.json
    I want to receive imperative guidance on foundation discovery workflow
    So that I follow the correct interactive workflow instead of manually creating JSON files

  Scenario: Error forbids manual foundation.json creation
    Given foundation.json does not exist
    When AI agent attempts to create a work unit
    Then error message must contain "NEVER manually create foundation.json"
    And error message must instruct to use discover-foundation workflow

  Scenario: Error shows complete workflow steps
    Given foundation.json does not exist
    When AI agent receives foundation missing error
    Then error must show "Step 1: fspec discover-foundation (creates draft)"
    And error must show "Step 2: Fill [QUESTION:] placeholders in draft"
    And error must show "Step 3: fspec discover-foundation --finalize"
