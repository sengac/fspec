@done
@RPC-263
@cli
@querying
@rust
Feature: Port query-work-units command to Rust
  """
  WorkUnit struct in rust/fspec-core/src/types/work_unit.rs currently exposes id, title, status, type, epic, createdAt, updatedAt + extra; query-work-units needs to read additional fields (tags, questions, stateHistory, featureFile, estimate) via the existing `extra` JSON map rather than extending the typed struct, keeping the worker self-contained and avoiding edits to shared type files
  Unlike list-work-units, query-work-units does NOT auto-create spec/work-units.json on first run; implementation uses std::fs::read_to_string + serde_json::from_str directly with TS-style error prefix 'Failed to query work units:' (rather than the ensure_work_units_file helper)
  Two-front-doors invariant: pub async fn run(args_json: &str, project_root: &Path) lives in rust/fspec-core/src/commands/query_work_units.rs; both the clap CLI bridge (rust/fspec/src/query_work_units.rs) and the LLM dispatcher delegate to this single function
  The CLI bridge mirrors a TS quirk: --format=json prints JSON to stdout, but --format=text/csv/table prints NOTHING (the Commander.js action only logs when format==='json'); preserving this for byte-for-byte parity
  Output JSON shape is RICHER than list-work-units: { workUnits: [<full WU objects>], format: 'json', data: [{ workUnitId, featureFilePath }, ...] } — featureFilePath defaults to 'unknown' when wu.featureFile is absent in the extra map
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to run `fspec query-work-units` with advanced filters (status, prefix, epic, type, tag) plus text/json/csv output formats
    So that I can perform advanced queries on work units identically to the TypeScript implementation without depending on Node.js

  Scenario: Dispatcher returns wrapped error when spec/work-units.json is missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch the query-work-units command against that project root
    Then the dispatcher returns an error whose message contains the substring 'Failed to query work units:'
    Then spec/work-units.json is NOT created (unlike list-work-units)

  Scenario: Tag filter returns only work units containing the specified tag
    Given spec/work-units.json contains AUTH-001 (backlog, tags '@cli') and AUTH-002 (implementing, tags '@high')
    When I dispatch query-work-units with tag='@cli' and format='json'
    Then the workUnits array contains only AUTH-001
    Then the data array contains only the entry {workUnitId:'AUTH-001'}

  Scenario: Combined status and prefix filters apply AND semantics
    Given spec/work-units.json contains AUTH-001 (backlog), AUTH-002 (implementing), DASH-001 (backlog)
    When I dispatch query-work-units with status='implementing' and prefix='AUTH' and format='json'
    Then the workUnits array contains only AUTH-002

  Scenario: JSON data array defaults featureFilePath to unknown when WU has no featureFile field
    Given spec/work-units.json contains AUTH-001 (featureFile 'auth.feature') and AUTH-002 (no featureFile field)
    When I dispatch query-work-units with format='json'
    Then the data array contains {workUnitId:'AUTH-001', featureFilePath:'auth.feature'}
    Then the data array contains {workUnitId:'AUTH-002', featureFilePath:'unknown'}

  Scenario: CSV format strips commas from title and writes header plus rows to output file
    Given spec/work-units.json contains AUTH-001 (title 'Login, advanced') and AUTH-002 (title 'Logout')
    When I dispatch query-work-units with format='csv' and output to a temp file path
    Then the output file's first line equals 'id,title,status,createdAt,updatedAt'
    Then the output file contains a row for AUTH-001 whose title field equals 'Login advanced' (comma stripped)
    Then the output file contains a row for AUTH-002 whose title field equals 'Logout'

  Scenario: Cycle-time mode returns per-state hour deltas and total
    Given spec/work-units.json contains AUTH-001 with stateHistory ['backlog'@2026-06-01T00:00:00Z, 'specifying'@2026-06-01T02:00:00Z, 'testing'@2026-06-01T05:00:00Z]
    When I dispatch query-work-units with workUnitId='AUTH-001' and showCycleTime=true
    Then the result contains stateTimings { backlog: '2 hours', specifying: '3 hours' }
    Then the result contains totalCycleTime '5 hours'

  Scenario: Cycle-time mode singularises 'hour' when delta equals 1
    Given spec/work-units.json contains AUTH-001 with stateHistory ['backlog'@2026-06-01T00:00:00Z, 'specifying'@2026-06-01T01:00:00Z]
    When I dispatch query-work-units with workUnitId='AUTH-001' and showCycleTime=true
    Then the result contains stateTimings { backlog: '1 hour' }
    Then the result contains totalCycleTime '1 hour'

  Scenario: questionsFor filter normalises bare username to @username and matches included mentions
    Given spec/work-units.json contains AUTH-001 (questions text '@bob what about timeout?' and '@alice clarify scope') and AUTH-002 (no questions)
    When I dispatch query-work-units with questionsFor='bob' and format='json'
    Then the workUnits array contains only AUTH-001
    When I dispatch query-work-units again with questionsFor='@bob' and format='json'
    Then the workUnits array still contains only AUTH-001

  Scenario: Sort by numeric field with descending order produces decreasing values
    Given spec/work-units.json contains AUTH-001 (estimate 5), AUTH-002 (estimate 3), AUTH-003 (estimate 8)
    When I dispatch query-work-units with sort='estimate' and order='desc' and format='json'
    Then the workUnits array order is AUTH-003 then AUTH-001 then AUTH-002

  Scenario: hasQuestions=true filter keeps only work units with non-empty questions
    Given spec/work-units.json contains AUTH-001 (questions present) and AUTH-002 (no questions)
    When I dispatch query-work-units with hasQuestions=true and format='json'
    Then the workUnits array contains only AUTH-001

  Scenario: Two front doors — the same fspec_core::commands::query_work_units::run function serves CLI and dispatcher
    Given rust/fspec-core/src/commands/query_work_units.rs exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`
    When I inspect rust/fspec/src/query_work_units.rs
    Then the CLI bridge module delegates to fspec_core::commands::query_work_units::run with the project_root resolved from std::env::current_dir
    Then no filter or rendering logic is duplicated in the CLI bridge
