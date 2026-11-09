@done
@EXMAP-002
@cli
@generator
@example-mapping
@acdd
Feature: Preserve example mapping context as comments in generated feature files
  """
  Architecture notes:
  - generate-scenarios (rust/fspec-core/src/commands/generate_scenarios.rs) writes a context-only file: tags, Feature line, architecture doc string, the bordered # EXAMPLE MAPPING CONTEXT comment block (BUSINESS RULES, EXAMPLES, QUESTIONS (ANSWERED), ASSUMPTIONS) and the Background with the user story; no scenarios
  - The user story appears only in the Background, never in the comments (FEAT-012 separation)
  - Comments use # syntax (Gherkin standard); architecture notes stay in the doc string, so add-architecture and the comment block never overwrite each other
  - One consolidated system-reminder tells the AI to write scenarios from the # EXAMPLES section with the Edit tool, plus the prefill reminder when the Background still holds placeholders
  - The gherkin crate's parser drops comments, so the formatter (rust/fspec-core/src/io/gherkin_format.rs extract_comments / insert_comments_before) re-extracts them from the raw source and writes each one back before the element it preceded; add-scenario and add-background keep the block
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

  Scenario: User story only in Background section, not in comments
    Given I set user story for work unit using "fspec set-user-story"
    When I run "fspec generate-scenarios" on the work unit
    Then the feature file # comments should NOT contain the user story
    And the Background section should contain the user story
    And the user story should only appear in Background (FEAT-012 separation)

  Scenario: System-reminder guides AI to write scenarios from examples
    Given I generate scenarios for a work unit
    And the feature file lists "1. User logs in with valid creds" under "# EXAMPLES:"
    When the command completes
    Then a system-reminder should be emitted
    And the reminder should say "Write scenarios based on # EXAMPLES section"
    And the reminder should list all examples found
    And the reminder should instruct using Edit tool

  Scenario: Answered questions preserved in comments
    Given I answer question during example mapping
    And the answer says "OAuth support deferred to Phase 2"
    When I generate scenarios from the work unit
    Then the feature file should contain "# QUESTIONS (ANSWERED):" section
    And the section should show "Q: OAuth support?" followed by "A: OAuth support deferred to Phase 2"

  Scenario: Assumptions documented in comments
    Given I add assumption "email verification handled by external service"
    When I generate scenarios from the work unit
    Then the feature file should contain "# ASSUMPTIONS:" section
    And the section should list the email verification assumption
    And the scenario-generation reminder says "Check # ASSUMPTIONS to know what NOT to test"

  Scenario: Comment block has visual borders for easy identification
    Given I generate scenarios from a work unit
    When I view the generated feature file
    Then the comment block should have "# ===" separator lines at top and bottom
    And the block starts with "# EXAMPLE MAPPING CONTEXT" between the first two separator lines

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

  Scenario: Formatter puts comments back at their original positions
    Given I have a feature file with comments before the Feature keyword, a Scenario and a step
    When the formatter formats it
    Then the formatted output should include all comments
    And an indented comment keeps its indentation
    And comments should appear before the elements they precede

  Scenario: Prefill detection still works after adding comments
    Given I generate scenarios without setting user story
    And the Background contains placeholder text
    When the command completes
    Then a system-reminder should detect the prefill
    And the reminder should suggest "fspec set-user-story" command
    And the prefill reminder should coexist with scenario generation reminder

  Scenario: Zero scenarios means file incomplete - system reminder triggered
    Given I generate scenarios from a work unit
    And the generated file has # comments + Background
    But the file has zero Scenario blocks
    When the command completes
    Then a system-reminder should be emitted
    And the reminder should say "ZERO scenarios (AI must write them)"
    And the reminder should reference the # EXAMPLES section
