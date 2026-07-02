@done
@cli
@querying
@RPC-256
@wip
Feature: Port query-bottlenecks command to Rust
  """
  Use ensure_work_units_file from crate::io::ensure (auto-creating spec/work-units.json with canonical defaults if missing) — TS source-of-truth at src/commands/query-bottlenecks.ts:69 calls ensureWorkUnitsFile, so the Rust port matches that auto-create behaviour exactly.
  Read the blocks array from WorkUnit.extra via extra.get("blocks").and_then(Value::as_array) — do NOT add typed dependency fields to shared types/work_unit.rs (file-ownership rule: shared types module is supervisor-only). Mirror the dependency-stats port pattern.
  Bottleneck struct uses #[derive(Serialize)] with declaration-order fields {id, title, status, score, directBlocks, transitiveBlocks} and #[serde(rename_all = "camelCase")] to preserve TS JSON.stringify field order. Do NOT route through json!{} which alphabetizes via BTreeMap.
  calculate_blocked_work_units DFS uses HashSet<String> for visited; clones the set on each recursive descent (parity with TS new Set(visited)); collects results into an IndexSet to preserve direct-then-transitive (JS Set insertion) order.
  Sort by descending score using sort_by_key(|b| Reverse(b.score)); Rust's stable sort preserves the IndexMap iteration order for ties (parity with TS Array.prototype.sort).
  Both invocation paths (LLM dispatcher and clap subcommand) call the single fspec_core::commands::query_bottlenecks::run function; CLI bridge does only JSON arg marshalling and stdout rendering.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Loads spec/work-units.json via ensure_work_units_file (auto-creates canonical empty store on ENOENT, escalates malformed JSON via ParseJson)
  #   2. Iterates work units in IndexMap insertion order and skips any unit whose status is 'done' or 'blocked' OR whose blocks array is missing/empty
  #   3. For each candidate unit, calculate the transitive closure of blocked units via DFS over the blocks adjacency with a per-branch cloned visited set
  #   4. score = total size of the blocked-units set (direct + transitive deduped); only units with score >= 2 are included as bottlenecks
  #   5. directBlocks = copy of the unit's blocks array; transitiveBlocks = blocked set minus direct (insertion order preserved)
  #   6. Bottlenecks are sorted by descending score (stable sort preserves iteration-order tie-breaking)
  #   7. JSON output is pretty-printed with declaration-order fields
  #   8. Text output (default) emits a multi-line rendering with '✓ No bottlenecks found' when the bottlenecks array is empty
  #   9. CLI exit codes: 0 on success, 1 on any FspecCoreError with stderr prefixed 'Error:'
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust port of query-bottlenecks wired through both the LLM dispatcher and the clap subcommand
    So that the fspec daemon and the standalone Rust binary share one bottleneck-detection implementation with byte-parity to the TS source

  Scenario: Returns empty bottlenecks array when work-units.json is auto-created in an empty workspace
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch query-bottlenecks with output='json'
    Then the dispatcher returns success=true
    And the returned JSON has bottlenecks=[]
    And spec/work-units.json exists after the call (auto-created by ensure_work_units_file)

  Scenario: Excludes work units in done status even when they have blocks
    Given spec/work-units.json contains A with status='done' and blocks=['B','C']
    And spec/work-units.json also contains B with blocks=['D'] and C with no dependency fields and D with no dependency fields
    When I dispatch query-bottlenecks with output='json'
    Then the returned JSON has bottlenecks containing no entry whose id='A'

  Scenario: Excludes work units in blocked status even when they have blocks
    Given spec/work-units.json contains A with status='blocked' and blocks=['B','C']
    And spec/work-units.json also contains B and C with no dependency fields
    When I dispatch query-bottlenecks with output='json'
    Then the returned JSON has bottlenecks containing no entry whose id='A'

  Scenario: Excludes work units whose blocks array is empty or missing
    Given spec/work-units.json contains A with blocks=[] (empty array) and B with no blocks field
    When I dispatch query-bottlenecks with output='json'
    Then the returned JSON has bottlenecks=[]

  Scenario: Single direct block does not meet threshold of 2
    Given spec/work-units.json contains A with status='backlog' and blocks=['B'] and B with no dependency fields
    When I dispatch query-bottlenecks with output='json'
    Then the returned JSON has bottlenecks=[] (score 1 is below threshold)

  Scenario: Two direct blocks with one transitive yields score 3 and qualifies
    Given spec/work-units.json contains A with status='backlog' and blocks=['B','C']
    And spec/work-units.json also contains B with blocks=['D'] and C with no dependency fields and D with no dependency fields
    When I dispatch query-bottlenecks with output='json'
    Then the returned JSON has exactly one bottleneck whose id='A'
    And that bottleneck has score=3
    And that bottleneck has directBlocks=['B','C']
    And that bottleneck has transitiveBlocks=['D']

  Scenario: Cycle A blocks B and B blocks A yields score 2 for A
    Given spec/work-units.json contains A with status='backlog' and blocks=['B']
    And spec/work-units.json also contains B with status='backlog' and blocks=['A']
    When I dispatch query-bottlenecks with output='json'
    Then the returned JSON has a bottleneck whose id='A' with score=2
    And that bottleneck has directBlocks=['B']
    And that bottleneck has transitiveBlocks=['A']

  Scenario: Two qualifying bottlenecks are ranked by descending score
    Given spec/work-units.json contains A with blocks=['B','C','D'] producing transitive blocks for total score 5
    And spec/work-units.json contains E with blocks=['F','G','H'] producing total score 3
    When I dispatch query-bottlenecks with output='json'
    Then the returned JSON has bottlenecks[0].id='A' with score=5
    And bottlenecks[1].id='E' with score=3

  Scenario: JSON field declaration order is preserved
    Given spec/work-units.json contains a single qualifying bottleneck A blocking B and C
    When I dispatch query-bottlenecks with output='json'
    Then the bottleneck object's field declaration order is id, title, status, score, directBlocks, transitiveBlocks

  Scenario: Escalates malformed work-units.json as a structured parse error
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch query-bottlenecks against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
