@done
@mutation
@cli
@rust
@RPC-214
Feature: Port create-story command to Rust
  """
  Core impl at rust/fspec-core/src/commands/create_story.rs ports src/commands/create-story.ts. It (1)
  requires spec/foundation.json to exist (checkFoundationExists at src/commands/create-story.ts:38-42; when
  missing it throws the foundation-missing user message + <system-reminder>); (2) validates a non-empty title
  (src/commands/create-story.ts:45-47); (3) loads spec/prefixes.json via ensurePrefixesFile and rejects an
  unregistered prefix with "Prefix '<p>' is not registered. Run 'fspec create-prefix <p> \"Description\"'
  first." (src/commands/create-story.ts:50-57); (4) loads spec/work-units.json via ensureWorkUnitsFile;
  (5) optional --parent must exist else "Parent story '<p>' does not exist" and nesting depth must be
  < MAX_NESTING_DEPTH=3 else "Maximum nesting depth (3) exceeded" (src/commands/create-story.ts:62-73);
  (6) optional --epic via ensureEpicsFile must exist else "Epic '<e>' does not exist"
  (src/commands/create-story.ts:75-82).
  ID generation uses prefixCounters high-water-mark max(stored, max-existing-id-suffix)+1, formatted
  `<PREFIX>-<NNN>` zero-padded width 3 (src/commands/create-story.ts:176-204). The new story object literal
  field order is: id, title, type:"story", status:"backlog", createdAt, updatedAt, then optional description,
  epic, parent, and children:[] ONLY when there is NO parent (src/commands/create-story.ts:89-100). The id is
  pushed to states.backlog; parent.children gets the id appended; prefixCounters[prefix] is set to the new
  number. When --epic is given the epic's workUnits array in spec/epics.json gets the id appended
  (src/commands/create-story.ts:129-140). Dispatcher success result text contains '✓ Created story <id>',
  '  Title: <title>', and optional Description/Epic/Parent lines plus the Example-Mapping <system-reminder>.
  Reference rust/fspec-core/src/commands/create_epic.rs for the read→merge→write_json_atomic pattern.
  Two-front-doors: clap CLI and LLM dispatcher both call commands::create_story::run(args_json, project_root).
  """

  Background: User Story
    As a fspec maintainer
    I want to port the create-story command to the Rust fspec-core crate
    So that the standalone fspec binary can create story work units natively without delegating to TypeScript

  Scenario: Dispatcher creates a minimal story and writes spec/work-units.json
    Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    When I dispatch create-story with prefix='AUTH' and title='User login'
    Then the dispatcher returns success=true
    And spec/work-units.json contains a work unit AUTH-001 with type='story', status='backlog'
    And AUTH-001 has a non-empty createdAt and updatedAt
    And AUTH-001 has a children field equal to an empty array
    And AUTH-001 does NOT contain a 'parent' key
    And states.backlog contains 'AUTH-001'
    And prefixCounters.AUTH equals 1

  Scenario: New story object field order matches the TS object literal
    Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    When I dispatch create-story with prefix='AUTH' and title='User login'
    Then in the on-disk JSON the AUTH-001 keys appear in order id, title, type, status, createdAt, updatedAt, children

  Scenario: Dispatcher stores an optional description after updatedAt
    Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    When I dispatch create-story with prefix='AUTH', title='User login', and description='Email + password'
    Then the dispatcher returns success=true
    And spec/work-units.json shows AUTH-001.description='Email + password'
    And in the on-disk JSON the 'updatedAt' key appears before the 'description' key

  Scenario: ID generation increments using the prefixCounters high-water-mark
    Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH and an existing AUTH-001 story
    When I dispatch create-story with prefix='AUTH' and title='Second story'
    Then the dispatcher returns success=true
    And spec/work-units.json contains a work unit AUTH-002
    And prefixCounters.AUTH equals 2

  Scenario: A child story is linked to its parent and omits the children array
    Given a project root tempdir with spec/foundation.json, spec/prefixes.json registering prefix AUTH, and an existing parent AUTH-001
    When I dispatch create-story with prefix='AUTH', title='Child story', and parent='AUTH-001'
    Then the dispatcher returns success=true
    And spec/work-units.json shows AUTH-002.parent='AUTH-001'
    And AUTH-002 does NOT contain a 'children' key
    And AUTH-001.children contains 'AUTH-002'

  Scenario: An epic association appends the story id to the epic workUnits array
    Given a project root tempdir with spec/foundation.json, spec/prefixes.json registering prefix AUTH, and spec/epics.json containing epic 'auth'
    When I dispatch create-story with prefix='AUTH', title='User login', and epic='auth'
    Then the dispatcher returns success=true
    And spec/work-units.json shows AUTH-001.epic='auth'
    And spec/epics.json shows epic 'auth' workUnits contains 'AUTH-001'

  Scenario: Dispatcher rejects a missing foundation with the foundation-missing message
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch create-story with prefix='AUTH' and title='User login'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Project foundation not found'
    And the error message contains the substring '<system-reminder>'
    And spec/work-units.json does NOT exist

  Scenario: Dispatcher rejects an empty title
    Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    When I dispatch create-story with prefix='AUTH' and title=''
    Then the dispatcher returns success=false
    And the error message contains the substring 'Title is required'

  Scenario: Dispatcher rejects an unregistered prefix
    Given a project root tempdir with spec/foundation.json present and an empty spec/prefixes.json
    When I dispatch create-story with prefix='NOPE' and title='User login'
    Then the dispatcher returns success=false
    And the error message contains the substring "Prefix 'NOPE' is not registered"

  Scenario: Dispatcher rejects a non-existent parent
    Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    When I dispatch create-story with prefix='AUTH', title='Child', and parent='AUTH-999'
    Then the dispatcher returns success=false
    And the error message contains the substring "Parent story 'AUTH-999' does not exist"

  Scenario: Dispatcher rejects exceeding the maximum nesting depth
    Given a project root tempdir with spec/foundation.json, spec/prefixes.json registering prefix AUTH, and a three-level parent chain AUTH-001 -> AUTH-002 -> AUTH-003
    When I dispatch create-story with prefix='AUTH', title='Too deep', and parent='AUTH-003'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Maximum nesting depth (3) exceeded'

  Scenario: Dispatcher rejects a non-existent epic
    Given a project root tempdir with spec/foundation.json present, spec/prefixes.json registering prefix AUTH, and an empty spec/epics.json
    When I dispatch create-story with prefix='AUTH', title='User login', and epic='ghost'
    Then the dispatcher returns success=false
    And the error message contains the substring "Epic 'ghost' does not exist"

  Scenario: Dispatcher response text renders the success block and Example-Mapping reminder
    Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    When I dispatch create-story with prefix='AUTH' and title='User login'
    Then the DispatchResult.data contains the line '✓ Created story AUTH-001'
    And the DispatchResult.data contains the line '  Title: User login'
    And the DispatchResult.data contains the substring '<system-reminder>'
