@done
@rust
@cli
@RPC-250
Feature: Port list-schedules command to Rust

  """
  New impl file at codelet/fspec-core/src/commands/list_schedules.rs replaces the NotYetPorted stub. The module exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` with the same signature shape as list_hooks::run. Args struct deserializes `{format?: 'text'|'json'}` with `#[serde(default)]`.
  Schedules data is parsed using a lightweight Rust shape: `struct SchedulesFile { schedules: IndexMap<String, serde_json::Value> }` so that insertion order is preserved AND we surface each schedule entry value as-is (parity with TS `Object.values(data.schedules)`). The full ScheduleEntry union (agent vs shell) is intentionally not modelled — we re-emit the raw entry Value on the structured path.
  Error swallowing: the impl tolerates missing OR malformed `spec/schedules.json` (parity with the TS `fileManager.readJSON<SchedulesData>(file, defaultData)` semantics, which returns the default on missing/invalid file). Both paths produce the canonical `{schedules: [], columns: [...]}` payload.
  The `columns` array is a hard-coded constant: `["name","cron","timezone","type","status","lastRun","nextRun"]`. Both happy and swallow paths emit it.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-schedules` MUST replace the NotYetPorted stub
  #   2. Missing spec/schedules.json → success with schedules=[] + canonical columns; file is NOT auto-created
  #   3. Malformed JSON in spec/schedules.json → ALSO swallowed (success with schedules=[] + columns)
  #   4. Empty schedules map ({"schedules": {}}) → schedules=[] but columns still present
  #   5. Insertion order of schedules preserved (IndexMap)
  #   6. Populated file surfaces ALL ScheduleEntry fields verbatim (Object.values semantics)
  #   7. columns array is constant: ["name","cron","timezone","type","status","lastRun","nextRun"]
  #   8. JSON format produces 2-space indented payload with schedules + columns
  #   9. Text format: 'No schedules configured.' sentinel for empty; tab-separated header + rows + 'Total: N schedule(s)' for populated
  #  10. Default format (no format key supplied) is text
  #  11. CLI surface is flag-less aside from `--json` (parity with TS Commander.js `.option('--json', ...)`); dispatcher uses `format` key
  #  12. CLI delegates to single source of truth in fspec_core
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-schedules from the agent loop AND invoke `fspec list-schedules` from a shell
    So that I can audit the scheduled jobs configured for the project, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Returns empty schedules with canonical columns when spec/schedules.json does not exist
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch the list-schedules command against that project root with format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has schedules array of length 0
    Then the parsed JSON has columns equal to ["name","cron","timezone","type","status","lastRun","nextRun"]
    Then spec/schedules.json does not exist after the call

  Scenario: Returns schedule entries verbatim when spec/schedules.json is populated
    Given spec/schedules.json contains a shell schedule named 'nightly-build' with cron '0 2 * * *' and an agent schedule named 'morning-standup' with cron '0 9 * * 1-5' in that order
    When I dispatch list-schedules with format='json'
    Then the dispatcher returns success=true
    Then the schedules array contains exactly two entries
    Then the first schedule has name='nightly-build', cron='0 2 * * *', jobType='shell', status='active'
    Then the second schedule has name='morning-standup', cron='0 9 * * 1-5', jobType='agent'
    Then the parsed JSON has columns equal to ["name","cron","timezone","type","status","lastRun","nextRun"]

  Scenario: Treats empty schedules map as no schedules but still emits columns
    Given spec/schedules.json exists and parses to an object whose 'schedules' field is the empty object
    When I dispatch list-schedules with format='json'
    Then the dispatcher returns success=true
    Then the schedules array has length 0
    Then the parsed JSON has columns equal to ["name","cron","timezone","type","status","lastRun","nextRun"]

  Scenario: Swallows invalid JSON as empty result with canonical columns
    Given spec/schedules.json exists but contains the malformed bytes '{ not json'
    When I dispatch list-schedules with format='json'
    Then the dispatcher returns success=true
    Then the schedules array has length 0
    Then the parsed JSON has columns equal to ["name","cron","timezone","type","status","lastRun","nextRun"]
    Then spec/schedules.json still contains the original malformed bytes after the call

  Scenario: Preserves insertion order of schedules (not alphabetical)
    Given spec/schedules.json contains three schedule entries declared in order ZED, AAA, MID
    When I dispatch list-schedules with format='json'
    Then the dispatcher returns success=true
    Then the schedules array contains three entries in order ZED, AAA, MID

  Scenario: JSON format emits two-space indented payload for the empty/missing case
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch list-schedules with format='json'
    Then the DispatchResult.data starts with the exact string "{\n  \"schedules\": [],\n"
    Then the DispatchResult.data contains the exact substring "\"columns\": ["
    Then the DispatchResult.data contains the exact substring "\"name\""

  Scenario: Text format prints 'No schedules configured.' sentinel for the empty/missing case
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch list-schedules with format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the exact line 'No schedules configured.'
    Then the DispatchResult.data contains the exact line 'Use `fspec add-schedule` to create a schedule.'

  Scenario: Text format renders the populated case using the documented help-example layout
    Given spec/schedules.json contains one shell schedule named 'nightly-build' with cron '0 2 * * *' timezone 'UTC' status 'active' and lastRunAt null
    When I dispatch list-schedules with format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the tab-separated header line 'Name\tCron\tTimezone\tType\tStatus\tLast Run\tNext Run'
    Then the DispatchResult.data contains a line that begins with 'nightly-build\t0 2 * * *\tUTC\tshell\tactive\t'
    Then the DispatchResult.data contains the exact line 'Total: 1 schedule(s)'

  Scenario: Default format (no format key supplied) is text
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch list-schedules with an empty args object {}
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the exact line 'No schedules configured.'
