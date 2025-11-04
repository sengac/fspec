@done
@cli
@high
@validation
@system-reminder
@acdd
@REFAC-003
Feature: Improve @step workflow clarity to prevent wrong test file and missing comments

  """
  Add guidance about DELETE and RECREATE when tests written without @step comments (JOURNAL.md BUG-065 pattern)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. @step requirement currently buried in 42-line testing state reminder (lines 94-113 of system-reminder.ts)
  #   2. AI adds @step comments to WRONG test file, causing validation to fail even though comments exist
  #   3. ONE scenario must map to ONE test with ALL @step comments in THAT test (not spread across files)
  #   4. @step comments must be added DURING test writing, not as separate step after
  #   5. When AI creates tests without @step comments, it should DELETE and RECREATE tests correctly rather than editing
  #   6. Validation error already shows missing steps with exact text (formatValidationError in step-validation.ts:243-248)
  #
  # EXAMPLES:
  #   1. JOURNAL.md BUG-064: AI only learned about @step requirement when blocked, said requirement was 'buried in long message'
  #   2. AI writes tests without @step, link-coverage fails, AI then adds @step comments but to wrong test file, validation still fails
  #   3. JOURNAL.md BUG-065: User told AI to 'just recreate the test' instead of editing when @step placement was wrong structurally
  #   4. Correct workflow: AI creates test WITH @step comments from the start, validation passes immediately
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec ACDD workflow
    I want to understand @step requirements clearly before writing tests
    So that I create tests correctly the first time with proper @step comments in the right test file

  Scenario: Testing state reminder makes @step requirement prominent
    Given work unit moves to testing state
    When system-reminder is shown
    Then @step requirement should be in FIRST 10 lines of reminder
    And requirement should emphasize ONE scenario = ONE test mapping
    And reminder should state @step comments added DURING test writing


  Scenario: Validation error shows which test file is being checked
    Given test file missing @step comments
    When link-coverage validation fails
    Then error message should show test file path being validated
    And error should warn about adding @step to CORRECT test file


  Scenario: Reminder guides recreation when tests created without @step comments
    Given tests were created without @step comments
    When validation error occurs
    Then error should suggest DELETE and RECREATE tests ONLY if created in current work unit
    And error should explain recreation is better than editing for structural issues
    And error should state existing tests from other cards should use checkpoint restore instead

