@done
@rust
@querying
@cli
@RPC-243
Feature: Port list-epics command to Rust
  """
  New shared helper `io::ensure::read_epics_or_empty(cwd) -> Result<EpicsData, FspecCoreError>` lives alongside `read_prefixes_or_empty` and returns `Ok(EpicsData::initial())` on ENOENT instead of auto-creating. Mirrors the RPC-248 read-only / load-or-init split exactly.

  Reuses the existing RPC-248 `read_work_units_or_empty` helper for the bare-catch work-units read path — the TS bare `catch {}` semantics at src/commands/list-epics.ts:60-66 are identical to those at src/commands/list-prefixes.ts:57-63, so no new helper is needed.

  The shape of each Epic record in spec/epics.json is `{ id: string, title?: string, description?: string, ...extra }` (TS interface at src/commands/list-epics.ts:7-12). Add a typed `Epic` struct to rust/fspec-core/src/types/epic.rs with `#[serde(flatten)] extra: Map<String, Value>` for forward-compat. EpicsData container goes alongside PrefixesData keyed by `IndexMap<String, Epic>` to preserve insertion order.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-epics` MUST replace the NotYetPorted stub and return a real DispatchResult through the same `poll_sync_future` path used by RPC-248 / RPC-253
  #   2. If spec/epics.json is missing, the command MUST return success with an empty epics list and MUST NOT auto-create the file
  #   3. If spec/work-units.json is missing OR contains malformed JSON, the command MUST silently treat work-unit counts as zero for every epic and MUST NOT throw
  #   4. If spec/epics.json exists but is malformed, the command MUST propagate a structured error containing the substring 'Failed to parse epics.json'
  #   5. For each epic, totalWorkUnits = work-units whose `epic` field exactly equals epic.id; completedWorkUnits = subset with status=='done'; completionPercentage = Math.round((completed/total)*100) when total > 0, else 0
  #   6. The association between work units and epics is EXACT-MATCH on the WorkUnit.epic field — NOT prefix-based
  #   7. Output preserves insertion order of spec/epics.json (IndexMap in Rust)
  #   8. Each epic entry: id (string), title (optional), description (optional), totalWorkUnits, completedWorkUnits, completionPercentage
  #   9. Text format prints 'No epics found' for empty lists; populated lists print '\nEpics (N)\n' header followed by each epic; description and Work Units lines are conditionally omitted
  #   10. JSON format wraps the result in `{ epics: [...] }` with 2-space indentation
  #
  # EXAMPLES:
  #   1. Empty dir → empty epics array; no files created
  #   2. Two epics with progress aggregated correctly via exact-match
  #   3. Missing work-units.json → zero counts (no throw)
  #   4. Malformed work-units.json → zero counts (no throw)
  #   5. Malformed epics.json → error 'Failed to parse epics.json'
  #   6. Insertion order preserved (zed, aaa, mid)
  #   7. No matching work units → no 'Work Units:' line
  #   8. No description → no description line
  #   9. Math.round 1/3 → 33%
  #   10. Math.round 2/3 → 67%
  #   11. JSON output uses 2-space indent and omits optional fields
  #   12. Empty epics object → 'No epics found'
  #   13. Unmatched epic field → ignored by aggregation
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-epics from the agent loop and get the same epic listing — with per-epic work-unit completion progress — as the TypeScript implementation
    So that I can audit epic registration and progress without relying on Node.js, sharing one source-of-truth between the LLM dispatcher and the CLI

  Scenario: Returns an empty epics list when spec/ does not exist and does not auto-create files
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch the list-epics command against that project root with format='json'
    Then the dispatcher returns success=true with an empty epics array
    Then spec/epics.json does not exist after the call
    Then spec/work-units.json does not exist after the call

  Scenario: Aggregates work-unit completion progress per epic by exact-match
    Given spec/epics.json contains auth (title 'Authentication', description 'Login features') and dash (title 'Dashboard', description 'Dashboard features') in that order
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=backlog), DASH-001 (epic=dash, status=done), DASH-002 (epic=dash, status=done)
    When I dispatch list-epics with format='json'
    Then the epics array contains exactly two entries in order auth then dash
    Then the auth entry has totalWorkUnits=2, completedWorkUnits=1, completionPercentage=50
    Then the dash entry has totalWorkUnits=2, completedWorkUnits=2, completionPercentage=100

  Scenario: Treats missing work-units.json as zero counts without throwing
    Given spec/epics.json contains auth (title 'Authentication')
    Given spec/work-units.json does NOT exist
    When I dispatch list-epics with format='json'
    Then the dispatcher returns success=true
    Then the auth entry has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0

  Scenario: Treats malformed work-units.json as zero counts without throwing
    Given spec/epics.json contains auth (title 'Authentication')
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch list-epics with format='json'
    Then the dispatcher returns success=true
    Then the auth entry has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0

  Scenario: Escalates malformed epics.json as a structured parse error
    Given spec/epics.json exists but contains invalid JSON syntax
    When I dispatch list-epics against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse epics.json'

  Scenario: Preserves insertion order of epics.json (not alphabetical)
    Given spec/epics.json contains three epics registered in order zed, aaa, mid
    When I dispatch list-epics with format='text'
    Then the DispatchResult.data contains the substring 'Epics (3)'
    Then the substring 'zed' appears before 'aaa' which appears before 'mid' in the output

  Scenario: Text format omits the 'Work Units' line when totalWorkUnits is zero
    Given spec/epics.json contains auth with title 'Authentication' and description 'Login features'
    Given spec/work-units.json contains no work units whose epic equals 'auth'
    When I dispatch list-epics with format='text'
    Then the DispatchResult.data contains the line 'auth'
    Then the DispatchResult.data contains the line '  Authentication'
    Then the DispatchResult.data contains the line '  Login features'
    Then the DispatchResult.data does NOT contain the substring 'Work Units:'

  Scenario: Text format omits the description line when the description is missing
    Given spec/epics.json contains auth with title 'Authentication' and no description field
    Given spec/work-units.json does NOT exist
    When I dispatch list-epics with format='text'
    Then the DispatchResult.data contains the line 'auth'
    Then the DispatchResult.data contains the line '  Authentication'
    Then the DispatchResult.data does NOT contain the substring '  Login features'

  Scenario: Text format renders completion progress with Math.round semantics (1/3 rounds to 33)
    Given spec/epics.json contains auth with title 'Authentication'
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=backlog), AUTH-003 (epic=auth, status=backlog)
    When I dispatch list-epics with format='text'
    Then the DispatchResult.data contains the exact line '  Work Units: 1/3 (33%)'

  Scenario: Text format rounds 2/3 progress to 67 percent
    Given spec/epics.json contains auth with title 'Authentication'
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=done), AUTH-003 (epic=auth, status=backlog)
    When I dispatch list-epics with format='text'
    Then the DispatchResult.data contains the exact line '  Work Units: 2/3 (67%)'

  Scenario: JSON format emits two-space indented payload omitting unset optional fields
    Given spec/epics.json contains auth with title 'Authentication' and no description
    Given spec/work-units.json does NOT exist
    When I dispatch list-epics with format='json'
    Then the DispatchResult.data parses as JSON whose root object has an 'epics' array of length 1
    Then the first epics entry has id='auth', title='Authentication', totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    Then the first epics entry does NOT contain a 'description' key
    Then the DispatchResult.data uses 2-space indentation

  Scenario: Text format prints 'No epics found' for an empty epics object
    Given spec/epics.json exists with an empty epics object
    When I dispatch list-epics with format='text'
    Then the DispatchResult.data is exactly the string 'No epics found'

  Scenario: Work units with unmatched epic field are ignored by aggregation
    Given spec/epics.json contains auth with title 'Authentication'
    Given spec/work-units.json contains AUTH-001 (epic=nonexistent, status=done) and AUTH-002 (epic=auth, status=done)
    When I dispatch list-epics with format='json'
    Then the auth entry has totalWorkUnits=1, completedWorkUnits=1, completionPercentage=100

  Scenario: Shared infrastructure modules exist under rust/fspec-core for reuse
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/
    Then the module io::ensure::read_epics_or_empty exists and is publicly accessible from the crate root
    Then types::epic::Epic exists and EpicsData.epics is keyed by an IndexMap to preserve insertion order
    Then list_epics::run delegates to these shared modules rather than embedding its own filesystem logic
