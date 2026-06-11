@done
@RPC-181
Feature: Port add-example command to Rust

  """
  Core impl file: codelet/fspec-core/src/commands/add_example.rs — replaces the NotYetPorted stub.
  Public signature `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` (parity with list_prefixes::run).
  Reuses shared infrastructure: io::ensure::ensure_work_units_file (load-or-init),
  io::locked_file::write_json_atomic (atomic write), io::time::iso8601_now.
  ExampleItem is stored inside WorkUnit.extra (since WorkUnit uses #[serde(flatten)]),
  with field insertion order id, text, deleted, createdAt — matching TS object-literal order.
  Two-front-doors: dispatcher and clap CLI both call commands::add_example::run(args_json, project_root).
  Bridge marshals clap positional args into JSON object {workUnitId, example}. NO domain logic in the bridge.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want a Rust port of `add-example` callable from both dispatchers
    So that the standalone fspec binary can capture Example Mapping examples with the same parity as TypeScript

  Scenario: First example appended with stable id 0 and nextExampleId bumped to 1
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with no examples array
    When I dispatch add-example with workUnitId='AUTH-001' and example='User logs in with valid credentials'
    Then the dispatcher returns success
    And spec/work-units.json on disk shows AUTH-001.examples[0].id=0
    And spec/work-units.json on disk shows AUTH-001.examples[0].text='User logs in with valid credentials'
    And spec/work-units.json on disk shows AUTH-001.examples[0].deleted=false
    And spec/work-units.json on disk shows AUTH-001.examples[0].createdAt is a freshly bumped ISO-8601 timestamp
    And spec/work-units.json on disk shows AUTH-001.nextExampleId=1
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Second example reuses the incrementing counter
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with one existing example id=0 and nextExampleId=1
    When I dispatch add-example with workUnitId='AUTH-001' and example='User enters wrong password'
    Then the dispatcher returns success
    And spec/work-units.json on disk shows AUTH-001.examples has length 2
    And spec/work-units.json on disk shows AUTH-001.examples[1].id=1
    And spec/work-units.json on disk shows AUTH-001.examples[1].text='User enters wrong password'
    And spec/work-units.json on disk shows AUTH-001.nextExampleId=2

  Scenario: Status guard rejects add-example when work unit is not in specifying
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I dispatch add-example with workUnitId='AUTH-001' and example='Anything'
    Then the dispatcher returns an error
    And the error message contains the substring "Can only add examples during discovery/specification phase. AUTH-001 is in 'backlog' state."
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing no NOPE-001 entry
    When I dispatch add-example with workUnitId='NOPE-001' and example='Anything'
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"

  Scenario: Auto-creates spec/work-units.json then reports missing-source error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-example with workUnitId='AUTH-001' and example='Anything'
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json now exists on disk with the canonical empty initial structure

  Scenario: Success rendering embeds the system reminder block
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with userStory.role='developer'
    When I dispatch add-example with workUnitId='AUTH-001' and example='Valid login'
    Then the dispatcher returns success
    And the rendered output starts with "✓ Example added successfully"
    And the rendered output contains the substring "<system-reminder>"
    And the rendered output contains the substring "User story: \"As a developer...\""
    And the rendered output contains the substring "Example: \"Valid login\""
    And the rendered output contains the substring "</system-reminder>"

  Scenario: System reminder falls back to 'the user' when userStory.role is absent
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with no userStory
    When I dispatch add-example with workUnitId='AUTH-001' and example='Valid login'
    Then the dispatcher returns success
    And the rendered output contains the substring "User story: \"As a the user...\""
