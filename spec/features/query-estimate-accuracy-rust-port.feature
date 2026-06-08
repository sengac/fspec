@done
@RPC-258 @cli @querying @wip
Feature: Port query-estimate-accuracy command to Rust

  """
  This feature file describes the LLM-facing dispatcher behaviour of the
  Rust port of the `query-estimate-accuracy` command (RPC-258, child of
  RPC-003). The dispatcher front-door exposes the full TS function shape:
  workUnitId, byPrefix, and format args. The CLI front-door is covered
  by spec/features/query-estimate-accuracy-cli-subcommand.feature.

  Architecture notes:
    - Reuses crate::io::ensure::read_work_units_or_empty for spec/work-units.json reads
    - Parse errors are re-wrapped with the 'Failed to query estimate accuracy:' prefix
      to honour TS outer try/catch behaviour
    - Aggregator uses IndexMap<String, ...> to preserve TS Object.keys() insertion order
    - avgIterations = ((sum/count) * 10.0).round() / 10.0; serialized as f64
      so 2.0 prints as `2` and 1.5 prints as `1.5` (TS template-literal parity)
    - WorkUnit estimate / iterations / metrics.iterations fields are read from the
      `extra` map via serde_json::Value access (TS [key: string]: unknown indexer parity)
    - Both invocation paths (LLM dispatcher AND standalone clap binary) call the
      SAME fspec_core::commands::query_estimate_accuracy::run function
  """

  Background: User Story
    As a fspec maintainer
    I want to invoke the ported Rust `query-estimate-accuracy` command via both the LLM-facing dispatcher and the standalone fspec Rust binary
    So that the canonical TypeScript behaviour ships as a native Rust subcommand with byte-for-byte help and dispatcher parity

  Scenario: Returns an empty byStoryPoints object when spec/ does not exist and does not auto-create files
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch the query-estimate-accuracy command against that project root with format='json'
    Then the dispatcher returns success=true with a payload whose byStoryPoints object is empty
    And spec/work-units.json does not exist after the call

  Scenario: Escalates malformed work-units.json as a structured wrapped error
    Given spec/work-units.json exists but contains invalid JSON syntax
    When I dispatch the query-estimate-accuracy command against that project root
    Then the dispatcher returns success=false
    And the error message contains the substring 'Failed to query estimate accuracy:'
    And the error message contains the substring 'Failed to parse work-units.json'

  Scenario: Single work-unit query returns estimated, actual, and comparison fields
    Given spec/work-units.json contains the work unit AUTH-001 with estimate=5, iterations=2, and status='done'
    When I dispatch query-estimate-accuracy with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    And the payload field 'estimated' equals '5 points'
    And the payload field 'actual' equals '0 tokens, 2 iterations'
    And the payload field 'comparison' equals 'Within expected range'

  Scenario: Single work-unit query for unknown id returns wrapped not-found error
    Given spec/work-units.json contains no work unit with id 'MISSING-999'
    When I dispatch query-estimate-accuracy with workUnitId='MISSING-999'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Failed to query estimate accuracy:'
    And the error message contains the substring 'Work unit MISSING-999 not found'

  Scenario: Single work-unit query defaults missing estimate and iterations to zero
    Given spec/work-units.json contains the work unit BUG-001 with no estimate field and no iterations field
    When I dispatch query-estimate-accuracy with workUnitId='BUG-001' and format='json'
    Then the dispatcher returns success=true
    And the payload field 'estimated' equals '0 points'
    And the payload field 'actual' equals '0 tokens, 0 iterations'
    And the payload field 'comparison' equals 'Within expected range'

  Scenario: Single work-unit query reads iterations from metrics.iterations when root-level iterations is undefined
    Given spec/work-units.json contains the work unit AUTH-002 with no root-level iterations field but with metrics.iterations=7
    When I dispatch query-estimate-accuracy with workUnitId='AUTH-002' and format='json'
    Then the dispatcher returns success=true
    And the payload field 'actual' equals '0 tokens, 7 iterations'

  Scenario: All-completed aggregation buckets by story point with averaged iterations
    Given spec/work-units.json contains five done work units AUTH-001 (estimate=1 iterations=1), AUTH-002 (estimate=1 iterations=2), AUTH-003 (estimate=3 iterations=2), AUTH-004 (estimate=3 iterations=3), AUTH-005 (estimate=5 iterations=2)
    When I dispatch query-estimate-accuracy with format='json' and no workUnitId
    Then the dispatcher returns success=true
    And the byStoryPoints entry for '1' has avgIterations=1.5 and samples=2
    And the byStoryPoints entry for '3' has avgIterations=2.5 and samples=2
    And the byStoryPoints entry for '5' has avgIterations=2 and samples=1

  Scenario: All-completed aggregation excludes work units missing estimate or iterations
    Given spec/work-units.json contains the done work unit AUTH-100 with estimate=3 but no iterations field
    And spec/work-units.json also contains the done work unit AUTH-101 with iterations=4 but no estimate field
    And spec/work-units.json also contains the done work unit AUTH-102 with estimate=2 and iterations=3
    When I dispatch query-estimate-accuracy with format='json' and no workUnitId
    Then the byStoryPoints object has exactly one key '2'
    And the byStoryPoints entry for '2' has avgIterations=3 and samples=1

  Scenario: byPrefix=true groups by id prefix and reports avgAccuracy and recommendation strings
    Given spec/work-units.json contains AUTH-001 (estimate=5 iterations=2 status=done), AUTH-002 (estimate=3 iterations=4 status=done), SEC-001 (estimate=5 iterations=3 status=done)
    When I dispatch query-estimate-accuracy with byPrefix=true and format='json'
    Then the dispatcher returns success=true
    And the byPrefix entry for 'AUTH' has avgAccuracy='3.0 avg iterations' and recommendation='2 samples'
    And the byPrefix entry for 'SEC' has avgAccuracy='3.0 avg iterations' and recommendation='1 sample'

  Scenario: byStoryPoints key iteration order follows first-encounter insertion order
    Given spec/work-units.json contains the done work units ZED-001 (estimate=5 iterations=1), AAA-001 (estimate=1 iterations=1), MID-001 (estimate=3 iterations=1) registered in that order
    When I dispatch query-estimate-accuracy with format='json' and no workUnitId
    Then the byStoryPoints keys appear in the order '5', '1', '3' in the serialized payload

  Scenario: JSON dispatcher payload is pretty-printed with 2-space indent and canonical field order
    Given spec/work-units.json contains the done work unit AUTH-001 (estimate=5 iterations=2)
    When I dispatch query-estimate-accuracy with byPrefix=true and format='json'
    Then the dispatcher returns success=true
    And the payload string contains a line that starts with two spaces followed by '"byStoryPoints"'
    And the payload string contains a line that starts with two spaces followed by '"byPrefix"'
    And the byStoryPoints field appears before the byPrefix field in the payload string

  Scenario: All-completed aggregation against missing work-units.json returns empty byStoryPoints
    Given spec/work-units.json does not exist in the project root
    When I dispatch query-estimate-accuracy with format='json' and no workUnitId
    Then the dispatcher returns success=true
    And the payload field 'byStoryPoints' is an empty object
    And spec/work-units.json was NOT created in the project root

  Scenario: Both invocation paths call the same fspec_core::commands::query_estimate_accuracy::run function
    Given a project root whose spec/work-units.json contains the done work unit AUTH-001 (estimate=5 iterations=2)
    When I dispatch query-estimate-accuracy through fspec_core::dispatch::dispatch_command with format='json'
    Then the dispatcher returns success=true
    And the dispatcher payload contains a byStoryPoints entry for '5' with samples=1
    And the CLI bridge module codelet/fspec/src/query_estimate_accuracy.rs contains no inline aggregation or rendering logic
