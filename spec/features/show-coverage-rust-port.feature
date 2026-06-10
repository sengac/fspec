@done
@coverage
@cli
@RPC-300
Feature: Port show-coverage command to Rust

  """
  The Rust port lives at codelet/fspec-core/src/commands/show_coverage.rs with the signature `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`. Both invocation paths (LLM dispatcher AND standalone CLI) call this single function — RPC-003 §7/§11 two-front-doors invariant.
  Coverage sidecar types live in a new shared module codelet/fspec-core/src/types/coverage.rs that mirrors src/utils/coverage-file.ts: CoverageFile { scenarios: Vec<CoverageScenario>, stats: Option<CoverageStats> }, CoverageScenario { name, test_mappings }, TestMapping { file, lines: String, impl_mappings: Vec<ImplMapping> }, ImplMapping { file, lines: ImplLines (untagged enum of Vec<u32> | String) }, CoverageStats { total_scenarios, covered_scenarios, coverage_percent, test_files, impl_files, total_lines_covered }. All structs carry #[serde(rename_all = "camelCase")] and #[serde(flatten)] extra: serde_json::Map<String, Value> to preserve unknown fields.
  Coverage status is computed by the helper get_coverage_status: returns FullyCovered (✅) when any testMapping has ≥1 implMapping; PartiallyCovered (⚠️) when testMappings present but none have implMappings; Uncovered (❌) when testMappings is empty.
  Markdown rendering MUST be done via explicit `lines.push(...)` followed by `lines.join("\n")`, mirroring TS exactly. JSON rendering MUST use #[derive(Serialize)] structs with explicit declaration-order fields — DO NOT route through json!{} which alphabetizes via BTreeMap.
  The async fn signature is preserved for dispatcher contract uniformity even though every branch resolves on first poll (only std::fs / serde_json used). poll_sync_future will handle dispatch under the sync runtime.
  Coverage file path resolution: `<project_root>/spec/features/<name>.feature.coverage` where <name> tolerates a trailing '.feature' (e.g. 'user-login' and 'user-login.feature' both resolve to user-login.feature.coverage).
  Errors leaving fspec-core MUST be FspecCoreError variants (Io, InvalidArgs). Caller-facing messages (TS-parity) are embedded in the reason string and prefixed by the CLI bridge.
  """

  Background: User Story
    As a fspec maintainer porting the show-coverage command to Rust
    I want to have a single Rust function (and its dual CLI/dispatcher front doors) that loads .feature.coverage JSON sidecars, enriches them with missing-file warnings, calculates stats from scenarios when legacy files omit them, and renders per-feature or project-wide reports as markdown or 2-space JSON
    So that the Rust agent loop no longer hangs on the show-coverage stub and shell users see byte-for-byte parity with the TypeScript source

  # =====================================================
  # Per-feature mode (positional feature-name supplied)
  # =====================================================

  Scenario: Bare feature name resolves to spec/features/<name>.feature.coverage and renders markdown report
    Given a temp project root contains spec/features/user-login.feature.coverage with 5 scenarios, 4 of which have testMappings with implMappings and 1 with no testMappings
    When I dispatch show-coverage with featureName='user-login' (no format)
    Then the call returns Ok(rendered_string)
    And the rendered string starts with the line '# Coverage Report: user-login.feature'
    And the rendered string contains the line '**Coverage**: 80% (4/5 scenarios)'

  Scenario: Trailing .feature on the bare name is tolerated
    Given a temp project root contains spec/features/user-login.feature.coverage with 5 scenarios, 4 of which have testMappings with implMappings and 1 with no testMappings
    When I dispatch show-coverage with featureName='user-login.feature'
    Then the call returns Ok(rendered_string)
    And the rendered string is byte-equal to the call made with featureName='user-login'

  Scenario: Missing coverage file is reported as an Io error with TS-parity message
    Given a temp project root contains spec/features/ but no missing.feature.coverage
    When I dispatch show-coverage with featureName='missing'
    Then the call returns Err(FspecCoreError::Io)
    And the error's message contains 'Coverage file not found: missing.feature.coverage'
    And the error's message contains "Suggestion: Run 'fspec create-feature' to create the feature with coverage tracking"

  Scenario: Invalid JSON in the coverage file is reported as an InvalidArgs error
    Given a temp project root contains spec/features/broken.feature.coverage with the bytes '{ not json'
    When I dispatch show-coverage with featureName='broken'
    Then the call returns Err(FspecCoreError::InvalidArgs)
    And the error's message starts with 'Invalid JSON in coverage file: broken.feature.coverage'
    And the error's message contains 'Parse error:'
    And the error's message contains 'Suggestion: Validate the JSON or recreate the file'

  Scenario: Markdown summary section is emitted in the exact TS order
    Given a temp project root contains spec/features/feat.feature.coverage with 3 scenarios all fully covered (testMappings with implMappings)
    When I dispatch show-coverage with featureName='feat'
    Then the call returns Ok(rendered_string)
    And the rendered string contains a '## Summary' section
    And immediately under '## Summary' the lines appear in this order: 'Total Scenarios', 'Covered', 'Uncovered', 'Test Files', 'Implementation Files', 'Test Lines', 'Implementation Lines', 'Total Lines'

  Scenario: Per-scenario block uses ✅ / FULLY COVERED label for scenarios with implMappings
    Given a temp project root contains spec/features/feat.feature.coverage where scenario 'Login' has 1 testMapping with 1 implMapping referencing existing files
    When I dispatch show-coverage with featureName='feat'
    Then the rendered string contains the line '### ✅ Login (FULLY COVERED)'
    And the rendered string contains a '- **Test**: ' bullet for that scenario
    And the rendered string contains a '- **Implementation**: ' bullet for that scenario

  Scenario: Per-scenario block uses ⚠️ / PARTIALLY COVERED label when testMapping has no implMappings
    Given a temp project root contains spec/features/feat.feature.coverage where scenario 'Logout' has 1 testMapping but ZERO implMappings
    When I dispatch show-coverage with featureName='feat'
    Then the rendered string contains the line '### ⚠️ Logout (PARTIALLY COVERED)'
    And the rendered string contains the line '- **Implementation**: ⚠️  No implementation mappings'

  Scenario: Per-scenario block uses ❌ / UNCOVERED label and the file gains a Coverage Gaps section
    Given a temp project root contains spec/features/feat.feature.coverage where scenario 'Reset' has ZERO testMappings
    When I dispatch show-coverage with featureName='feat'
    Then the rendered string contains the line '### ❌ Reset (UNCOVERED)'
    And the rendered string contains the line '- No test mappings'
    And the rendered string contains a '## ⚠️  Coverage Gaps' section preceded by a '---' separator
    And the gaps section contains the bullet '- Reset'

  Scenario: Missing referenced test or impl files are surfaced as Warnings section but command still succeeds
    Given a temp project root contains spec/features/feat.feature.coverage referencing src/__tests__/deleted.test.ts as a testMapping file
    And the file src/__tests__/deleted.test.ts does NOT exist on disk
    When I dispatch show-coverage with featureName='feat'
    Then the call returns Ok(rendered_string)
    And the rendered string contains a '## Warnings' section
    And the rendered string contains the line '⚠️  File not found: src/__tests__/deleted.test.ts'

  Scenario: Legacy coverage file without a stats key has stats calculated silently from scenarios
    Given a temp project root contains spec/features/legacy.feature.coverage whose top-level JSON object omits the 'stats' key but has 4 scenarios all with testMappings
    When I dispatch show-coverage with featureName='legacy'
    Then the call returns Ok(rendered_string)
    And the rendered string contains the line '**Coverage**: 100% (4/4 scenarios)'

  Scenario: Legacy stats calculation deduplicates test and impl files and rounds coveragePercent with Math.round semantics
    Given a temp project root contains spec/features/legacy.feature.coverage with no stats key and 2 scenarios where scenario A has a testMapping referencing test1.ts and implMapping referencing impl1.ts and scenario B has no testMappings
    When I dispatch show-coverage with featureName='legacy' and format='json'
    Then the call returns Ok(rendered_string)
    And the rendered JSON's stats.testFiles equals ['test1.ts']
    And the rendered JSON's stats.implFiles equals ['impl1.ts']
    And the rendered JSON's stats.coveragePercent equals 50

  Scenario: Line counting accumulates test ranges and both array-and-string impl ranges
    Given a temp project root contains spec/features/feat.feature.coverage with one scenario whose testMapping.lines='45-62' and implMappings contain one with lines=[10,11,12,23,24] and one with lines='1-149'
    When I dispatch show-coverage with featureName='feat'
    Then the rendered markdown contains the line '- Test Lines: 18'
    And the rendered markdown contains the line '- Implementation Lines: 154'
    And the rendered markdown contains the line '- Total Lines: 172'

  Scenario: JSON format for single-file mode emits 2-space-indented object with keys in declaration order
    Given a temp project root contains spec/features/feat.feature.coverage with 2 scenarios and a stats key
    When I dispatch show-coverage with featureName='feat' and format='json'
    Then the call returns Ok(rendered_string)
    And the rendered string parses as JSON
    And the rendered JSON's top-level keys in declaration order are 'fileName', 'scenarios', 'stats', 'warnings'
    And the rendered JSON's fileName equals 'feat.feature'
    And each entry in the rendered JSON's scenarios array has an appended 'coverageStatus' field whose value is one of 'fully-covered', 'partially-covered', 'uncovered'
    And the rendered string uses 2-space indentation

  Scenario: Single-file JSON omits warnings field when no files are missing
    Given a temp project root contains spec/features/feat.feature.coverage with one scenario referencing only files that exist on disk
    When I dispatch show-coverage with featureName='feat' and format='json'
    Then the rendered JSON's warnings field is null or omitted

  # =====================================================
  # Project-wide mode (no positional)
  # =====================================================

  Scenario: Project-wide mode aggregates totals and prints '# Project Coverage Report'
    Given a temp project root contains spec/features/a.feature.coverage with 2 scenarios both fully covered AND spec/features/b.feature.coverage with 2 scenarios, 1 fully covered and 1 uncovered
    When I dispatch show-coverage with no featureName
    Then the call returns Ok(rendered_string)
    And the rendered string starts with the line '# Project Coverage Report'
    And the rendered string contains the line '**Overall Coverage**: 75% (3/4 scenarios)'

  Scenario: Project Summary section appears in TS field order
    Given a temp project root contains spec/features/a.feature.coverage with 2 scenarios both fully covered AND spec/features/b.feature.coverage with 2 scenarios, 1 fully covered and 1 uncovered
    When I dispatch show-coverage with no featureName
    Then the rendered string contains a '## Project Summary' section
    And inside Project Summary the lines appear in this order: 'Total Features: 2', 'Total Scenarios: 4', 'Covered: 3', 'Uncovered: 1'

  Scenario: Features Overview uses ✅ for 100% coverage features
    Given a temp project root contains spec/features/full.feature.coverage where the stats.coveragePercent equals 100
    When I dispatch show-coverage with no featureName
    Then the rendered string contains the line '- full.feature: 100% (.*) ✅' (regex match)

  Scenario: Features Overview uses ⚠️ for ≥50% but <100% coverage features
    Given a temp project root contains spec/features/half.feature.coverage where the stats.coveragePercent equals 50
    When I dispatch show-coverage with no featureName
    Then the rendered string contains the substring 'half.feature: 50%'
    And the line containing 'half.feature: 50%' ends with the ⚠️ symbol

  Scenario: Features Overview uses ❌ for <50% coverage features including 0%
    Given a temp project root contains spec/features/none.feature.coverage where the stats.coveragePercent equals 0
    When I dispatch show-coverage with no featureName
    Then the rendered string contains the substring 'none.feature: 0%'
    And the line containing 'none.feature: 0%' ends with the ❌ symbol

  Scenario: Project-wide mode emits Detailed Coverage by Feature with per-feature scenario list
    Given a temp project root contains spec/features/a.feature.coverage with scenarios 'X' (covered) and 'Y' (uncovered)
    When I dispatch show-coverage with no featureName
    Then the rendered string contains a '---' separator followed by '## Detailed Coverage by Feature'
    And the rendered string contains the line '### a.feature'
    And the rendered string contains a per-scenario bullet '- ✅ X'
    And the rendered string contains a per-scenario bullet '- ❌ Y'

  Scenario: Project-wide mode silently skips coverage files whose JSON fails to parse
    Given a temp project root contains spec/features/good.feature.coverage with 1 fully covered scenario AND spec/features/bad.feature.coverage with the bytes '{ not json'
    When I dispatch show-coverage with no featureName
    Then the call returns Ok(rendered_string)
    And the rendered string contains 'good.feature'
    And the rendered string does NOT contain 'bad.feature'
    And the rendered string contains the line '**Overall Coverage**: 100% (1/1 scenarios)'

  Scenario: Project-wide mode errors when spec/features directory does not exist
    Given a temp project root with no spec/features directory
    When I dispatch show-coverage with no featureName
    Then the call returns Err(FspecCoreError::Io)
    And the error's message contains 'Features directory not found: spec/features/'
    And the error's message contains "Suggestion: Run 'fspec create-feature' to create your first feature"

  Scenario: Project-wide mode errors when spec/features exists but contains no .feature.coverage files
    Given a temp project root contains spec/features/ but no *.feature.coverage files
    When I dispatch show-coverage with no featureName
    Then the call returns Err(FspecCoreError::Io)
    And the error's message contains 'No coverage files found in spec/features/'
    And the error's message contains "Suggestion: Run 'fspec create-feature' to create features with coverage tracking"

  Scenario: Project-wide mode with zero scenarios across all features renders Overall Coverage 0% without NaN
    Given a temp project root contains spec/features/empty.feature.coverage whose scenarios array is empty
    When I dispatch show-coverage with no featureName
    Then the call returns Ok(rendered_string)
    And the rendered string contains the line '**Overall Coverage**: 0% (0/0 scenarios)'

  Scenario: Project-wide JSON format emits 2-space-indented object with declaration-order root keys
    Given a temp project root contains spec/features/a.feature.coverage with 2 scenarios and spec/features/b.feature.coverage with 1 scenario
    When I dispatch show-coverage with no featureName and format='json'
    Then the call returns Ok(rendered_string)
    And the rendered string parses as JSON
    And the rendered JSON's top-level keys in declaration order are 'aggregated', 'features'
    And the rendered JSON's aggregated field has keys in declaration order: 'totalFeatures', 'totalScenarios', 'coveredScenarios', 'coveragePercent'
    And the rendered JSON's features field is an array of objects with declaration-order keys 'fileName', 'coverage'
    And the rendered string uses 2-space indentation

  # =====================================================
  # Two-front-doors / shared infrastructure
  # =====================================================

  Scenario: args_json that fails to parse returns FspecCoreError::InvalidArgs
    Given an arbitrary project root directory
    When I call show_coverage::run with args_json='{ not valid json'
    Then the call returns Err(FspecCoreError::InvalidArgs)
    And the error's command field equals 'show-coverage'

  Scenario: Coverage sidecar types module is publicly accessible from the crate root
    Given the codelet/fspec-core crate is built
    When I inspect codelet/fspec-core/src/types/
    Then the module types::coverage exists and exposes CoverageFile, CoverageScenario, TestMapping, ImplMapping, CoverageStats
    And the CoverageFile struct uses #[serde(rename_all = "camelCase")] and preserves unknown fields via a flattened extra map
    And commands/show_coverage.rs no longer returns FspecCoreError::NotYetPorted
