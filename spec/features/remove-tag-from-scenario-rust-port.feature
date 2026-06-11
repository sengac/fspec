@done
@RPC-282
Feature: Port remove-tag-from-scenario command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/remove_tag_from_scenario.rs. Reads the target
  feature file (relative to project_root), parses with parse_feature_lenient to find the named
  top-level Scenario, intersects requested tags with the scenario's existing tags, then mutates
  the raw source lines (NOT a re-emit) to drop only the matching tag lines that sit between the
  previous Scenario/Feature boundary and the target Scenario line. Idempotent semantics: missing
  scenario or zero matching tags returns success without writing. Direct fs::write — no locking,
  parity with TS writeFile. Two-front-doors: bridge marshals positional args into JSON
  {file, scenario, tags} and forwards to commands::remove_tag_from_scenario::run.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `remove-tag-from-scenario` command added as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both remove tags from specific scenarios in feature files with TS-parity idempotent semantics

  Scenario: Remove a single tag from a multi-tag scenario
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical @regression
    When I dispatch remove-tag-from-scenario with file='spec/features/login.feature' scenario='Login' tags=['@critical']
    Then the dispatcher returns success=true
    And the returned data contains valid=true
    And the returned data contains message="Removed @critical from scenario 'Login'"
    And the file on disk shows the Login scenario tagged @smoke @regression
    And the file on disk still parses as valid Gherkin

  Scenario: Remove multiple tags in one call
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical @regression @wip
    When I dispatch remove-tag-from-scenario with file='spec/features/login.feature' scenario='Login' tags=['@critical','@wip']
    Then the dispatcher returns success=true
    And the file on disk shows the Login scenario tagged @smoke @regression
    And the returned data contains message="Removed @critical, @wip from scenario 'Login'"

  Scenario: Requested tag absent from scenario is idempotent success
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke
    When I dispatch remove-tag-from-scenario with file='spec/features/login.feature' scenario='Login' tags=['@critical']
    Then the dispatcher returns success=true
    And the returned data contains valid=true
    And the returned data contains message="No changes made - none of the specified tags found on scenario 'Login'"
    And the file on disk is byte-equal to its pre-call contents

  Scenario: Remove all tags from a scenario
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical
    When I dispatch remove-tag-from-scenario with file='spec/features/login.feature' scenario='Login' tags=['@smoke','@critical']
    Then the dispatcher returns success=true
    And the file on disk shows the Login scenario with no tag lines immediately above it

  Scenario: Missing scenario is idempotent success
    Given a project root tempdir with spec/features/login.feature containing only Scenario 'Login'
    When I dispatch remove-tag-from-scenario with file='spec/features/login.feature' scenario='Nonexistent' tags=['@smoke']
    Then the dispatcher returns success=true
    And the returned data contains valid=true
    And the returned data contains message="Scenario 'Nonexistent' not found in spec/features/login.feature - no changes made"
    And the file on disk is byte-equal to its pre-call contents

  Scenario: Missing feature file surfaces the canonical error
    Given an empty project root directory with no spec/features/missing.feature
    When I dispatch remove-tag-from-scenario with file='spec/features/missing.feature' scenario='Login' tags=['@smoke']
    Then the dispatcher returns success=false
    And the error message contains the substring 'File not found: spec/features/missing.feature'

  Scenario: Feature-level tags survive the mutation
    Given a project root tempdir with spec/features/auth.feature containing feature tag @authentication and Scenario 'Login' tagged @smoke @critical
    When I dispatch remove-tag-from-scenario with file='spec/features/auth.feature' scenario='Login' tags=['@smoke']
    Then the dispatcher returns success=true
    And the file on disk still contains the feature-level tag @authentication
    And the file on disk shows the Login scenario tagged @critical

  Scenario: Tags on a sibling scenario are untouched
    Given a project root tempdir with spec/features/auth.feature containing Scenario 'Login' tagged @smoke and Scenario 'Logout' tagged @smoke @regression
    When I dispatch remove-tag-from-scenario with file='spec/features/auth.feature' scenario='Login' tags=['@smoke']
    Then the dispatcher returns success=true
    And the file on disk shows the Login scenario with no tag lines immediately above it
    And the file on disk shows the Logout scenario tagged @smoke @regression
