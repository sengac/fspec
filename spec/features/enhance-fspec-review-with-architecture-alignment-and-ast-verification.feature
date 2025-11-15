@workflow
@high
@cli
@acdd
@code-quality
@REMIND-014
Feature: Enhance fspec review with architecture alignment and AST verification

  """
  Critical implementation: Must check work unit attachments for AST research results. Must validate architectural notes reference actual code (not assumptions). Must align proposed approach with FOUNDATION.md/CLAUDE.md/AGENTS.md principles. Must detect if AI is reinventing existing utilities/functions. Must follow existing ast-data-gatherer.ts pattern (data presentation, not judgment).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Architectural review must be automatic and mandatory before transitioning from specifying to testing
  #   2. AST analysis must verify: all referenced files were analyzed, all mentioned functions/classes exist, all dependencies/imports are documented, and refactoring candidates were researched via AST
  #   3. Architecture alignment must verify: proposed approach matches existing code patterns, decisions align with FOUNDATION.md/CLAUDE.md/AGENTS.md principles, architectural notes reference actual discovered code structures, and explicit justification required when diverging from established patterns
  #   4. DRY/SOLID validation must verify: AI performed AST research during specifying phase (check attachments), architectural notes reference actual discovered code (not assumptions), AI is not proposing to reinvent existing utilities/functions, and proposed solutions follow SOLID principles based on architectural notes
  #   5. Review enforcement must be hard block implemented inside update-work-unit-status command (not as hook), automatically run review when transitioning specifying to testing, throw error and prevent state change if critical issues or ACDD compliance failed, output full review report with specific issues and guidance
  #   6. CRITICAL: fspec MUST NOT perform semantic code analysis - NO code quality judgments, NO similarity detection, NO anti-pattern detection in fspec itself. fspec ONLY provides structural data via AST (counts, names, locations). The AI agent makes ALL analysis decisions based on data provided.
  #   7. Review has two-level blocking: Level 1 (Objective ACDD) - fspec hard blocks for no Example Mapping, no architectural notes, no AST research attachments, no feature file, invalid Gherkin. Level 2 (Subjective Analysis) - review passes but emits system-reminder with AST data, architectural notes, guidance questions for AI to analyze and decide whether to continue or revert to specifying
  #
  # EXAMPLES:
  #   1. AI moves work unit from specifying to testing, review auto-runs and finds no AST research attachments, command fails with error: 'Cannot transition to testing - no AST research performed during discovery', AI must go back and run fspec research --tool=ast
  #   2. AI moves work unit from specifying to testing, objective checks pass, review outputs system-reminder showing AST data (15 functions found across 3 files, validateFeature appears in 2 locations), AI analyzes data and realizes it was about to reinvent validateFeature, AI reverts to specifying using fspec update-work-unit-status WORK-001 specifying
  #   3. AI creates architectural note 'Refactoring: Extract validation logic to shared utility because current code has 3 copies of same validation pattern across files', attaches AST research showing the 3 locations, review passes and allows transition to testing with all data validated
  #
  # ========================================

  Background: User Story
    As a AI agent working on a feature
    I want to run architectural review before moving to testing phase
    So that I ensure alignment with existing codebase architecture, follow DRY/SOLID principles, and verify complete code understanding via AST analysis

  Scenario: Block transition when AST research is missing
    Given I have a work unit in specifying state with completed feature file
    When I run 'fspec update-work-unit-status WORK-001 testing'
    Then the command should fail with error code 1
    And the work unit has no AST research attachments
    And the error message should contain 'Cannot transition to testing - no AST research performed during discovery'
    And the work unit status should remain 'specifying'


  Scenario: AI detects reinvention via AST data in system-reminder
    Given I have a work unit with AST research attachments showing 'validateFeature' exists in 2 files
    When I run 'fspec update-work-unit-status WORK-001 testing'
    Then the command should succeed with status code 0
    And the work unit has architectural notes and feature file
    And a system-reminder should be emitted with AST structural data
    And the system-reminder should show 'validateFeature appears in 2 files'
    And the AI should revert to specifying state after analyzing the data


  Scenario: Pass review with proper architecture documentation and AST research
    Given I have a work unit with architectural note 'Refactoring: Extract validation logic to shared utility'
    When I run 'fspec update-work-unit-status WORK-001 testing'
    Then the command should succeed with status code 0
    And the work unit has AST research attachments showing 3 copies of validation pattern
    And the work unit has a completed feature file with no prefill placeholders
    And the work unit status should be 'testing'
    And the review validation should pass all objective ACDD checks

