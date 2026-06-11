@done
@RPC-194
Feature: Port add-tag-to-scenario command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/add_tag_to_scenario.rs. Reads the target feature
  file (relative to project_root), validates each tag format (work-unit pattern @[A-Z]{2,6}-[0-9]+
  OR regular pattern @lowercase-with-hyphens), parses with parse_feature_lenient to find the named
  top-level Scenario, optionally enforces the spec/tags.json registry, then mutates the raw
  source lines (NOT a re-emit) to insert the new tag lines immediately above the Scenario line
  while preserving indentation. New shared module gherkin_tags.rs supplies is_work_unit_tag and
  is_regular_tag. Direct fs::write — no locking, parity with TS writeFile. Two-front-doors:
  bridge marshals positional args + the validate-registry flag into JSON object
  {file, scenario, tags, validateRegistry} and forwards to commands::add_tag_to_scenario::run.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `add-tag-to-scenario` command added as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both add tags to specific scenarios in feature files with TS-parity validation and error envelopes

  Scenario: Add single tag to a scenario that previously had no tags
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login with valid credentials' with no tags
    When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login with valid credentials' tags=['@smoke']
    Then the dispatcher returns success=true
    And the returned data contains valid=true
    And the returned data contains message="Added @smoke to scenario 'Login with valid credentials'"
    And the file on disk shows a single '  @smoke' line immediately above the Scenario line
    And the file on disk still parses as valid Gherkin

  Scenario: Append a new tag after an existing tag block
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke
    When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@critical']
    Then the dispatcher returns success=true
    And the file on disk shows '  @smoke' followed by '  @critical' immediately above the Scenario line
    And no other lines in the file are mutated

  Scenario: Multiple tags inserted in argument order
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@critical','@regression']
    Then the dispatcher returns success=true
    And the file on disk shows '  @critical' followed by '  @regression' above the Scenario line
    And the returned data contains message="Added @critical, @regression to scenario 'Login'"

  Scenario: Duplicate tag is rejected
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke
    When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@smoke']
    Then the dispatcher returns success=false
    And the error message contains the substring 'Tag @smoke already exists on this scenario'
    And the file on disk is byte-equal to its pre-call contents

  Scenario: Tag without leading @ is rejected
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['InvalidTag']
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid tag format. Tags must start with @'
    And the file on disk is byte-equal to its pre-call contents

  Scenario: Mixed-case regular tag is rejected
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@CamelCase']
    Then the dispatcher returns success=false
    And the error message contains the substring 'Regular tags must use lowercase-with-hyphens'
    And the file on disk is byte-equal to its pre-call contents

  Scenario: Work-unit tag with uppercase is accepted
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@AUTH-001']
    Then the dispatcher returns success=true
    And the file on disk shows '  @AUTH-001' immediately above the Scenario line

  Scenario: Registry validation accepts a registered tag
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    And spec/tags.json registers @custom-tag under category 'Test Tags'
    When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@custom-tag'] validateRegistry=true
    Then the dispatcher returns success=true
    And the file on disk shows '  @custom-tag' immediately above the Scenario line

  Scenario: Registry validation rejects an unregistered tag
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    And spec/tags.json does NOT register @unregistered
    When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@unregistered'] validateRegistry=true
    Then the dispatcher returns success=false
    And the error message contains the substring '@unregistered is not registered in spec/tags.json'
    And the file on disk is byte-equal to its pre-call contents

  Scenario: Missing scenario name surfaces the canonical error
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Nonexistent' tags=['@smoke']
    Then the dispatcher returns success=false
    And the error message contains the substring "Scenario 'Nonexistent' not found in spec/features/login.feature"
    And the file on disk is byte-equal to its pre-call contents

  Scenario: Missing feature file surfaces the canonical error
    Given an empty project root directory with no spec/features/missing.feature
    When I dispatch add-tag-to-scenario with file='spec/features/missing.feature' scenario='Login' tags=['@smoke']
    Then the dispatcher returns success=false
    And the error message contains the substring 'File not found: spec/features/missing.feature'

  Scenario: Feature-level tag and other scenarios are preserved
    Given a project root tempdir with spec/features/auth.feature containing feature tag @authentication and two scenarios 'Login' (tagged @smoke) and 'Logout' (tagged @regression)
    When I dispatch add-tag-to-scenario with file='spec/features/auth.feature' scenario='Login' tags=['@critical']
    Then the dispatcher returns success=true
    And the file on disk still contains the feature-level tag @authentication
    And the file on disk shows the Login scenario tags as '@smoke' then '@critical'
    And the file on disk shows the Logout scenario tags as '@regression' unchanged
