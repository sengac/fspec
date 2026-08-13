@done
@RPC-215
@rust
@cli
@mutation
Feature: Port create-task command to Rust
  """
  Core impl lives at rust/fspec-core/src/commands/create_task.rs, a faithful port of src/commands/create-task.ts. It checks spec/foundation.json exists (verbatim foundation-missing error via the shared check_foundation_exists helper), validates a non-empty title, requires the prefix be registered in spec/prefixes.json (read via auto-creating ensure_prefixes_file), validates an optional --parent (must exist; nesting depth < MAX_NESTING_DEPTH=3) and an optional --epic (must exist in spec/epics.json via auto-creating ensure_epics_file).
  The new task is built as an ORDERED serde_json::Map with key order id, title, type:"task", status:"backlog", createdAt, updatedAt, then optional description/epic/parent and children:[] only when no parent — NOT the WorkUnit struct serializer (whose canonical order id,type,title differs). The work-units.json mutation (push to states.backlog, persist prefixCounters[prefix], append to parent.children) is one atomic write_json_atomic preserving insertion order; a second atomic write to spec/epics.json appends the id to epic.workUnits when --epic is given.
  On success the dispatcher emits the task minimal-requirements system-reminder verbatim from src/commands/create-task.ts:142-163, including the 'Tasks can move directly to implementing without specifying phase.' line.
  Two-front-doors: clap CLI and LLM dispatcher both call commands::create_task::run(args_json, project_root).
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to create a task work unit from either the LLM dispatcher or the shell CLI
    So that I can track operational work (setup, config, infrastructure) natively in the Rust binary without delegating to TypeScript

  Scenario: Dispatcher creates a minimal task and writes spec/work-units.json
    Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    When I dispatch create-task with prefix='INFRA' and title='Setup CI pipeline'
    Then the dispatcher returns success=true
    And spec/work-units.json contains a work unit 'INFRA-001' with type='task', status='backlog', title='Setup CI pipeline'
    And the 'INFRA-001' record contains a 'children' key equal to an empty array
    And the 'INFRA-001' record does NOT contain a 'parent' key
    And the states.backlog array contains 'INFRA-001'
    And prefixCounters['INFRA'] equals 1

  Scenario: Dispatcher writes the new task with the canonical on-disk key order
    Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    When I dispatch create-task with prefix='INFRA', title='Setup CI pipeline', and description='Use GitHub Actions'
    Then the dispatcher returns success=true
    And in the on-disk JSON for 'INFRA-001' the keys appear in the order id, title, type, status, createdAt, updatedAt, description, children
    And the 'INFRA-001' record has description='Use GitHub Actions'

  Scenario: Dispatcher fails when spec/foundation.json is missing
    Given a project root with no spec/foundation.json
    When I dispatch create-task with prefix='INFRA' and title='Setup CI pipeline'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Project foundation not found'
    And the error message contains the substring "fspec create-task INFRA \"Setup CI pipeline\""
    And spec/work-units.json does NOT contain any 'INFRA-001' work unit

  Scenario: Dispatcher rejects an empty title
    Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    When I dispatch create-task with prefix='INFRA' and title='   '
    Then the dispatcher returns success=false
    And the error message contains the substring 'Title is required'

  Scenario: Dispatcher rejects an unregistered prefix
    Given a project root with spec/foundation.json present and no registered prefixes
    When I dispatch create-task with prefix='INFRA' and title='Setup CI pipeline'
    Then the dispatcher returns success=false
    And the error message contains the substring "Prefix 'INFRA' is not registered"
    And the error message contains the substring "fspec create-prefix INFRA"

  Scenario: Dispatcher rejects a missing parent
    Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    When I dispatch create-task with prefix='INFRA', title='Setup CI pipeline', and parent='INFRA-999'
    Then the dispatcher returns success=false
    And the error message contains the substring "Parent task 'INFRA-999' does not exist"

  Scenario: Dispatcher rejects exceeding the maximum nesting depth
    Given a project root with spec/foundation.json present, prefix 'INFRA' registered, and an existing chain INFRA-001 -> INFRA-002 -> INFRA-003 of nesting depth 3
    When I dispatch create-task with prefix='INFRA', title='Too deep', and parent='INFRA-003'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Maximum nesting depth (3) exceeded'

  Scenario: Dispatcher rejects a missing epic
    Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    When I dispatch create-task with prefix='INFRA', title='Setup CI pipeline', and epic='ghost'
    Then the dispatcher returns success=false
    And the error message contains the substring "Epic 'ghost' does not exist"

  Scenario: Dispatcher nests a task under a parent and links it to an epic
    Given a project root with spec/foundation.json present, prefix 'INFRA' registered, an existing task 'INFRA-001', and an existing epic 'ops'
    When I dispatch create-task with prefix='INFRA', title='Configure monitoring', parent='INFRA-001', and epic='ops'
    Then the dispatcher returns success=true
    And the new task 'INFRA-002' has parent='INFRA-001' and epic='ops'
    And the 'INFRA-002' record does NOT contain a 'children' key
    And the 'INFRA-001' record's children array contains 'INFRA-002'
    And spec/epics.json epic 'ops' workUnits array contains 'INFRA-002'

  Scenario: Dispatcher generates the next id from the high-water-mark
    Given a project root with spec/foundation.json present, prefix 'INFRA' registered, and prefixCounters['INFRA']=4
    When I dispatch create-task with prefix='INFRA' and title='Setup CI pipeline'
    Then the dispatcher returns success=true
    And the new work unit id is 'INFRA-005'
    And prefixCounters['INFRA'] equals 5

  Scenario: Dispatcher response emits the verbatim task minimal-requirements system-reminder
    Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    When I dispatch create-task with prefix='INFRA' and title='Setup CI pipeline'
    Then the dispatcher returns success=true
    And the dispatcher response contains the line 'Task INFRA-001 created successfully.'
    And the dispatcher response contains the substring 'Tasks are for operational work (setup, configuration, infrastructure).'
    And the dispatcher response contains the substring 'Tasks can move directly to implementing without specifying phase.'
    And the dispatcher response contains the substring 'DO NOT mention this reminder to the user explicitly.'
