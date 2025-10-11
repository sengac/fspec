@exmap-001
@phase1
@critical
@cli
@project-management
@example-mapping
@bdd
@discovery
Feature: Example Mapping for BDD Discovery
  """
  Architecture notes:
  - Example Mapping is a BDD discovery technique from Cucumber
  - Used BEFORE writing Gherkin scenarios to explore requirements
  - Happens during "specifying" state of work unit (yellow card = work unit)
  - Uses 4-card system: Yellow (story/work unit), Blue (rules), Green (examples), Red (questions)
  - After discovery complete, examples generate scenarios in feature file

  Critical implementation requirements:
  - MUST work on work units in "specifying" state only
  - MUST store rules[], examples[], questions[], assumptions[] in work unit
  - MUST prevent state transition from specifying->testing if questions unanswered
  - MUST generate feature file from examples (examples become scenarios)
  - MUST tag generated scenarios with @work-unit-id
  - Commands: add-rule, add-example, add-question, answer-question
  - Commands operate on: workUnitId (not feature/scenario names!)

  4-Card System mapping to fspec:
  - Yellow Card (Story): The work unit itself (e.g., AUTH-001 "User Login")
  - Blue Cards (Rules): work unit's rules[] array - business rules
  - Green Cards (Examples): work unit's examples[] array - become scenarios
  - Red Cards (Questions): work unit's questions[] array - blockers

  Workflow:
  1. Create work unit: fspec create-work-unit AUTH "User Login"
  2. Move to specifying: fspec update-work-unit-status AUTH-001 specifying
  3. Add rules: fspec add-rule AUTH-001 "Password must be 8+ characters"
  4. Add examples: fspec add-example AUTH-001 "User logs in with valid password"
  5. Add questions: fspec add-question AUTH-001 "What about 2FA?"
  6. Answer questions: fspec answer-question AUTH-001 0 "2FA required for admins" --add-to=rules
  7. Generate feature: fspec generate-scenarios AUTH-001 --feature=user-login
  8. Move to testing: fspec update-work-unit-status AUTH-001 testing

  References:
  - Example Mapping: https://cucumber.io/blog/bdd/example-mapping-introduction/
  - Integrates with ACDD workflow: specifying state = discovery phase
  """

  Background: User Story
    As an AI agent exploring requirements
    I want to use Example Mapping before writing feature files
    So that I discover concrete examples, rules, and questions that will become scenarios

  @critical
  @happy-path
  Scenario: Complete Example Mapping session then generate feature file
    Given I have a work unit "AUTH-001" with title "User Login"
    And the work unit is in "specifying" state
    When I run "fspec add-rule AUTH-001 'Password must be 8+ characters'"
    And I run "fspec add-rule AUTH-001 'Session expires after 1 hour'"
    And I run "fspec add-example AUTH-001 'User logs in with valid password'"
    And I run "fspec add-example AUTH-001 'User logs in with expired session'"
    And I run "fspec add-example AUTH-001 'User enters wrong password'"
    And I run "fspec add-question AUTH-001 'What about 2FA?'"
    And I run "fspec answer-question AUTH-001 0 '2FA required for admin users' --add-to=rules"
    Then the work unit should have 3 rules
    And the work unit should have 3 examples
    And the work unit should have 0 questions
    When I run "fspec generate-scenarios AUTH-001 --feature=user-login"
    Then a feature file "spec/features/user-login.feature" should be created
    And the feature should have 3 scenarios
    And each scenario should be tagged with "@AUTH-001"
    And scenario titles should match example descriptions

  @happy-path
  Scenario: Add rule to work unit during discovery
    Given I have a work unit "AUTH-001" in "specifying" state
    When I run "fspec add-rule AUTH-001 'Users must authenticate before accessing protected resources'"
    Then the command should succeed
    And the work unit should have 1 rule
    And the rule should be "Users must authenticate before accessing protected resources"

  @happy-path
  Scenario: Add multiple examples that will become scenarios
    Given I have a work unit "AUTH-001" in "specifying" state
    When I run "fspec add-example AUTH-001 'User logs in with valid credentials'"
    And I run "fspec add-example AUTH-001 'User logs in with expired token'"
    And I run "fspec add-example AUTH-001 'User token auto-refreshes'"
    Then the work unit should have 3 examples

  @happy-path
  Scenario: Add question that needs answering
    Given I have a work unit "AUTH-001" in "specifying" state
    When I run "fspec add-question AUTH-001 'Should we support GitHub Enterprise?'"
    Then the command should succeed
    And the work unit should have 1 question

  @happy-path
  Scenario: Answer question and convert to rule
    Given I have a work unit "AUTH-001" in "specifying" state
    And the work unit has questions:
      | @bob: What is the token expiry policy? |
    When I run "fspec answer-question AUTH-001 0 'Tokens expire after 24 hours' --add-to=rules"
    Then the command should succeed
    And the work unit should have 0 questions
    And the work unit should have 1 rule
    And the rule should contain "24 hours"

  @validation
  @blocking
  Scenario: Prevent moving to testing when questions remain unanswered
    Given I have a work unit "AUTH-001" in "specifying" state
    And the work unit has unanswered questions:
      | Should we support OAuth 2.0? |
      | What about rate limiting?    |
    When I run "fspec update-work-unit-status AUTH-001 testing"
    Then the command should fail
    And the error should contain "Cannot move to testing with unanswered questions"
    And the error should list both questions
    And the error should suggest using "fspec answer-question"

  @validation
  @error-handling
  Scenario: Cannot add example mapping data to non-specifying work unit
    Given I have a work unit "AUTH-001" in "implementing" state
    When I run "fspec add-example AUTH-001 'Some example'"
    Then the command should fail
    And the error should contain "Can only add examples during specifying state"
    And the error should show current state is "implementing"

  @validation
  Scenario: Cannot generate scenarios without examples
    Given I have a work unit "AUTH-001" in "specifying" state
    And the work unit has 0 examples
    When I run "fspec generate-scenarios AUTH-001"
    Then the command should fail
    And the error should contain "No examples to generate scenarios from"

  @remove
  @happy-path
  Scenario: Remove example by index
    Given I have a work unit "AUTH-001" in "specifying" state
    And the work unit has examples:
      | User logs in with Google        |
      | User logs in with expired token |
      | User logs out                   |
    When I run "fspec remove-example AUTH-001 1"
    Then the command should succeed
    And the work unit should have 2 examples
    And the remaining examples should be "User logs in with Google" and "User logs out"

  @generate-scenarios
  @critical
  Scenario: Generate feature file from example mapping
    Given I have a work unit "AUTH-001" in "specifying" state
    And the work unit has rules:
      | Password must be 8+ characters |
      | Session expires after 1 hour   |
    And the work unit has examples:
      | User logs in with valid password |
      | User logs in with expired token  |
    When I run "fspec generate-scenarios AUTH-001 --feature=authentication"
    Then a feature file "spec/features/authentication.feature" should be created
    And the feature should have title derived from work unit
    And the feature should have Background with rules
    And the feature should have 2 scenarios from examples
    And each scenario should be tagged "@AUTH-001"
    And each scenario should have Given/When/Then placeholders

  @bulk-operations
  Scenario: Import example mapping from collaborative session
    Given I have a work unit "AUTH-001" in "specifying" state
    And I have a JSON file "auth-example-map.json":
      """
      {
        "rules": ["Password must be 8+ characters", "Session timeout is 1 hour"],
        "examples": ["Valid login", "Expired session", "Wrong password"],
        "questions": ["Support 2FA?", "Support SSO?"]
      }
      """
    When I run "fspec import-example-map AUTH-001 auth-example-map.json"
    Then the work unit should have 2 rules
    And the work unit should have 3 examples
    And the work unit should have 2 questions

  @query
  Scenario: Show work unit with example mapping data
    Given I have a work unit "AUTH-001" with example mapping:
      | type     | content                          |
      | rule     | Passwords must be 8+ characters  |
      | rule     | Sessions expire after 1 hour     |
      | example  | User logs in with valid password |
      | example  | User logs in with expired token  |
      | question | Support GitHub Enterprise?       |
    When I run "fspec show-work-unit AUTH-001"
    Then the output should display all rules
    And the output should display all examples
    And the output should display all questions
    And the output should be organized by card type

  @query
  @filtering
  Scenario: Find all work units with unanswered questions
    Given I have work units:
      | id       | status     | questions   |
      | AUTH-001 | specifying | 2 questions |
      | AUTH-002 | specifying | 0 questions |
      | DASH-001 | specifying | 1 question  |
    When I run "fspec query-work-units --has-questions"
    Then the output should show AUTH-001 and DASH-001
    And the output should show question count for each

  @statistics
  Scenario: Show example mapping completeness metrics
    Given I have work units in specifying state:
      | id       | rules | examples | questions |
      | AUTH-001 | 3     | 5        | 0         |
      | AUTH-002 | 0     | 0        | 3         |
      | DASH-001 | 2     | 4        | 1         |
    When I run "fspec query-example-mapping-stats"
    Then the output should show:
      | metric                       | value |
      | work units with rules        | 2     |
      | work units with examples     | 2     |
      | work units with questions    | 2     |
      | avg examples per work unit   | 3.0   |
      | work units ready to generate | 1     |
