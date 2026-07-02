@done
@cli
@querying
@RPC-259
@wip
Feature: Port query-estimation-guide command to Rust
  """
  Use ensure_work_units_file from crate::io::ensure (auto-creating spec/work-units.json with canonical defaults if missing) — Framing A divergence from TS source: TS calls bare readFile and errors on ENOENT with an unhelpful "Failed to query estimation guide: ENOENT" message; the Rust port auto-creates the canonical empty store so the dispatcher returns {patterns: []} on a fresh project.
  Read estimate and iterations from WorkUnit.extra via extra.get("estimate").and_then(Value::as_u64) and extra.get("iterations").and_then(Value::as_u64) — these fields are not modeled on shared types/work_unit.rs (file-ownership rule: shared types module is supervisor-only).
  Group buckets in a BTreeMap<u64, Vec<u64>> so iteration order is naturally ascending by points; TS uses Object.entries which yields integer-key buckets in ascending order — BTreeMap preserves this parity for free.
  EstimationPattern struct uses #[derive(Serialize)] with declaration order {points, expectedIterations, confidence} and #[serde(rename_all = "camelCase")]; expectedIterations is rendered via format!("{min}-{max}").
  Args struct accepts {workUnitId, format} — workUnitId is REQUIRED at the clap surface; core function deserializes it but discards it (TS parity bug we replicate). CLI bridge prints JSON only when args.format == 'json'.
  Both invocation paths (LLM dispatcher and clap subcommand) call the single fspec_core::commands::query_estimation_guide::run function; CLI bridge does only JSON arg marshalling and stdout rendering.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Loads spec/work-units.json via ensure_work_units_file (Framing A: Rust auto-creates canonical empty store)
  #   2. Filters to work units whose status equals 'done'
  #   3. Only counts done units with BOTH a truthy estimate and a defined iterations field
  #   4. Groups iteration values by their estimate (story-point bucket)
  #   5. For each bucket emits {points, expectedIterations: 'min-max', confidence}
  #      where confidence = 'high' (>=4 samples), 'medium' (>=2 samples), 'low' (<2)
  #   6. Patterns sorted ascending by points
  #   7. Result JSON field order is declaration order
  #   8. Positional workUnitId is REQUIRED at clap but unused by core function
  #   9. Text format (default) prints NOTHING to stdout; JSON format prints pretty JSON
  #  10. CLI exit codes: 0 on success, 1 on error
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust port of query-estimation-guide wired through both the LLM dispatcher and the clap subcommand
    So that the fspec daemon and the standalone Rust binary share one estimation-guidance implementation with byte-parity to the TS source

  Scenario: Returns empty patterns array when work-units.json is auto-created in an empty workspace
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    Then the dispatcher returns success=true
    And the returned JSON has patterns=[]
    And spec/work-units.json exists after the call (auto-created by ensure_work_units_file)

  Scenario: Ignores non-done work units entirely
    Given spec/work-units.json contains A with status='backlog', estimate=3, iterations=1
    And spec/work-units.json also contains B with status='implementing', estimate=5, iterations=2
    When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    Then the returned JSON has patterns=[]

  Scenario: Skips done unit missing iterations field
    Given spec/work-units.json contains A with status='done', estimate=3 and no iterations field
    When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    Then the returned JSON has patterns=[]

  Scenario: Skips done unit missing estimate field
    Given spec/work-units.json contains A with status='done', iterations=1 and no estimate field
    When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    Then the returned JSON has patterns=[]

  Scenario: Single done unit with estimate=3 and iterations=1 yields a low-confidence pattern
    Given spec/work-units.json contains A with status='done', estimate=3, iterations=1
    When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    Then the returned JSON has patterns containing exactly one entry {points=3, expectedIterations='1-1', confidence='low'}

  Scenario: Two done units with estimate=3 and iterations [1,2] yield medium confidence
    Given spec/work-units.json contains A with status='done', estimate=3, iterations=1 and B with status='done', estimate=3, iterations=2
    When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    Then the returned JSON has patterns containing exactly one entry {points=3, expectedIterations='1-2', confidence='medium'}

  Scenario: Four done units with estimate=5 and iterations [1,2,3,4] yield high confidence
    Given spec/work-units.json contains four done units all with estimate=5 and iterations 1, 2, 3, 4 respectively
    When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    Then the returned JSON has patterns containing exactly one entry {points=5, expectedIterations='1-4', confidence='high'}

  Scenario: Two buckets emit patterns sorted ascending by points
    Given spec/work-units.json contains two done units with estimate=5, iterations [1,2] and two done units with estimate=3, iterations [1,2]
    When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    Then patterns[0].points=3 and patterns[1].points=5

  Scenario: JSON field declaration order is preserved
    Given spec/work-units.json contains a single done unit with estimate=3 and iterations=1
    When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    Then the pattern object's field declaration order is points, expectedIterations, confidence

  Scenario: Escalates malformed work-units.json as a structured parse error
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch query-estimation-guide against that project root with workUnitId='ANY-001' and format='json'
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
