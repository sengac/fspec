@done
@RPC-315
@wip
@file-ops
@feature-management
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

  Scenario: Update step in specific scenario when multiple scenarios have same step
    Given a feature file "spec/features/test.feature" with scenarios "First" and "Second" that both contain step "Given I am logged in"
    When I dispatch update-step with feature "test" scenario "First" current-step "Given I am logged in" and text "Given I have authenticated"
    Then the response has success true
    And the step in "First" scenario should be updated
    And the step in "Second" scenario should remain unchanged

  Scenario: Update step preserves position in scenario
    Given a feature file "spec/features/test.feature" with a scenario whose steps are in order: "Given A", "When B", "Then C"
    When I dispatch update-step with feature "test" scenario "Test scenario" current-step "When B" and text "When B updated"
    Then the response has success true
    And the steps should remain in order: "Given A", "When B updated", "Then C"

  Scenario: Update step with special characters
    Given a feature file "spec/features/test.feature" with scenario "Test scenario" containing step "Given I can login"
    When I dispatch update-step with feature "test" scenario "Test scenario" current-step "Given I can login" and text "Given I can't login with @invalid credentials"
    Then the response has success true
    And the feature file step line reads "    Given I can't login with @invalid credentials"
    And the feature file should be valid Gherkin

  Scenario: Update And step to Given step
    Given a feature file "spec/features/test.feature" with scenario "Test scenario" containing step "And I enter password"
    When I dispatch update-step with feature "test" scenario "Test scenario" current-step "And I enter password" and keyword "Given"
    Then the response has success true
    And the feature file step line reads "    Given I enter password"

  Scenario: Update step preserves indentation
    Given a feature file "spec/features/test.feature" with scenario "Test scenario" whose step "Given I am on page" is indented by 6 spaces
    When I dispatch update-step with feature "test" scenario "Test scenario" current-step "Given I am on page" and text "Given I am on the login page"
    Then the response has success true
    And the feature file step line reads "      Given I am on the login page"

  Scenario: Handle feature file with invalid syntax
    Given a feature file "spec/features/broken.feature" with Gherkin syntax errors
    When I dispatch update-step with feature "broken" scenario "Some scenario" current-step "Given test" and text "New text"
    Then the response has success false
    And the response error contains "Invalid Gherkin syntax"
    And the file should remain unchanged
