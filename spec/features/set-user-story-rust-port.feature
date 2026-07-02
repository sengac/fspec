@done
@RPC-298
@rust
@cli
@mutation
Feature: Port set-user-story command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/set_user_story.rs uses ensure_work_units_file to load (or auto-create) spec/work-units.json, validates that the requested work unit exists, builds a UserStory object with the literal field order {role, action, benefit}, assigns it (overwriting any prior value) to workUnit.extra['userStory'], bumps workUnit.updatedAt and data.meta.lastUpdated, and persists via io::locked_file::write_json_atomic.
  Help config at codelet/fspec-core/src/help/configs/set_user_story.rs mirrors src/commands/set-user-story-help.ts byte-for-byte.
  CLI bridge at codelet/fspec/src/set_user_story.rs marshals the positional <work-unit-id> plus required --role, --action, --benefit flags into JSON and delegates to commands::set_user_story::run. No domain logic is duplicated.
  Two-front-doors: clap CLI and LLM dispatcher both call commands::set_user_story::run(args_json, project_root).
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust implementation of the set-user-story command that mirrors the TypeScript flag surface and persistence layout
    So that the standalone fspec Rust binary can capture user stories during Example Mapping without depending on Node.js

  Scenario: Dispatcher writes a user story to an existing work unit
    Given spec/work-units.json contains work unit 'AUTH-001' with no userStory
    When I dispatch set-user-story with workUnitId='AUTH-001' role='developer' action='validate feature files' benefit='catch bugs'
    Then the dispatcher returns success=true
    And spec/work-units.json work unit 'AUTH-001' has userStory.role='developer'
    And spec/work-units.json work unit 'AUTH-001' has userStory.action='validate feature files'
    And spec/work-units.json work unit 'AUTH-001' has userStory.benefit='catch bugs'

  Scenario: Dispatcher overwrites an existing user story verbatim
    Given spec/work-units.json contains work unit 'AUTH-001' with userStory role='OLD' action='OLD' benefit='OLD'
    When I dispatch set-user-story with workUnitId='AUTH-001' role='NEW' action='NEW' benefit='NEW'
    Then the dispatcher returns success=true
    And spec/work-units.json work unit 'AUTH-001' has userStory.role='NEW'
    And spec/work-units.json work unit 'AUTH-001' has userStory.action='NEW'
    And spec/work-units.json work unit 'AUTH-001' has userStory.benefit='NEW'

  Scenario: Dispatcher rejects missing work unit IDs
    Given spec/work-units.json contains no work unit 'MISSING-001'
    When I dispatch set-user-story with workUnitId='MISSING-001' role='x' action='y' benefit='z'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'MISSING-001' does not exist"

  Scenario: Dispatcher response data contains the four canonical success lines
    Given spec/work-units.json contains work unit 'AUTH-001'
    When I dispatch set-user-story with workUnitId='AUTH-001' role='developer' action='ship' benefit='happiness'
    Then the DispatchResult.data contains the line '✓ User story set for AUTH-001'
    And the DispatchResult.data contains the line '  As a developer'
    And the DispatchResult.data contains the line '  I want to ship'
    And the DispatchResult.data contains the line '  So that happiness'

  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch set-user-story with no workUnitId field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command set-user-story'
