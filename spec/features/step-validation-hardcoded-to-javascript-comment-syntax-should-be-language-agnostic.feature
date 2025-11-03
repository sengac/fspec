@done
@validation
@critical
@step-validation
@utils
@cross-platform
@BUG-063
Feature: @step validation hardcoded to JavaScript comment syntax, should be language-agnostic

  """
  Current implementation in src/utils/step-validation.ts uses hardcoded regex: /^\/\/\s*@step\s+(Given|When|Then|And|But)\s+(.+)$/ on line 53-54
  Function extractStepComments() needs to be refactored to use language-agnostic regex that matches @step pattern anywhere in the line
  New regex should match: .*@step\s+(Given|When|Then|And|But)\s+(.+?)(?:\s*\*\/)?$ to capture step text and ignore trailing comment delimiters
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. @step validation must be language-agnostic and support all comment syntaxes
  #   2. Must match @step anywhere in a comment line, not just at the start after //
  #   3. Must extract only the step text between keyword and end of step, ignoring comment delimiters
  #   4. Must support block comments like /* @step Given text */ and line comments like // @step, # @step, -- @step, % @step, ' @step
  #
  # EXAMPLES:
  #   1. JavaScript: // @step Given a user is authenticated → should extract 'Given a user is authenticated'
  #   2. Python: # @step When I click the button → should extract 'When I click the button'
  #   3. SQL: -- @step Then I see the result → should extract 'Then I see the result'
  #   4. Block comment: /* @step Given a user is authenticated */ → should extract 'Given a user is authenticated'
  #   5. MATLAB: % @step And the database is updated → should extract 'And the database is updated'
  #   6. Visual Basic: ' @step But the error is logged → should extract 'But the error is logged'
  #
  # ========================================

  Background: User Story
    As a developer using fspec with any programming language
    I want to use @step comments in my test files
    So that the validation works regardless of my language's comment syntax

  Scenario: Extract @step comment from JavaScript-style line comment
    Given a test file contains the line "// @step Given a user is authenticated"
    When the extractStepComments function processes the file
    Then it should extract a step comment with keyword "Given"
    And the step text should be "a user is authenticated"

  Scenario: Extract @step comment from Python-style line comment
    Given a test file contains the line "# @step When I click the button"
    When the extractStepComments function processes the file
    Then it should extract a step comment with keyword "When"
    And the step text should be "I click the button"

  Scenario: Extract @step comment from SQL-style line comment
    Given a test file contains the line "-- @step Then I see the result"
    When the extractStepComments function processes the file
    Then it should extract a step comment with keyword "Then"
    And the step text should be "I see the result"

  Scenario: Extract @step comment from block comment
    Given a test file contains the line "/* @step Given a user is authenticated */"
    When the extractStepComments function processes the file
    Then it should extract a step comment with keyword "Given"
    And the step text should be "a user is authenticated"

  Scenario: Extract @step comment from MATLAB-style line comment
    Given a test file contains the line "% @step And the database is updated"
    When the extractStepComments function processes the file
    Then it should extract a step comment with keyword "And"
    And the step text should be "the database is updated"

  Scenario: Extract @step comment from Visual Basic-style line comment
    Given a test file contains the line "' @step But the error is logged"
    When the extractStepComments function processes the file
    Then it should extract a step comment with keyword "But"
    And the step text should be "the error is logged"

  Scenario: Match @step anywhere in the line, not just at the start
    Given a test file contains the line "  // @step Given I am logged in"
    When the extractStepComments function processes the file
    Then it should extract a step comment with keyword "Given"
    And the step text should be "I am logged in"

  Scenario: Ignore trailing comment delimiters from block comments
    Given a test file contains the line "/* @step When I submit the form */ // trailing comment"
    When the extractStepComments function processes the file
    Then it should extract a step comment with keyword "When"
    And the step text should be "I submit the form"
    And the step text should not contain "*/"
