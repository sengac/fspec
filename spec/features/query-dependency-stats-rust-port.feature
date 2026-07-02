@done
@cli
@querying
@RPC-257
Feature: Port query-dependency-stats command to Rust
  """
  Use ensure_work_units_file from crate::io::ensure (auto-creating spec/work-units.json with canonical defaults if missing) — TS source-of-truth at src/commands/query-dependency-stats.ts:72 calls ensureWorkUnitsFile, so the Rust port matches that auto-create behaviour exactly.
  Read the four dependency arrays (blocks, blockedBy, dependsOn, relatesTo) from WorkUnit.extra via extra.get(k).and_then(Value::as_array) — do NOT add typed fields to the shared types/work_unit.rs (file-ownership rule: shared types module is supervisor-only).
  Result struct uses #[derive(Serialize)] with explicit declaration-order fields and #[serde(rename_all = "camelCase")] to mirror TS JSON.stringify field order. Do NOT route through json!{} which alphabetizes via BTreeMap.
  Math.round(x*100)/100 implemented as ((x*100.0)+0.5).floor()/100.0 — positive values only because counts are non-negative.
  averageDependenciesPerUnit is serialized as a JSON number. When the rounded value is integer-valued (e.g. 1.0) it MUST serialize as "1" (no decimal point) to match TS JSON.stringify behaviour; non-integer values serialize with their decimal representation (e.g. 0.5).
  DFS for max chain depth: clone visited HashSet on each recursive call (parity with TS `new Set(visited)`). Return 0 on already-visited; recurse over blocks array entries; final value = max_child_depth + (blocks.is_empty() ? 0 : 1).
  CLI bridge text path is intentionally silent (TS source bug we replicate). Do NOT add a render_text function — the bridge prints stdout only when args.format == 'json'.
  Both invocation paths (LLM dispatcher and clap subcommand) call the single fspec_core::commands::query_dependency_stats::run function; CLI bridge does only JSON arg marshalling and stdout rendering.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Loads spec/work-units.json via the shared ensure_work_units_file helper (auto-creating the file with canonical defaults if missing)
  #   2. Aggregates four hard counters: totalBlocks, totalBlockedBy, totalDependsOn, totalRelatesTo, each incremented by the length of the matching dependency array per work unit
  #   3. Aggregates four per-unit boolean counters: workUnitsBlockingOthers (has blocks), workUnitsWithBlockers (has blockedBy), workUnitsWithSoftDependencies (has dependsOn), workUnitsWithDependencies (has any of the four arrays)
  #   4. averageDependenciesPerUnit = sum-of-four-totals / workUnitCount, rounded to two decimal places via Math.round(x*100)/100; returns 0 when work unit count is zero
  #   5. maxDependencyChainDepth is the deepest blocks chain across all work units, calculated by DFS that clones the visited set per branch and short-circuits visited nodes by returning 0
  #   6. Chain depth contributes +1 only when wu.blocks is non-empty; a work unit with blocks=[] adds nothing, while blocks pointing to a missing id still adds 1
  #   7. Result JSON field order is declaration order, not alphabetical
  #   8. Dispatcher path always returns 2-space pretty-printed JSON
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust port of query-dependency-stats wired through both the LLM dispatcher and the clap subcommand
    So that the fspec daemon and the standalone Rust binary share one aggregation implementation with byte-parity to the TS source

  Scenario: Returns all-zero stats when work-units.json is auto-created in an empty workspace
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch query-dependency-stats with format='json'
    Then the dispatcher returns success=true
    Then the returned JSON has totalBlocks=0, totalBlockedBy=0, totalDependsOn=0, totalRelatesTo=0
    Then the returned JSON has workUnitsWithDependencies=0, workUnitsWithBlockers=0, workUnitsBlockingOthers=0, workUnitsWithSoftDependencies=0
    Then the returned JSON has averageDependenciesPerUnit=0 and maxDependencyChainDepth=0
    Then spec/work-units.json exists after the call (auto-created by ensure_work_units_file)

  Scenario: Aggregates totals across all four dependency arrays
    Given spec/work-units.json contains A whose blocks=['B'], blockedBy=[], dependsOn=['C'], relatesTo=['D']
    Given spec/work-units.json also contains B (no dependency fields), C (no dependency fields), D (no dependency fields)
    When I dispatch query-dependency-stats with format='json'
    Then the returned JSON has totalBlocks=1, totalBlockedBy=0, totalDependsOn=1, totalRelatesTo=1
    Then the returned JSON has workUnitsBlockingOthers=1, workUnitsWithBlockers=0, workUnitsWithSoftDependencies=1
    Then the returned JSON has workUnitsWithDependencies=1

  Scenario: Two-unit A-blocks-B graph reports chain depth 1
    Given spec/work-units.json contains A with blocks=['B'] and B with no dependency fields
    When I dispatch query-dependency-stats with format='json'
    Then the returned JSON has totalBlocks=1
    Then the returned JSON has maxDependencyChainDepth=1
    Then the returned JSON has averageDependenciesPerUnit=0.5

  Scenario: Three-unit linear chain A->B->C reports chain depth 2
    Given spec/work-units.json contains A with blocks=['B'], B with blocks=['C'], C with no dependency fields
    When I dispatch query-dependency-stats with format='json'
    Then the returned JSON has totalBlocks=2
    Then the returned JSON has maxDependencyChainDepth=2

  Scenario: Self-cycle on blocks yields chain depth 1 not infinite recursion
    Given spec/work-units.json contains A with blocks=['A']
    When I dispatch query-dependency-stats with format='json'
    Then the dispatcher returns success=true
    Then the returned JSON has totalBlocks=1
    Then the returned JSON has maxDependencyChainDepth=1

  Scenario: blocks pointing to a missing work unit still contributes +1 to chain depth
    Given spec/work-units.json contains A with blocks=['NONEXISTENT']
    When I dispatch query-dependency-stats with format='json'
    Then the returned JSON has totalBlocks=1
    Then the returned JSON has maxDependencyChainDepth=1

  Scenario: Empty blocks array does not bump workUnitsBlockingOthers
    Given spec/work-units.json contains A with blocks=[] (empty array)
    When I dispatch query-dependency-stats with format='json'
    Then the returned JSON has totalBlocks=0
    Then the returned JSON has workUnitsBlockingOthers=0
    Then the returned JSON has workUnitsWithDependencies=0
    Then the returned JSON has maxDependencyChainDepth=0

  Scenario: workUnitsWithDependencies counts each unit only once even with multiple populated arrays
    Given spec/work-units.json contains a single work unit A whose blocks=['X'], blockedBy=['Y'], dependsOn=['Z'], relatesTo=['W']
    When I dispatch query-dependency-stats with format='json'
    Then the returned JSON has totalBlocks=1, totalBlockedBy=1, totalDependsOn=1, totalRelatesTo=1
    Then the returned JSON has workUnitsBlockingOthers=1, workUnitsWithBlockers=1, workUnitsWithSoftDependencies=1
    Then the returned JSON has workUnitsWithDependencies=1

  Scenario: Integer-valued average serializes without a decimal point
    Given spec/work-units.json contains three units where total dependency count divides exactly to integer 1
    When I dispatch query-dependency-stats with format='json'
    Then the returned JSON text contains the line '  "averageDependenciesPerUnit": 1,'
    Then the returned JSON text does NOT contain the substring 'averageDependenciesPerUnit": 1.0'

  Scenario: Non-integer average serializes as decimal 0.5
    Given spec/work-units.json contains exactly two units, one with blocks=['X'] and one with no dependency fields
    When I dispatch query-dependency-stats with format='json'
    Then the returned JSON text contains the line '  "averageDependenciesPerUnit": 0.5,'

  Scenario: JSON field order matches TS declaration order exactly
    Given spec/work-units.json contains a single unit A with blocks=['X']
    When I dispatch query-dependency-stats with format='json'
    Then the returned JSON's field declaration order is totalBlocks, totalBlockedBy, totalDependsOn, totalRelatesTo, workUnitsWithDependencies, workUnitsWithBlockers, workUnitsBlockingOthers, workUnitsWithSoftDependencies, averageDependenciesPerUnit, maxDependencyChainDepth

  Scenario: Escalates malformed work-units.json as a structured parse error
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch query-dependency-stats against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
