@querying
@cli
@done
@RPC-302
Feature: Port show-epic command to Rust
  """
  show-epic is the single-epic variant of list-epics (already ported under RPC-243). The Rust port reuses the existing typed `Epic` struct at rust/fspec-core/src/types/epic.rs and the bare-catch helper `io::ensure::read_work_units_or_empty`. The epics.json read path differs from list-epics: show-epic must surface ENOENT as the canonical 'Epic <id> not found' error (NOT the empty-list fallback), so the implementation calls std::fs::read_to_string directly with explicit ErrorKind::NotFound handling.

  Percentage rounding differs from list-epics: show-epic uses TS `Math.round((c/t)*100*100)/100` to retain 2 decimal places (1/3 → 33.33, 2/3 → 66.67, 1/2 → 50), whereas list-epics rounds to an integer (1/3 → 33).

  The dispatcher payload shape is `{ epicId: String (REQUIRED), format: Option<String> }`. The CLI bridge marshals the positional `<epicId>` argument and `-f, --format` flag into the same JSON shape and delegates to the SAME `fspec_core::commands::show_epic::run` function used by the dispatcher (RPC-003 §7/§11 two-front-doors invariant).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `show-epic` MUST replace the NotYetPorted stub and return a real DispatchResult
  #   2. When spec/epics.json is MISSING (ENOENT), the command MUST return success=false with error 'Epic <epicId> not found' and MUST NOT auto-create files
  #   3. When the requested epicId is not a key in epicsData.epics, the command MUST return success=false with the SAME 'Epic <epicId> not found' error
  #   4. When spec/work-units.json is missing OR malformed, the command MUST silently treat counts as zero (parity with TS bare catch {})
  #   5. totalWorkUnits counts work units whose `epic` field exactly equals the requested epicId; completedWorkUnits is the subset with status=='done'
  #   6. completionPercentage uses 2-decimal-place Math.round semantics: 1/3 → 33.33, 2/3 → 66.67, 1/2 → 50, 4/4 → 100, 0/0 → 0
  #   7. JSON format wraps the result as { epic: {full Epic object}, totalWorkUnits, completedWorkUnits, completionPercentage } with 2-space indentation
  #   8. Text format prints a single block headed by 'Epic: <id>' with Title/Description/Progress sections
  #   9. When epic.title is missing the text output renders 'Title: N/A'
  #   10. show-epic --help is byte-for-byte identical to the captured TS reference fixture
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch show-epic against an epic id and see that epic's details and progress
    So that I can audit a single epic's metadata and completion ratio without iterating the full list, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Returns Epic not found error when spec/epics.json is missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch the show-epic command with epicId='auth' against that project root
    Then the dispatcher returns success=false with an error message exactly 'Epic auth not found'
    Then spec/epics.json does not exist after the call
    Then spec/work-units.json does not exist after the call

  Scenario: Returns Epic not found error when epicId is not registered in epics.json
    Given spec/epics.json contains epic 'auth' with title 'Authentication'
    When I dispatch show-epic with epicId='nonexistent'
    Then the dispatcher returns success=false with an error message exactly 'Epic nonexistent not found'

  Scenario: Escalates malformed epics.json as a structured parse error
    Given spec/epics.json exists but contains invalid JSON syntax
    When I dispatch show-epic with epicId='auth' against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse epics.json'

  Scenario: Aggregates work-unit completion progress for the requested epic
    Given spec/epics.json contains epic 'auth' with title 'Authentication' and description 'Login features'
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=backlog), DASH-001 (epic=dash, status=done)
    When I dispatch show-epic with epicId='auth' and format='json'
    Then the dispatcher returns success=true
    Then the result has totalWorkUnits=2, completedWorkUnits=1, completionPercentage=50
    Then the result.epic.id equals 'auth'

  Scenario: Treats missing work-units.json as zero counts without throwing
    Given spec/epics.json contains epic 'auth' with title 'Authentication'
    Given spec/work-units.json does NOT exist
    When I dispatch show-epic with epicId='auth' and format='json'
    Then the dispatcher returns success=true
    Then the result has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0

  Scenario: Treats malformed work-units.json as zero counts without throwing
    Given spec/epics.json contains epic 'auth' with title 'Authentication'
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch show-epic with epicId='auth' and format='json'
    Then the dispatcher returns success=true
    Then the result has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0

  Scenario: completionPercentage rounds 1/3 to 33.33 with 2-decimal precision
    Given spec/epics.json contains epic 'auth' with title 'Authentication'
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=backlog), AUTH-003 (epic=auth, status=backlog)
    When I dispatch show-epic with epicId='auth' and format='json'
    Then the result has totalWorkUnits=3, completedWorkUnits=1, completionPercentage=33.33

  Scenario: completionPercentage rounds 2/3 to 66.67 with 2-decimal precision
    Given spec/epics.json contains epic 'auth' with title 'Authentication'
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=done), AUTH-003 (epic=auth, status=backlog)
    When I dispatch show-epic with epicId='auth' and format='json'
    Then the result has totalWorkUnits=3, completedWorkUnits=2, completionPercentage=66.67

  Scenario: completionPercentage returns 100 when every work unit is done
    Given spec/epics.json contains epic 'auth' with title 'Authentication'
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done) and AUTH-002 (epic=auth, status=done)
    When I dispatch show-epic with epicId='auth' and format='json'
    Then the result has totalWorkUnits=2, completedWorkUnits=2, completionPercentage=100

  Scenario: Text format renders the Epic header Title Description and Progress block
    Given spec/epics.json contains epic 'auth' with title 'Authentication' and description 'Login features'
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=done), AUTH-003 (epic=auth, status=backlog), AUTH-004 (epic=auth, status=backlog)
    When I dispatch show-epic with epicId='auth' and format='text'
    Then the DispatchResult.data contains the line 'Epic: auth'
    Then the DispatchResult.data contains the line 'Title: Authentication'
    Then the DispatchResult.data contains the line 'Description: Login features'
    Then the DispatchResult.data contains the line 'Progress:'
    Then the DispatchResult.data contains the exact line '  Total work units: 4'
    Then the DispatchResult.data contains the exact line '  Completed: 2'
    Then the DispatchResult.data contains the exact line '  Completion: 50%'

  Scenario: Text format omits the Description line when description is missing
    Given spec/epics.json contains epic 'auth' with title 'Authentication' and no description field
    Given spec/work-units.json does NOT exist
    When I dispatch show-epic with epicId='auth' and format='text'
    Then the DispatchResult.data contains the line 'Epic: auth'
    Then the DispatchResult.data contains the line 'Title: Authentication'
    Then the DispatchResult.data does NOT contain the substring 'Description:'

  Scenario: Text format renders Title N/A when epic title is missing
    Given spec/epics.json contains epic 'auth' with no title field
    Given spec/work-units.json does NOT exist
    When I dispatch show-epic with epicId='auth' and format='text'
    Then the DispatchResult.data contains the line 'Title: N/A'

  Scenario: Default format is text when format flag is not supplied
    Given spec/epics.json contains epic 'auth' with title 'Authentication'
    Given spec/work-units.json does NOT exist
    When I dispatch show-epic with epicId='auth' and no format flag
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the line 'Epic: auth'
    Then the DispatchResult.data contains the line 'Title: Authentication'

  Scenario: JSON format emits two-space indented payload with the canonical field set
    Given spec/epics.json contains epic 'auth' with title 'Authentication'
    Given spec/work-units.json does NOT exist
    When I dispatch show-epic with epicId='auth' and format='json'
    Then the DispatchResult.data parses as JSON whose root object has an 'epic' object key
    Then the root object has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    Then the root.epic object has id='auth' and title='Authentication'
    Then the DispatchResult.data uses 2-space indentation

  Scenario: Missing epicId argument surfaces a structured InvalidArgs error
    Given spec/epics.json contains epic 'auth' with title 'Authentication'
    When I dispatch show-epic with no epicId argument
    Then the dispatcher returns success=false with an error message containing the substring 'failed to parse args'

  Scenario: Shared infrastructure modules already exist for reuse
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/
    Then the module io::ensure::read_work_units_or_empty exists and is publicly accessible from the crate root
    Then types::epic::Epic exists with id, title, description and a flatten extra map
    Then commands/show_epic.rs delegates to these shared modules rather than embedding its own filesystem logic
    Then commands/show_epic.rs does NOT return FspecCoreError::NotYetPorted
