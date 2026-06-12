@done
@RPC-210
@rust
@cli
@mutation
Feature: Port create-bug command to Rust

  """
  Core impl lives at codelet/fspec-core/src/commands/create_bug.rs, a faithful port of src/commands/create-bug.ts. It checks spec/foundation.json exists (verbatim foundation-missing error via the shared check_foundation_exists helper), validates a non-empty title, requires the prefix be registered in spec/prefixes.json (read via auto-creating ensure_prefixes_file), validates an optional --parent (must exist; nesting depth < MAX_NESTING_DEPTH=3) and an optional --epic (must exist in spec/epics.json via auto-creating ensure_epics_file).
  The new bug is built as an ORDERED serde_json::Map with key order id, title, type:"bug", status:"backlog", createdAt, updatedAt, then optional description/epic/parent and children:[] only when no parent — NOT the WorkUnit struct serializer (whose canonical order id,type,title differs). The work-units.json mutation (push to states.backlog, persist prefixCounters[prefix], append to parent.children) is one atomic write_json_atomic preserving insertion order; a second atomic write to spec/epics.json appends the id to epic.workUnits when --epic is given.
  On success the dispatcher emits the bug research-guidance system-reminder verbatim from src/commands/create-bug.ts:142-168, including a search-scenarios suggestion derived from the first two lowercased title words.
  Two-front-doors: clap CLI and LLM dispatcher both call commands::create_bug::run(args_json, project_root).
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to create a bug work unit from either the LLM dispatcher or the shell CLI
    So that I can track defects natively in the Rust binary without delegating to TypeScript

  Scenario: Dispatcher creates a minimal bug and writes spec/work-units.json
    Given a project root with spec/foundation.json present and prefix 'BUG' registered
    When I dispatch create-bug with prefix='BUG' and title='Login crash'
    Then the dispatcher returns success=true
    And spec/work-units.json contains a work unit 'BUG-001' with type='bug', status='backlog', title='Login crash'
    And the 'BUG-001' record contains a 'children' key equal to an empty array
    And the 'BUG-001' record does NOT contain a 'parent' key
    And the states.backlog array contains 'BUG-001'
    And prefixCounters['BUG'] equals 1

  Scenario: Dispatcher writes the new bug with the canonical on-disk key order
    Given a project root with spec/foundation.json present and prefix 'BUG' registered
    When I dispatch create-bug with prefix='BUG', title='Login crash', and description='Crashes on submit'
    Then the dispatcher returns success=true
    And in the on-disk JSON for 'BUG-001' the keys appear in the order id, title, type, status, createdAt, updatedAt, description, children
    And the 'BUG-001' record has description='Crashes on submit'

  Scenario: Dispatcher fails when spec/foundation.json is missing
    Given a project root with no spec/foundation.json
    When I dispatch create-bug with prefix='BUG' and title='Login crash'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Project foundation not found'
    And the error message contains the substring "fspec create-bug BUG \"Login crash\""
    And spec/work-units.json does NOT contain any 'BUG-001' work unit

  Scenario: Dispatcher rejects an empty title
    Given a project root with spec/foundation.json present and prefix 'BUG' registered
    When I dispatch create-bug with prefix='BUG' and title='   '
    Then the dispatcher returns success=false
    And the error message contains the substring 'Title is required'

  Scenario: Dispatcher rejects an unregistered prefix
    Given a project root with spec/foundation.json present and no registered prefixes
    When I dispatch create-bug with prefix='BUG' and title='Login crash'
    Then the dispatcher returns success=false
    And the error message contains the substring "Prefix 'BUG' is not registered"
    And the error message contains the substring "fspec create-prefix BUG"

  Scenario: Dispatcher rejects a missing parent
    Given a project root with spec/foundation.json present and prefix 'BUG' registered
    When I dispatch create-bug with prefix='BUG', title='Login crash', and parent='BUG-999'
    Then the dispatcher returns success=false
    And the error message contains the substring "Parent bug 'BUG-999' does not exist"

  Scenario: Dispatcher rejects exceeding the maximum nesting depth
    Given a project root with spec/foundation.json present, prefix 'BUG' registered, and an existing chain BUG-001 -> BUG-002 -> BUG-003 of nesting depth 3
    When I dispatch create-bug with prefix='BUG', title='Too deep', and parent='BUG-003'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Maximum nesting depth (3) exceeded'

  Scenario: Dispatcher rejects a missing epic
    Given a project root with spec/foundation.json present and prefix 'BUG' registered
    When I dispatch create-bug with prefix='BUG', title='Login crash', and epic='ghost'
    Then the dispatcher returns success=false
    And the error message contains the substring "Epic 'ghost' does not exist"

  Scenario: Dispatcher nests a bug under a parent and links it to an epic
    Given a project root with spec/foundation.json present, prefix 'BUG' registered, an existing bug 'BUG-001', and an existing epic 'auth'
    When I dispatch create-bug with prefix='BUG', title='Login crash', parent='BUG-001', and epic='auth'
    Then the dispatcher returns success=true
    And the new bug 'BUG-002' has parent='BUG-001' and epic='auth'
    And the 'BUG-002' record does NOT contain a 'children' key
    And the 'BUG-001' record's children array contains 'BUG-002'
    And spec/epics.json epic 'auth' workUnits array contains 'BUG-002'

  Scenario: Dispatcher generates the next id from the high-water-mark
    Given a project root with spec/foundation.json present, prefix 'BUG' registered, and prefixCounters['BUG']=7
    When I dispatch create-bug with prefix='BUG' and title='Login crash'
    Then the dispatcher returns success=true
    And the new work unit id is 'BUG-008'
    And prefixCounters['BUG'] equals 8

  Scenario: Dispatcher response emits the verbatim bug research-guidance system-reminder
    Given a project root with spec/foundation.json present and prefix 'BUG' registered
    When I dispatch create-bug with prefix='BUG' and title='Login Crash'
    Then the dispatcher returns success=true
    And the dispatcher response contains the line 'Bug BUG-001 created successfully.'
    And the dispatcher response contains the substring 'CRITICAL: Research existing code FIRST before fixing bugs.'
    And the dispatcher response contains the substring 'search-scenarios --query="login crash"'
    And the dispatcher response contains the substring 'DO NOT mention this reminder to the user explicitly.'
