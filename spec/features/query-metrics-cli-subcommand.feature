@done
@querying
@cli
@RPC-261
Feature: Port query-metrics command to Rust
  """
  Routing: dispatcher needs query-metrics moved from run_stub to run_ported and added to is_ported predicate — shared file change, supervisor required.
  CLI binding: codelet/fspec/src/main.rs needs a Mode::QueryMetrics clap variant — shared file change, supervisor required.
  Help registry: codelet/fspec-core/src/help/configs/mod.rs needs pub mod query_metrics; — shared file change, supervisor required.
  Timestamp parsing: use a tiny inline RFC-3339-ish parser yielding epoch milliseconds (i64); we never need the civil date. Parse failures collapse to 0 ms per TS Date(NaN) tolerance.
  File read MUST escalate parse and ENOENT errors. Cannot reuse read_work_units_or_empty (which swallows). Inline std::fs::read_to_string + serde_json + 'Failed to query metrics:' wrapping in commands/query_metrics.rs.
  Type field tolerance: WorkUnit type is Option<String> in fspec-core/types/work_unit.rs; we MUST NOT deserialise it strictly. Use WorkUnit::type_str() for the wu.type||'story' default.
  Existing WorkUnit struct lacks stateHistory typed field — round-trips via #[serde(flatten) extra]. Either extend WorkUnit (shared file) or read stateHistory ad-hoc via wu.extra.get("stateHistory"). PREFERRED: extend WorkUnit with Option<Vec<StateHistoryEntry>> typed field for clean access; flag as shared-file request.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Reads spec/work-units.json directly and ESCALATES any read or parse error wrapped as 'Failed to query metrics: <inner-message>'
  #   2. Does NOT auto-create spec/work-units.json: missing file is treated as a read error and surfaces as 'Failed to query metrics' (exit 1)
  #   3. When workUnitId is supplied and the id is missing, throws 'Work unit <id> not found' wrapped as 'Failed to query metrics: Work unit <id> not found'
  #   4. When workUnitId is supplied and the unit has no stateHistory (missing or empty), throws 'Work unit <id> has no state history' wrapped as 'Failed to query metrics: Work unit <id> has no state history'
  #   5. Single-unit cycleTime is computed as Math.round((last.timestamp - first.timestamp) / 3_600_000) hours, formatted '<H> hour' for 1 and '<H> hours' otherwise (including 0)
  #   6. Single-unit timePerState walks indices [0..len-2] and stores 'next.timestamp - current.timestamp' (Math.round hours, same pluralisation) under currentState.state — even if a state repeats later, only the FIRST occurrence's duration is kept (last wins per state key in TS Record write)
  #   7. Aggregate path with no workUnitId iterates Object.values(data.workUnits) in insertion order
  #   8. Type filter applies wu.type || 'story' (missing OR empty string collapses to 'story'); unknown variants are preserved verbatim and fail string-equality matches
  #   9. averageCycleTime averages over the subset where status=='done' AND stateHistory.length>0; if that subset is empty the field is OMITTED from the JSON entirely (undefined → not serialised)
  #   10. byType is populated ONLY when --type is NOT supplied; the map always contains the three keys story/task/bug in that exact insertion order, each with count and (optionally) averageCycleTime
  #   11. CLI accepts --work-unit-id <id>, --type <story|task|bug>, --format <text|json> (default 'text'); the help fixture additionally lists a vestigial --metric flag that the action handler never reads — Rust must preserve byte-for-byte help parity
  #   12. JSON output uses 2-space-indented pretty print preserving TS object-literal insertion order (cycleTime, timePerState for single; aggregateMetrics: { totalWorkUnits, completedWorkUnits, averageCycleTime?, byType? } for aggregate)
  #   13. Text output for aggregate prints 'Project Metrics' header, Total/Completed lines, optional Average Cycle Time, then 'By Type:' block (when byType is set) listing story/task/bug counts and per-type averages
  #   14. Text output for single-unit prints 'Work Unit Metrics' header, 'Cycle Time: <H> hour(s)' line, then 'Time Per State:' block listing every entry from timePerState in original insertion order
  #   15. Both invocation paths (dispatcher AND clap subcommand) MUST call the same fspec_core::commands::query_metrics::run function — RPC-003 two-front-doors invariant
  #
  # EXAMPLES:
  #   1. Dispatcher call: query-metrics with {} against a project whose spec/work-units.json contains 3 stories (1 done with 4h cycle) returns JSON aggregateMetrics with totalWorkUnits=3, completedWorkUnits=1, averageCycleTime='4 hours', byType.story.count=3
  #   2. Dispatcher call: query-metrics with {"workUnitId":"AUTH-001"} against a unit whose stateHistory has [{state:'backlog',t:0h}, {state:'specifying',t:2h}, {state:'done',t:5h}] returns JSON cycleTime='5 hours' and timePerState={backlog:'2 hours', specifying:'3 hours'}
  #   3. Dispatcher call: query-metrics with {"workUnitId":"NOPE-999"} against a project where the id is missing returns dispatch failure with error 'Failed to query metrics: Work unit NOPE-999 not found'
  #   4. Dispatcher call: query-metrics with {"workUnitId":"AUTH-001"} where AUTH-001 lacks stateHistory returns dispatch failure with error 'Failed to query metrics: Work unit AUTH-001 has no state history'
  #   5. Dispatcher call: query-metrics with {"type":"bug"} filters to bug-typed units only and the result has NO byType field
  #   6. Dispatcher call: query-metrics with {} against an empty work-units map yields aggregateMetrics={ totalWorkUnits:0, completedWorkUnits:0, byType:{ story:{count:0}, task:{count:0}, bug:{count:0} } } — averageCycleTime omitted at top level AND inside each byType entry
  #   7. Dispatcher call: query-metrics with {} against a missing spec/work-units.json returns dispatch failure with error beginning 'Failed to query metrics:' and exits 1; the file is NOT auto-created
  #   8. Dispatcher call: query-metrics with {"format":"text"} produces a non-JSON Project Metrics block when work-units exist; text vs json are the only allowed format values, with text default
  #   9. CLI: ./codelet/target/release/fspec query-metrics --help exits 0 and prints byte-for-byte the captured fixture (including the vestigial --metric line)
  #   10. CLI: fspec query-metrics --work-unit-id AUTH-001 --format json prints the same JSON the dispatcher produces against the same on-disk state, exit 0
  #   11. CLI: fspec query-metrics (no args) against an empty directory exits 1 and prints '✗ Query failed: Failed to query metrics: ...' to stderr (because work-units.json must exist)
  #   12. CLI bridge module codelet/fspec/src/query_metrics.rs contains NO inline aggregation or hours-formatting logic — only argv → JSON marshalling and delegation to fspec_core
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to invoke `query-metrics` through both the dispatcher and the Rust CLI binary
    So that I get identical metrics output to the TypeScript implementation with zero behavioural drift

  Scenario: CLI exposes query-metrics as a subcommand with flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec query-metrics --help` with NO_COLOR=1
    Then the command exits 0
    Then stdout is byte-for-byte identical to the captured fixture at codelet/fspec/tests/fixtures/help/query-metrics.txt
    Then stdout starts with a blank line followed by 'QUERY-METRICS'

  Scenario: CLI against missing work-units.json exits 1 with stderr Query failed prefix
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec query-metrics` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Query failed'
    Then stderr contains the substring 'Failed to query metrics:'
    Then spec/work-units.json was NOT created

  Scenario: CLI JSON output matches dispatcher output for the same on-disk state
    Given spec/work-units.json contains AUTH-001 with stateHistory at hour 0 (backlog) and hour 5 (done)
    When I run `./codelet/target/release/fspec query-metrics --work-unit-id AUTH-001 --format json` against that workspace
    Then the command exits 0
    Then stdout parses as JSON with cycleTime='5 hours'
    Then stdout equals the DispatchResult.data produced by dispatch_command for the same on-disk state followed by a trailing newline

  Scenario: CLI text output for aggregate path renders a Project Metrics block
    Given spec/work-units.json contains AUTH-001 (story, done with stateHistory 0→2h), AUTH-002 (story, backlog), BUG-001 (bug, done with stateHistory 0→1h)
    When I run `./codelet/target/release/fspec query-metrics`
    Then the command exits 0
    Then stdout contains the exact line 'Total Work Units: 3'
    Then stdout contains the exact line 'Completed Work Units: 2'
    Then stdout contains the substring 'By Type:'
    Then stdout contains the exact line '  story: 2 work units'
    Then stdout contains the exact line '  bug: 1 work unit'

  Scenario: CLI bridge module delegates to fspec_core with no inline aggregation logic
    Given the file codelet/fspec/src/query_metrics.rs exists as the CLI bridge module
    When I read the source of codelet/fspec/src/query_metrics.rs
    Then the source does NOT contain the substring 'Project Metrics'
    Then the source does NOT contain the substring 'aggregateMetrics'
    Then the source does NOT contain the substring 'cycleTime'
    Then the source does NOT contain the substring 'hour'
    Then the source calls codelet_fspec_core::commands::query_metrics::run
