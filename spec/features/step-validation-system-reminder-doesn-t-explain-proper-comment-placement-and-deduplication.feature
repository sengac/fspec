@validation
@coverage
@cli
@critical
@COV-053
Feature: Step validation system-reminder doesn't explain proper comment placement and deduplication
  """
  The step validation system-reminder is generated in the link-coverage command when validating step comments in test files. The error message format is currently just 'Add to test file: // @step [step text]' without additional context.
  The fix should enhance the system-reminder message to include: 1) Instruction to place comments near executing code, 2) Instruction to check for and remove duplicate comments, 3) Example showing proper placement with context about which line executes the step.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Step validation system-reminder MUST explain that step comments should be placed near the code that executes each step, not just anywhere in the test
  #   2. Step validation system-reminder MUST explain that AI should remove duplicate Given/When/Then comments before adding @step comments to avoid redundancy
  #   3. Step validation system-reminder should provide an example showing proper step comment placement with context about which line of code executes that step
  #
  # EXAMPLES:
  #   1. Current behavior: System-reminder says 'Add to test file: // @step When the scanDraftForNextField function processes the draft'. AI adds comment without understanding where to place it or that existing Given/When/Then comments might be duplicates.
  #   2. Expected behavior: System-reminder explains 'Place step comments NEAR the code that executes each step. If you have duplicate Given/When/Then comments, remove them first. Example: Place "// @step When the scanDraftForNextField function processes the draft" right before the line that calls scanDraftForNextField() or discoverFoundation() which internally calls it.'
  #   3. AI receives step validation error, reads test file, sees existing '// Given I have a foundation.json.draft' comments, doesn't realize these might conflict with '@step Given' comments, adds @step comments without removing duplicates, creates confusing redundant comments
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec for test coverage linking
    I want to receive clear guidance about step comment placement
    So that I correctly place step comments near relevant code and avoid duplicating existing comments

  Scenario: System-reminder includes placement guidance for step comments
    Given a test file is missing step comments for a scenario
    When the link-coverage command validates step comments
    And step validation fails
    Then the system-reminder should contain "Place step comments NEAR the code that executes each step"
    And the system-reminder should explain which line of code executes the step
    And the system-reminder should provide a contextual example of proper placement

  Scenario: System-reminder includes deduplication guidance for step comments
    Given a test file has existing Given/When/Then comments
    And the test file is missing @step comments for a scenario
    When the link-coverage command validates step comments
    And step validation fails
    Then the system-reminder should contain "If you have duplicate Given/When/Then comments, remove them first"
    And the system-reminder should explain that @step comments replace existing step comments
    And the system-reminder should warn against creating redundant comments

  Scenario: System-reminder provides concrete example of step comment placement
    Given step validation fails for a missing step comment
    When the system-reminder is generated
    Then the reminder should include an example showing proper placement
    And the example should reference the specific code line that executes the step
    And the example should show the @step comment placed immediately before the executing code
