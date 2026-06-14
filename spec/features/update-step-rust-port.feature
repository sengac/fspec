@done
@RPC-315 @wip @file-ops @feature-management
Feature: Port update-step command to Rust

  """
  Single source of truth: pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>; both dispatcher and CLI bridge converge here (RPC-003 two-front-doors)
  Use parse_feature_lenient (io/gherkin.rs); line-based split('\n')/join('\n') edit; recoverable errors as inner JSON envelope {success:false,error} like list_scenario_tags.rs; gherkin Step.keyword includes trailing space, handle trimming when matching
  """

  Background: User Story
    As a fspec developer
    I want to update a step's text and/or keyword in a scenario via the Rust port of update-step
    So that steps can be refined without manual editing, behaviour-identical to the TypeScript command

  Scenario: Update step text while keeping the keyword
    Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Given I am on the login page"
    When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "Given I am on the login page" and text "I navigate to the login page"
    Then the response has success true
    And the response message is "Successfully updated step in scenario 'Valid login' in user-auth.feature"
    And the feature file step line reads "    Given I navigate to the login page"

  Scenario: Change a step keyword while keeping the text
    Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Given I am logged out"
    When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "Given I am logged out" and keyword "When"
    Then the response has success true
    And the feature file step line reads "    When I am logged out"

  Scenario: Update both text and keyword where text carries a keyword prefix
    Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "When I enter credentials"
    When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "I enter credentials" and text "When I submit the login form" and keyword "When"
    Then the response has success true
    And the feature file step line reads "    When I submit the login form"

  Scenario: Match a step by its text alone without the keyword
    Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Then I should see the dashboard"
    When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "I should see the dashboard" and text "I land on the dashboard"
    Then the response has success true
    And the feature file step line reads "    Then I land on the dashboard"

  Scenario: Supplying neither text nor keyword fails without modifying the file
    Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Given I am on the login page"
    When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "Given I am on the login page" and no text and no keyword
    Then the response has success false
    And the response error is "No updates specified. Use --text and/or --keyword"

  Scenario: Updating a step in a missing feature file fails
    Given no feature file exists at "spec/features/missing.feature"
    When I dispatch update-step with feature "spec/features/missing.feature" scenario "S" current-step "Given x" and text "Given y"
    Then the response has success false
    And the response error contains "Feature file not found:"

  Scenario: Updating a step in an absent scenario fails
    Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Given I am on the login page"
    When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Nonexistent" current-step "Given x" and text "Given y"
    Then the response has success false
    And the response error is "Scenario 'Nonexistent' not found in feature file"

  Scenario: Updating a step that does not match fails
    Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Given I am on the login page"
    When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "Given I do not exist" and text "Given y"
    Then the response has success false
    And the response error is "Step 'Given I do not exist' not found in scenario 'Valid login'"
