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

  Scenario: Aggregate JSON with no filter computes totals, completion and averages
    Given spec/work-units.json contains AUTH-001 (status done, stateHistory backlog→done spanning 4 hours), AUTH-002 (backlog with stateHistory), and AUTH-003 (backlog, no stateHistory)
    When I dispatch query-metrics with format='json' and no other args
    Then the dispatcher returns success=true
    Then DispatchResult.data parses as JSON whose aggregateMetrics.totalWorkUnits equals 3
    Then aggregateMetrics.completedWorkUnits equals 1
    Then aggregateMetrics.averageCycleTime equals '4 hours'
    Then aggregateMetrics.byType has keys story, task, bug in that exact order with story.count=3, task.count=0, bug.count=0


  Scenario: Single work unit JSON returns cycleTime and timePerState
    Given spec/work-units.json contains AUTH-001 with stateHistory entries at hour 0 (backlog), hour 2 (specifying), hour 5 (done)
    When I dispatch query-metrics with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then DispatchResult.data parses as JSON with cycleTime='5 hours'
    Then timePerState.backlog='2 hours' and timePerState.specifying='3 hours'
    Then the JSON does NOT contain an aggregateMetrics key


  Scenario: Unknown work unit id fails with wrapped error
    Given spec/work-units.json contains AUTH-001 but no NOPE-999
    When I dispatch query-metrics with workUnitId='NOPE-999'
    Then the dispatcher returns success=false with an error message containing 'Failed to query metrics: Work unit NOPE-999 not found'


  Scenario: Work unit without state history fails with wrapped error
    Given spec/work-units.json contains AUTH-001 with no stateHistory field
    When I dispatch query-metrics with workUnitId='AUTH-001'
    Then the dispatcher returns success=false with an error message containing 'Failed to query metrics: Work unit AUTH-001 has no state history'


  Scenario: Type filter omits byType from the result
    Given spec/work-units.json contains a story AUTH-001, a task TASK-001 and a bug BUG-001
    When I dispatch query-metrics with type='bug' and format='json'
    Then DispatchResult.data.aggregateMetrics.totalWorkUnits equals 1
    Then DispatchResult.data.aggregateMetrics does NOT contain a byType key


  Scenario: Empty work units map preserves the three byType keys with zero counts
    Given spec/work-units.json exists with an empty workUnits object
    When I dispatch query-metrics with format='json'
    Then aggregateMetrics.totalWorkUnits=0 and aggregateMetrics.completedWorkUnits=0
    Then aggregateMetrics does NOT contain an averageCycleTime key
    Then aggregateMetrics.byType keys are exactly story, task, bug in that order with all counts equal to 0 and no averageCycleTime keys


  Scenario: Missing work-units.json escalates as a wrapped Failed to query metrics error
    Given the project root has no spec/work-units.json
    When I dispatch query-metrics with format='json'
    Then the dispatcher returns success=false with an error message starting with 'Failed to query metrics:'
    Then spec/work-units.json still does not exist after the call


  Scenario: Text aggregate output is human-readable and not JSON
    Given spec/work-units.json contains AUTH-001 (status done, stateHistory 0h→2h) and AUTH-002 (backlog)
    When I dispatch query-metrics with format='text'
    Then DispatchResult.data contains the substring 'Project Metrics'
    Then DispatchResult.data contains the exact line 'Total Work Units: 2'
    Then DispatchResult.data contains the exact line 'Completed Work Units: 1'
    Then DispatchResult.data contains the substring 'By Type:'
    Then DispatchResult.data does NOT start with '{'

