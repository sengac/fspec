@done
@EXMAP-002
@phase7
@cli
@generator
@example-mapping
@acdd
Feature: Preserve example mapping context as comments in generated feature files

  """
  Architecture notes:
  - Gherkin parser (@cucumber/gherkin) captures comments in ast.comments array
  - Current formatter (gherkin-formatter.ts) ignores ast.comments field - MUST be enhanced
  - Comments use # syntax (Gherkin standard), doc strings use triple quotes (for architecture)
  - Example mapping data (rules, examples, questions, assumptions) embedded as # comments
  - System-reminders guide AI to write scenarios based on embedded examples
  - Comments persist through all format/edit cycles (living documentation)

  Critical implementation requirements:
  - MUST enhance formatGherkinDocument() to preserve ast.comments
  - MUST generate context-only feature files (comments + Background, NO scenarios initially)
  - MUST emit system-reminder instructing AI to write scenarios
  - MUST NOT conflict with add-architecture (doc strings) or add-background commands
  - Generated feature files MUST contain zero scenarios (AI writes them later)
  """

  Background: User Story
    As a AI agent using fspec for ACDD workflow
    I want to see full example mapping context when writing scenarios
    So that I can write meaningful Given/When/Then steps based on business rules and concrete examples

  Scenario: Formatter preserves comments through format cycles
    Given I have a feature file with example mapping comments
    And the file contains "# BUSINESS RULES:" comment block
    When I run "fspec format" on the file
    Then the formatted file should still contain the comment block
    And the comments should be in the same location
    And no comments should be lost

  Scenario: Example mapping comments coexist with architecture doc strings
    Given I generate scenarios from a work unit
    And the feature file has "# EXAMPLE MAPPING CONTEXT" comments
    When I run "fspec add-architecture" to add technical notes
    Then the file should contain both # comments AND """ doc string
    And neither should overwrite the other
    And format command should preserve both

  Scenario: Generate context-only feature file with no scenarios
    Given I have a work unit EXMAP-002 with example mapping data
    And the work unit has 3 rules and 5 examples
    When I run "fspec generate-scenarios EXMAP-002"
    Then a feature file should be created
    And the file should contain # comment block with all rules and examples
    And the file should contain Background with user story
    And the file should contain ZERO scenarios
    And a system-reminder should tell AI to write scenarios

  Scenario: User story embedded in both comments and Background section
    Given I set user story for work unit using "fspec set-user-story"
    When I run "fspec generate-scenarios" on the work unit
    Then the feature file # comments should contain the user story
    And the Background section should contain the same user story
    And both locations should use identical text

  Scenario: System-reminder guides AI to write scenarios from examples
    Given I generate scenarios for a work unit
    And the feature file contains "# EXAMPLES: 1. User logs in with valid creds"
    When the command completes
    Then a system-reminder should be emitted
    And the reminder should say "write scenarios based on # EXAMPLES section"
    And the reminder should list all examples found
    And the reminder should instruct using Edit tool

  Scenario: Business rules in comments inform scenario preconditions
    Given a feature file has "# BUSINESS RULES: Passwords must be 8+ chars"
    When an AI agent reads the file to write scenarios
    Then the agent should reference the rule in Given steps
    And scenarios should validate the 8+ character requirement

  Scenario: Answered questions preserved in comments
    Given I answer question during example mapping
    And the answer says "OAuth support deferred to Phase 2"
    When I generate scenarios from the work unit
    Then the feature file should contain "# QUESTIONS (ANSWERED):" section
    And the section should show "Q: OAuth support? A: Phase 2"
    And developers reading the file understand deferred decisions

  Scenario: Assumptions documented in comments
    Given I add assumption "email verification handled by external service"
    When I generate scenarios from the work unit
    Then the feature file should contain "# ASSUMPTIONS:" section
    And the section should list the email verification assumption
    And AI agents know not to write scenarios for email verification

  Scenario: Comment block has visual borders for easy identification
    Given I generate scenarios from a work unit
    When I view the generated feature file
    Then the comment block should have "# ===" separator lines at top and bottom
    And the block should be visually distinct when scrolling

  Scenario: Multiple examples converted to separate scenarios
    Given a work unit has 3 examples in example mapping
    And the examples describe different behaviors
    When an AI agent writes scenarios based on the comments
    Then the agent can create 3 separate scenarios
    And the agent can combine related examples into single scenario
    And the agent decides based on semantic similarity

  Scenario: Add-scenario command preserves example mapping comments
    Given a feature file has # example mapping comments at the top
    When I run "fspec add-scenario" to manually add a scenario
    Then the new scenario should be appended at the end
    And the # comments at the top should remain unchanged
    And no comments should be removed

  Scenario: Add-background command preserves example mapping comments
    Given a feature file has # comments before Background section
    When I run "fspec add-background" to update the user story
    Then the Background section should be replaced
    And the # comments above Background should remain intact
    And no comments should be lost

  Scenario: Gherkin parser captures comments in AST
    Given I have a feature file with # comment lines
    When the file is parsed by @cucumber/gherkin parser
    Then the AST should contain ast.comments array
    And each comment should have line number and text
    And comments are not stripped during parsing

  Scenario: Formatter outputs comments from AST to correct positions
    Given I have a Gherkin AST with comments in ast.comments
    When formatGherkinDocument() is called
    Then the formatted output should include all comments
    And comments should be inserted at their original line positions
    And comments should appear before the elements they precede

  Scenario: Empty example map creates minimal comment block
    Given a work unit has no rules, examples, or questions
    When I run "fspec generate-scenarios" on it
    Then the feature file should contain a comment block
    And the block should say "# No example mapping data captured"
    And the file should still contain Background section

  Scenario: Prefill detection still works after adding comments
    Given I generate scenarios without setting user story
    And the Background contains placeholder text
    When the command completes
    Then a system-reminder should detect the prefill
    And the reminder should suggest "fspec set-user-story" command
    And the prefill reminder should coexist with scenario generation reminder

  Scenario: Git diffs show example mapping context
    Given a feature file with # example mapping comments
    When a developer reviews a PR containing the file
    Then the git diff should show the # comment block
    And the developer understands why scenarios exist
    And the developer sees business rules and examples
    And the context aids code review

  Scenario: Existing comments are preserved when adding example mapping
    Given a feature file already has some # comments
    When I generate scenarios and add example mapping comments
    Then both the old comments and new comments should exist
    And the example mapping comment block should be clearly separated
    And no existing comments should be overwritten

  Scenario: Zero scenarios means file incomplete - system reminder triggered
    Given I generate scenarios from a work unit
    And the generated file has # comments + Background
    But the file has zero Scenario blocks
    When the command completes
    Then a system-reminder should be emitted
    And the reminder should say "now write scenarios"
    And the reminder should reference the # EXAMPLES section

  Scenario: AI uses Edit tool with full context visible
    Given a feature file with "# EXAMPLES: 1. User logs in with valid creds"
    When an AI agent opens the file
    Then the agent can see the full example mapping context
    And the agent can write "Scenario: User logs in with valid credentials"
    And the agent can write proper Given/When/Then steps
    And the agent uses Edit tool to add scenarios directly
