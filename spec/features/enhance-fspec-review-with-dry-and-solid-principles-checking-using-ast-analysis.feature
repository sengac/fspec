@done
@validation
@code-quality
@cli
@high
@REVIEW-001
Feature: Enhance fspec review with DRY and SOLID principles checking using AST analysis

  """
  Uses fspec research --tool=ast for structural data gathering. Creates new module ast-data-gatherer.ts to execute AST commands programmatically. Integrates with existing review.ts command to emit system-reminders with data and guidance questions. NO semantic analysis implemented in fspec - AI agents make all quality decisions.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. fspec MUST NOT perform semantic code analysis or make quality judgments
  #   2. fspec review must gather structural data using fspec research --tool=ast commands
  #   3. System-reminders must present structural data in neutral, factual format without judgments
  #   4. System-reminders must suggest AST commands AI can run for deeper investigation
  #   5. System-reminders must include guidance questions to prompt AI thinking about DRY/SOLID principles
  #   6. fspec review must gather function counts, class counts, import counts, and export counts per file
  #   7. fspec review must identify functions/classes with identical names across multiple files
  #   8. AST data gathering must add less than 2 seconds overhead to fspec review command
  #
  # EXAMPLES:
  #   1. AI runs fspec review AUTH-001, receives system-reminder with function counts for each implementation file
  #   2. System-reminder shows formatOutput() function appears in review.ts:156 and validate.ts:234
  #   3. System-reminder suggests running: fspec research --tool=ast --operation=list-functions --file=src/commands/review.ts
  #   4. System-reminder asks: Are there functions with similar names that might have duplicate logic?
  #   5. System-reminder reports ReviewCommand class has 15 public methods detected
  #   6. System-reminder says 'Patterns detected (NOT violations, just observations)' when listing similar function names
  #   7. AI reads system-reminder data, runs suggested AST commands, uses Read tool, and decides if refactoring is needed
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec review command
    I want to receive AST-based structural data about implementation files
    So that I can analyze code for DRY/SOLID principles using the data and tools provided

  Scenario: Gather structural data for implementation files
    Given a work unit AUTH-001 has implementation files with test coverage
    When AI runs fspec review AUTH-001
    Then a system-reminder should be emitted with structural data
    And the system-reminder should include function counts for each file
    And the system-reminder should include class counts for each file


  Scenario: Identify functions with identical names across files
    Given implementation files have formatOutput() in review.ts and validate.ts
    When fspec review runs AST analysis
    Then the system-reminder should show formatOutput() appears in review.ts:156 and validate.ts:234


  Scenario: Suggest AST commands for deeper investigation
    Given fspec review has gathered initial structural data
    When the system-reminder is generated
    Then it should include command: fspec research --tool=ast --operation=list-functions --file=src/commands/review.ts


  Scenario: Include guidance questions for AI analysis
    Given structural data has been gathered
    When the system-reminder is formatted
    Then it should ask: Are there functions with similar names that might have duplicate logic?


  Scenario: Present data in neutral format without judgments
    Given similar function names have been detected
    When the system-reminder describes the pattern
    Then it should say: Patterns detected (NOT violations, just observations)
    And it should NOT say: violation detected or similarity score calculated

