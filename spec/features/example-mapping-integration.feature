@phase7
@cli
@project-management
@example-mapping
@bdd
Feature: Example Mapping Integration
  """
  Architecture notes:
  - Example mapping is a BDD discovery technique used during specifying state
  - Work units capture four key artifacts: rules, examples, questions, assumptions
  - Rules: Business rules that govern the feature
  - Examples: Concrete examples that will become Gherkin scenarios
  - Questions: Unknowns that block progress (requires human answers)
  - Assumptions: Things assumed to be true
  - Questions can mention people with @username syntax for notification
  - Examples can be converted to Gherkin scenarios with auto-tagging

  Critical implementation requirements:
  - MUST store example mapping data in work unit's rules, examples, questions, assumptions arrays
  - MUST support adding/removing individual items by index
  - MUST support bulk operations for rapid discovery sessions
  - MUST validate questions are answered before moving from specifying to testing
  - MUST auto-tag generated scenarios with work unit ID
  - MUST preserve example mapping data after scenario generation (for reference)
  - Questions with length > 0 should prevent or warn about state transitions

  Data model:
  - work-units.json: Each work unit has optional arrays: rules, examples, questions, assumptions
  - Questions format: "question text" or "@person: question text"
  - Generated scenarios tagged with @WORK-UNIT-ID in feature files

  Integration points:
  - Used during specifying state before writing formal Gherkin
  - Questions may trigger blocked state if human input needed
  - Examples become seeds for fspec generate-scenarios command

  References:
  - Project Management Design: project-management.md (section 9: Example Mapping)
  - Example Mapping Technique: https://cucumber.io/blog/bdd/example-mapping-introduction/
  """

  Background: User Story
    As an AI agent in the specifying state
    I want to capture rules, examples, questions, and assumptions through example mapping
    So that I can properly discover and document requirements before writing Gherkin scenarios

  @critical
  @happy-path
  Scenario: Add rule to work unit during discovery
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    When I run "fspec add-rule AUTH-001 'Users must authenticate before accessing protected resources'"
    Then the command should succeed
    And the work unit should have 1 rule
    And the rule should be "Users must authenticate before accessing protected resources"

  @happy-path
  Scenario: Add multiple rules to work unit
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    When I run "fspec add-rule AUTH-001 'OAuth tokens expire after 1 hour'"
    And I run "fspec add-rule AUTH-001 'Refresh tokens valid for 30 days'"
    And I run "fspec add-rule AUTH-001 'Only one active session per user'"
    Then the work unit should have 3 rules
    And the rules should be in order

  @happy-path
  Scenario: Add example that will become a scenario
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    When I run "fspec add-example AUTH-001 'User logs in with Google account'"
    Then the command should succeed
    And the work unit should have 1 example
    And the example should be "User logs in with Google account"

  @happy-path
  Scenario: Add multiple examples for scenario candidates
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    When I run "fspec add-example AUTH-001 'User logs in with valid credentials'"
    And I run "fspec add-example AUTH-001 'User logs in with expired token'"
    And I run "fspec add-example AUTH-001 'User token auto-refreshes before expiry'"
    And I run "fspec add-example AUTH-001 'User logs out and token is invalidated'"
    Then the work unit should have 4 examples

  @happy-path
  Scenario: Add question that needs human answer
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    When I run "fspec add-question AUTH-001 'Should we support GitHub Enterprise?'"
    Then the command should succeed
    And the work unit should have 1 question
    And the question should be "Should we support GitHub Enterprise?"

  @happy-path
  Scenario: Add question mentioning specific person
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    When I run "fspec add-question AUTH-001 '@bob: What is the token expiry policy?'"
    Then the command should succeed
    And the question should contain "@bob:"
    And the question should be parseable for notifications

  @happy-path
  Scenario: Add assumption about requirements
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    When I run "fspec add-assumption AUTH-001 'Users have valid OAuth accounts'"
    Then the command should succeed
    And the work unit should have 1 assumption

  @happy-path
  Scenario: Complete example mapping session with all four artifacts
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    When I run "fspec add-rule AUTH-001 'OAuth tokens expire after 1 hour'"
    And I run "fspec add-rule AUTH-001 'Users must authenticate before accessing protected resources'"
    And I run "fspec add-example AUTH-001 'User logs in with Google'"
    And I run "fspec add-example AUTH-001 'User logs in with expired token'"
    And I run "fspec add-question AUTH-001 '@security-team: Do we need PKCE flow?'"
    And I run "fspec add-assumption AUTH-001 'Users have valid email addresses'"
    Then the work unit should have 2 rules
    And the work unit should have 2 examples
    And the work unit should have 1 question
    And the work unit should have 1 assumption

  @read
  @query
  Scenario: Show work unit with example mapping data
    Given I have a project with spec directory
    And a work unit "AUTH-001" has example mapping data:
      | type       | content                          |
      | rule       | OAuth tokens expire after 1 hour |
      | rule       | Users must authenticate first    |
      | example    | User logs in with Google         |
      | example    | User logs in with expired token  |
      | question   | @bob: Support GitHub Enterprise? |
      | assumption | Users have valid OAuth accounts  |
    When I run "fspec show-work-unit AUTH-001"
    Then the output should display all rules
    And the output should display all examples
    And the output should display all questions
    And the output should display all assumptions
    And the output should group them by type

  @remove
  @happy-path
  Scenario: Remove rule by index
    Given I have a project with spec directory
    And a work unit "AUTH-001" has rules:
      | OAuth tokens expire after 1 hour |
      | Users must authenticate first    |
      | Refresh tokens valid for 30 days |
    When I run "fspec remove-rule AUTH-001 1"
    Then the command should succeed
    And the work unit should have 2 rules
    And the second rule should now be "Refresh tokens valid for 30 days"

  @remove
  @happy-path
  Scenario: Remove example by index
    Given I have a project with spec directory
    And a work unit "AUTH-001" has examples:
      | User logs in with Google        |
      | User logs in with expired token |
      | User logs out                   |
    When I run "fspec remove-example AUTH-001 0"
    Then the command should succeed
    And the work unit should have 2 examples
    And the first example should be "User logs in with expired token"

  @remove
  @happy-path
  Scenario: Remove question by index
    Given I have a project with spec directory
    And a work unit "AUTH-001" has questions:
      | @bob: Support GitHub Enterprise? |
      | What is the token expiry policy? |
    When I run "fspec remove-question AUTH-001 0"
    Then the command should succeed
    And the work unit should have 1 question

  @remove
  @validation
  Scenario: Attempt to remove item with invalid index
    Given I have a project with spec directory
    And a work unit "AUTH-001" has 2 rules
    When I run "fspec remove-rule AUTH-001 5"
    Then the command should fail
    And the error should contain "Index 5 out of range"
    And the error should contain "Valid indices: 0-1"

  @answer-question
  @happy-path
  Scenario: Answer question and add to assumptions
    Given I have a project with spec directory
    And a work unit "AUTH-001" has questions:
      | @bob: Should we support GitHub Enterprise? |
    When I run "fspec answer-question AUTH-001 0 'No, only GitHub.com supported' --add-to=assumptions"
    Then the command should succeed
    And the work unit should have 0 questions
    And the work unit should have 1 assumption
    And the assumption should be "Only GitHub.com supported, not Enterprise"

  @answer-question
  @happy-path
  Scenario: Answer question and add to rules
    Given I have a project with spec directory
    And a work unit "AUTH-001" has questions:
      | What is the maximum session length? |
    When I run "fspec answer-question AUTH-001 0 '24 hours' --add-to=rules"
    Then the command should succeed
    And the work unit should have 0 questions
    And the work unit should have 1 rule
    And the rule should contain "24 hours"

  @answer-question
  @happy-path
  Scenario: Answer question without adding to rules or assumptions
    Given I have a project with spec directory
    And a work unit "AUTH-001" has questions:
      | Is this feature needed? |
    When I run "fspec answer-question AUTH-001 0 'No, descoping' --add-to=none"
    Then the command should succeed
    And the work unit should have 0 questions
    And the work unit should have 0 rules
    And the work unit should have 0 assumptions

  @generate-scenarios
  @critical
  Scenario: Generate context-only feature file with example mapping comments
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    And the work unit has user story "user", "log in with OAuth", "access my account"
    And the work unit has rules:
      | Valid OAuth token required |
    And the work unit has examples:
      | User logs in with OAuth |
    And the work unit has assumptions:
      | OAuth provider is available |
    When I run "fspec generate-scenarios AUTH-001 --feature=oauth-login"
    Then the command should succeed
    And a new feature file "spec/features/oauth-login.feature" should be created
    And the file should be tagged with "@AUTH-001"
    And the file should contain "Feature: OAuth login"
    And the file should contain "# EXAMPLE MAPPING CONTEXT"
    And the file should contain "# USER STORY:"
    And the file should contain "#   As a user"
    And the file should contain "# BUSINESS RULES:"
    And the file should contain "#   1. Valid OAuth token required"
    And the file should contain "# EXAMPLES:"
    And the file should contain "#   1. User logs in with OAuth"
    And the file should contain "# ASSUMPTIONS:"
    And the file should contain "#   1. OAuth provider is available"
    And the file should contain "Background: User Story"
    And the file should contain "As a user"
    And the file should contain "I want to log in with OAuth"
    And the file should contain "So that access my account"
    And the file should have ZERO scenarios

  @validation
  @blocking
  Scenario: Prevent moving to testing when questions remain unanswered
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    And the work unit has questions:
      | @bob: Should we support OAuth 2.0? |
    When I run "fspec update-work-unit AUTH-001 --status=testing"
    Then the command should fail
    And the error should contain "Unanswered questions prevent state transition"
    And the error should list the unanswered question
    And the error should suggest "Answer questions with 'fspec answer-question' or remove them"

  @validation
  @warning
  Scenario: Warn when moving to testing with no examples
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    And the work unit has 0 examples
    When I run "fspec update-work-unit AUTH-001 --status=testing"
    Then the command should display warning "No examples captured"
    And the command should suggest "Consider adding examples with 'fspec add-example'"
    But the transition should succeed

  @bulk-add
  @happy-path
  Scenario: Bulk add multiple items from JSON
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    And I have a JSON file "example-mapping.json" with:
      """
      {
        "rules": [
          "OAuth tokens expire after 1 hour",
          "Users must authenticate first"
        ],
        "examples": [
          "User logs in with Google",
          "User logs in with expired token"
        ],
        "questions": [
          "@bob: Support GitHub Enterprise?"
        ],
        "assumptions": [
          "Users have valid OAuth accounts"
        ]
      }
      """
    When I run "fspec import-example-map AUTH-001 example-mapping.json"
    Then the command should succeed
    And the work unit should have 2 rules
    And the work unit should have 2 examples
    And the work unit should have 1 question
    And the work unit should have 1 assumption

  @bulk-export
  @happy-path
  Scenario: Export example mapping to JSON
    Given I have a project with spec directory
    And a work unit "AUTH-001" has example mapping data:
      | type       | content                          |
      | rule       | OAuth tokens expire after 1 hour |
      | example    | User logs in with Google         |
      | question   | @bob: Support GitHub Enterprise? |
      | assumption | Users have valid OAuth accounts  |
    When I run "fspec export-example-map AUTH-001 --output=auth-example-map.json"
    Then the command should succeed
    And the file "auth-example-map.json" should contain valid JSON
    And the JSON should have arrays: rules, examples, questions, assumptions

  @query
  @filtering
  Scenario: Find all work units with unanswered questions
    Given I have a project with spec directory
    And work units exist:
      | id       | questions               |
      | AUTH-001 | @bob: Support OAuth?    |
      | AUTH-002 |                         |
      | DASH-001 | What should timeout be? |
      | API-001  |                         |
    When I run "fspec query work-units --has-questions --output=json"
    Then the output should contain 2 work units
    And the output should include "AUTH-001" and "DASH-001"

  @query
  @filtering
  Scenario: List work units by person mentioned in questions
    Given I have a project with spec directory
    And work units exist:
      | id       | questions                    |
      | AUTH-001 | @bob: Support GitHub?        |
      | AUTH-002 | @alice: What is the timeout? |
      | DASH-001 | @bob: Show user metrics?     |
    When I run "fspec query work-units --questions-for=@bob --output=json"
    Then the output should contain 2 work units
    And the output should include "AUTH-001" and "DASH-001"

  @consistency
  @validation
  Scenario: Validate example mapping data structure
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    When I run "fspec validate-work-units"
    Then the validation should check rules are strings
    And the validation should check examples are strings
    And the validation should check questions are strings
    And the validation should check assumptions are strings
    And the validation should check arrays don't contain nulls or empty strings

  @ui-refinement
  @happy-path
  Scenario: AI refines generated scenario with proper Given/When/Then
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And a scenario was generated from example "User logs in with Google"
    And the scenario has placeholder steps
    When AI agent refines the scenario with:
      """
      @auth-001
      Scenario: User logs in with Google account
        Given the user is not authenticated
        When the user clicks "Login with Google"
        And completes Google OAuth flow
        Then the user should be authenticated
        And a session token should be created
      """
    Then the scenario should be properly specified
    And the scenario should remain tagged with "@auth-001"

  @reporting
  @metrics
  Scenario: Show example mapping completeness metrics
    Given I have a project with spec directory
    And work units exist with example mapping:
      | id       | rules | examples | questions | assumptions |
      | AUTH-001 | 3     | 5        | 0         | 2           |
      | AUTH-002 | 0     | 0        | 3         | 0           |
      | DASH-001 | 2     | 4        | 1         | 1           |
    When I run "fspec query example-mapping-stats --output=json"
    Then the output should show:
      | metric                      | value |
      | work units with rules       | 2     |
      | work units with examples    | 2     |
      | work units with questions   | 2     |
      | work units with assumptions | 2     |
      | avg rules per work unit     | 1.67  |
      | avg examples per work unit  | 3.0   |

  @validation
  @error-handling
  Scenario: Attempt to add example mapping to non-existent work unit
    Given I have a project with spec directory
    And no work unit "AUTH-999" exists
    When I run "fspec add-rule AUTH-999 'Some rule'"
    Then the command should fail
    And the error should contain "Work unit 'AUTH-999' does not exist"
