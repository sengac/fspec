@done
@bulk-operations
@parser
@RPC-220
Feature: Port delete-scenarios command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/delete_scenarios.rs reuses crate::io::feature_glob::glob_feature_files for the recursive spec/features walk and crate::io::gherkin::parse_feature_lenient for parsing. It matches scenarios with AND logic on SCENARIO-level tags (every supplied tag must be present, compared with leading @), computes each scenario's lineStart (first tag line, else keyword line) and lineEnd (next scenario/background start, else EOF), and returns the JSON envelope {success, deletedCount, fileCount, message?, scenarios?, error?}. Dry-run reports without modifying. Real delete splices matching scenario line ranges bottom-up, collapses 4+ blank lines to 3, re-parses (validation failure → file unmodified, success=false), writes the file, and updates the .feature.coverage sidecar (remove deleted scenario names, recalc stats). The CLI bridge owns all rendering. delete-scenarios has NO matching custom -help.ts (the file maps to command name delete-scenarios-by-tag), so its --help is bare Commander.js, hard-coded as DELETE_SCENARIOS_HELP in main.rs (mirrors delete-features).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. At least one tag is required; an empty/missing tag list returns success=false with error 'At least one --tag is required' (CLI prints 'Error: ...' to stderr, exit 1)
  #   2. Scenarios are matched with AND logic on SCENARIO-level tags (every supplied tag present, compared with leading @); feature-level tags are ignored
  #   3. No feature files → success=true, deletedCount=0, message 'No feature files found'; files but no match → success=true, deletedCount=0, message 'No scenarios found matching tags'
  #   4. With dryRun=true NO files are modified; returns success=true, deletedCount=N, fileCount=M, message 'Would delete N scenario(s) from M file(s)', plus scenarios array of {file,name,tags,lineStart,lineEnd}
  #   5. Without dryRun, matching scenarios are spliced bottom-up; 4+ blank lines collapse to 3; result MUST re-parse as valid Gherkin or the file is unmodified and success=false; .feature.coverage sidecars have deleted scenarios removed and stats recalculated
  #   6. CLI dry-run prints 'Dry run mode - no files modified', a 'Would delete N scenario(s) from M file(s):' header, then per file '<file>:' and each scenario '  - <name> (<tags>)'; real delete prints '✓ <message>'; exit 0
  #
  # EXAMPLES:
  #   1. Dispatcher tags=['@spike'] over a feature with two @spike scenarios and one untagged: dry-run returns deletedCount=2, fileCount=1, file unchanged
  #   2. Dispatcher tags=['@spike'] real delete removes the two @spike scenarios, keeps the untagged one, deletedCount=2, file still valid
  #   3. Dispatcher tags=['@deprecated','@critical'] only matches a scenario carrying BOTH tags
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to bulk-delete scenarios whose scenario-level tags match ALL supplied tags (with a dry-run preview) across feature files via both the LLM dispatcher and the shell CLI
    So that I can clean up obsolete or prototype scenarios with byte-for-byte parity to the TypeScript implementation without relying on Node.js

  Scenario: Dispatcher dry-run reports matches without modifying files
    Given a project root tempdir with one feature containing two @spike scenarios and one untagged scenario
    When I dispatch delete-scenarios with tags=['@spike'] and dryRun=true
    Then the dispatcher returns success=true with deletedCount=2 and fileCount=1
    And the dispatcher scenarios array lists the two matching scenarios
    And the feature file is unchanged on disk

  Scenario: Dispatcher real delete removes matching scenarios and keeps the rest
    Given a project root tempdir with one feature containing two @spike scenarios and one untagged scenario
    When I dispatch delete-scenarios with tags=['@spike']
    Then the dispatcher returns success=true with deletedCount=2
    And the feature file no longer contains the @spike scenarios
    And the feature file still contains the untagged scenario
    And the feature file re-parses as valid Gherkin

  Scenario: Dispatcher applies AND logic across multiple tags
    Given a project root tempdir with one feature containing a scenario tagged @deprecated and @critical and a scenario tagged only @deprecated
    When I dispatch delete-scenarios with tags=['@deprecated','@critical']
    Then the dispatcher returns success=true with deletedCount=1
    And only the scenario carrying both tags is removed

  Scenario: Dispatcher reports no feature files
    Given a project root tempdir with no spec/features directory
    When I dispatch delete-scenarios with tags=['@spike']
    Then the dispatcher returns success=true with deletedCount=0
    And the dispatcher message equals 'No feature files found'

  Scenario: Dispatcher reports no matching scenarios
    Given a project root tempdir with one feature containing only untagged scenarios
    When I dispatch delete-scenarios with tags=['@spike']
    Then the dispatcher returns success=true with deletedCount=0
    And the dispatcher message equals 'No scenarios found matching tags'

  Scenario: Dispatcher rejects an empty tag list
    Given a project root tempdir with one feature tagged @spike
    When I dispatch delete-scenarios with tags=[]
    Then the dispatcher returns success=false
    And the dispatcher error equals 'At least one --tag is required'
