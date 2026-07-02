@done
@feature-management
@cli
@RPC-192
Feature: Port add-step command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_step.rs: run(args_json,&Path) args {feature, scenario, type (serde rename), text, dryRun?}. Validation + scenario lookup via crate::io::gherkin::parse_feature_lenient; uses scenario.position.line, step.position.line, step.value (==TS step.text), step.keyword. Indentation from first step line leading whitespace. Placeholder map given/when/then. Line-based replace-or-append using positions, mirroring TS exactly (table/docstring before-insert). Response {success, valid} + type/scenario for CLI msg. Two-front-doors: bridge marshals <feature> <scenario> <type> <text> + optional --dry-run into JSON only.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Step type MUST be one of given/when/then/and/but (case-insensitive); anything else fails with error='Invalid step type: "<input>"' and suggestion='Valid step types are: given, when, then, and, but'
  #   2. The emitted keyword MUST be the capitalized normalized type (Given/When/Then/And/But)
  #   3. Feature identifier resolves the same way as add-scenario; a missing file fails with 'Feature file not found: <path>' and a create-feature suggestion
  #   4. A scenario name not present in the file fails with error='Scenario not found: "<name>"' and suggestion='Available scenarios: <comma-list or none>'
  #   5. Step indentation MUST be inherited from the first existing step line in the scenario (leading whitespace), defaulting to four spaces when the scenario has no steps
  #   6. If the scenario contains a matching placeholder step (given→[precondition], when→[action], then→[expected outcome]) the new step MUST REPLACE that placeholder line in place rather than appending
  #   7. When no placeholder matches, the new '<indent><Keyword> <text>' line MUST be appended after the last step; if the last step is followed by a data table (|) or doc string (""") the new step is inserted BEFORE that table/docstring
  #   8. The result MUST be re-parsed to set a 'valid' boolean, the file is always written (unless dryRun), and success=true returned
  #
  # EXAMPLES:
  #   1. adding a 'given' step with text 'I am on the login page' to a scenario whose Given line is the '[precondition]' placeholder REPLACES the placeholder with '    Given I am on the login page'
  #   2. adding an 'and' step to a scenario that already has real Given/When/Then steps appends '    And <text>' after the last step
  #   3. adding a step with type 'maybe' fails with 'Invalid step type: "maybe"' and lists the valid step types
  #   4. running 'fspec add-step spec/features/login.feature "Login" given "I am on the login page"' prints '✓ Added given step to scenario "Login"' and exits 0
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to add a typed step (given/when/then/and/but) to a named scenario in a feature file
    So that I can flesh out scenario steps incrementally with correct indentation and placeholder replacement, without depending on Node.js

  Scenario: Replaces a matching placeholder step in place
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given [precondition]\n    When [action]\n    Then [expected outcome]\n'
    When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='given' and text='I am on the login page'
    Then the dispatcher returns success=true and valid=true
    And the file on disk contains the line '    Given I am on the login page'
    And the file on disk does NOT contain the line '    Given [precondition]'

  Scenario: Appends a new step after the last existing step
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given I am logged in\n    When I click\n    Then I see it\n'
    When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='and' and text='I am happy'
    Then the dispatcher returns success=true
    And the file on disk contains the line '    And I am happy' after the line '    Then I see it'

  Scenario: Indentation is inherited from existing steps
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n      Given deeply indented\n'
    When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='and' and text='also deep'
    Then the dispatcher returns success=true
    And the file on disk contains the line '      And also deep'

  Scenario: Invalid step type is rejected
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='maybe' and text='whatever'
    Then the dispatcher returns success=false
    And the error equals 'Invalid step type: "maybe"'
    And the suggestion equals 'Valid step types are: given, when, then, and, but'

  Scenario: Unknown scenario name is rejected with available list
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    When I dispatch add-step with feature='spec/features/login.feature', scenario='Nope', type='given' and text='x'
    Then the dispatcher returns success=false
    And the error equals 'Scenario not found: "Nope"'
    And the suggestion contains 'Available scenarios: Login'

  Scenario: Missing feature file surfaces the not-found error
    Given a project root tempdir with NO spec/features/missing.feature file
    When I dispatch add-step with feature='spec/features/missing.feature', scenario='Login', type='given' and text='x'
    Then the dispatcher returns success=false
    And the error contains 'Feature file not found: '
