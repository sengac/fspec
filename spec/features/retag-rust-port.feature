@wip
@RPC-293
@cli
@tag-management
Feature: Port retag command to Rust
  """
  Core impl rust/fspec-core/src/commands/retag.rs ports src/commands/retag.ts FAITHFULLY: validates from/to + tag-format regex, globs spec/features via io::feature_glob::glob_feature_files, counts/replaces whole-word occurrences with regex (^|\s)from(?=\s|$), re-parses replaced content via io::gherkin::parse_feature_lenient before writing, supports dryRun. Signature pub async fn run(args_json,&Path). Returns envelope {success, fileCount, occurrenceCount, message?, files?, error?} like delete_features.rs.
  DIVERGENCE FROM BRIEF: worker brief said retag also updates spec/tags.json registry; the actual TS source touches ONLY feature files, never tags.json. We port to TS source = feature files only. Flagged for supervisor.
  SURFACE DIVERGENCE: TS Commander.js registration uses FLAG options --from <tag> / --to <tag> / --dry-run (NOT positional args), despite retag-help.ts documenting positional <old-tag> <new-tag>. Runtime registration is canon -> Rust clap uses --from/--to/--dry-run. Help fixture captured from node dist/index.js retag --help reflects the actual flag surface. SUPERVISOR: confirm whether to mirror help-doc positional or runtime flags; recommendation = runtime flags since CLI uses results (Framing A inverts only when CLI discards results).
  CLI bridge rust/fspec/src/retag.rs marshals {from,to,dryRun} JSON, NO domain logic. Renders: failure -> 'Error: <error>' exit 1; dryRun -> 'Dry run mode - no files modified' + summary + file list exit 0; real with files -> '✓ <message>' + 'Modified files:' + list exit 0. Help config help/configs/retag.rs uses CommandHelpConfig (CommonError type). Uses existing regex crate dep (no new Cargo deps).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When --from or --to is missing the command fails with error 'Both --from and --to are required'
  #   2. When --to does not match @[a-z0-9-#]+ the command fails with 'Invalid tag format: "<to>". Valid format is @lowercase-with-hyphens'
  #   3. Tag occurrences are matched as whole words using the regex (^|\s)<from>(?=\s|$) per line and replaced preserving the leading whitespace
  #   4. When no feature files contain the from tag the command fails with 'Tag <from> not found in any feature files'; when there are zero feature files it succeeds with message 'No feature files found'
  #   5. With --dry-run no files are modified and the result reports fileCount, occurrenceCount and the list of matching files
  #   6. On a real run each modified file is re-parsed as Gherkin before writing; if any re-parse fails the command aborts with 'Validation failed after renaming in <file>: <msg>' and stops further writes
  #   7. retag mutates ONLY spec/features/**/*.feature files via text replacement (it does NOT read or write spec/tags.json), matching the TS source exactly
  #
  # EXAMPLES:
  #   1. Given two feature files containing @wip, dispatching retag from='@wip' to='@in-progress' renames every occurrence and reports 2 files changed
  #   2. Given feature files containing @wip, dispatching retag with dryRun=true reports the matching files and counts but leaves every file byte-equal
  #   3. Given a missing --to, dispatching retag fails with 'Both --from and --to are required'
  #   4. Given to='WIP' without a leading @, dispatching retag fails with the Invalid tag format message
  #   5. Given no feature file contains @missing, dispatching retag from='@missing' fails with 'Tag @missing not found in any feature files'
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the retag command ported to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can both bulk-rename a tag across all feature files without falling back to the TS implementation

  Scenario: Renaming a tag across two feature files rewrites every occurrence
    Given a project root tempdir with two spec/features feature files that each tag a scenario with @wip
    When I dispatch retag with from='@wip' and to='@in-progress'
    Then the dispatcher returns success=true
    And the result reports fileCount=2
    And neither feature file on disk contains the token '@wip' anymore
    And both feature files on disk now contain the token '@in-progress'

  Scenario: A dry run reports matches but leaves every file byte-equal
    Given a project root tempdir with two spec/features feature files that each tag a scenario with @wip
    When I dispatch retag with from='@wip', to='@in-progress' and dryRun=true
    Then the dispatcher returns success=true
    And the result reports fileCount=2 and a non-zero occurrenceCount
    And the result files array lists both matching feature files
    And both feature files on disk are byte-equal to their pre-call contents

  Scenario: A missing --to is rejected
    Given a project root tempdir with one spec/features feature file tagged @wip
    When I dispatch retag with from='@wip' and an empty to
    Then the dispatcher returns success=false
    And the error message is 'Both --from and --to are required'
    And the feature file on disk is byte-equal to its pre-call contents

  Scenario: An invalid target tag format is rejected
    Given a project root tempdir with one spec/features feature file tagged @wip
    When I dispatch retag with from='@wip' and to='WIP'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid tag format: "WIP". Valid format is @lowercase-with-hyphens'
    And the feature file on disk is byte-equal to its pre-call contents

  Scenario: A from tag present in no feature file reports the not-found error
    Given a project root tempdir with one spec/features feature file tagged @wip
    When I dispatch retag with from='@missing' and to='@found'
    Then the dispatcher returns success=false
    And the error message is 'Tag @missing not found in any feature files'
    And the feature file on disk is byte-equal to its pre-call contents

  Scenario: Rename feature-level tag
    Given a project root tempdir with a feature file tagged @legacy-api at feature level
    When I dispatch retag with from='@legacy-api' and to='@legacy'
    Then the dispatcher returns success=true
    And the feature-level tag should change from @legacy-api to @legacy
    And the file structure should be preserved

  Scenario: Rename scenario-level tag
    Given a project root tempdir with a feature file whose scenarios are tagged with @wip under a feature-level @api tag
    When I dispatch retag with from='@wip' and to='@in-progress'
    Then the dispatcher returns success=true
    And all scenario-level @wip tags should change to @in-progress
    And feature-level tags should remain unchanged

  Scenario: Rename tag used at both feature and scenario levels
    Given a project root tempdir with a feature file tagged @temp at feature level
    And a scenario in that file tagged @temp at scenario level
    When I dispatch retag with from='@temp' and to='@temporary'
    Then the dispatcher returns success=true
    And all @temp tags should change to @temporary at both levels
    And the result reports occurrenceCount=2

  Scenario: Rename tag with special characters
    Given a project root tempdir with a feature file tagged @bug-#123
    When I dispatch retag with from='@bug-#123' and to='@issue-123'
    Then the dispatcher returns success=true
    And all @bug-#123 tags should change to @issue-123
    And the file should remain valid Gherkin

  Scenario: Rename tag preserves other tags
    Given a project root tempdir with a feature tagged @release-one @critical @api
    When I dispatch retag with from='@release-one' and to='@v1'
    Then the dispatcher returns success=true
    And the feature should have tags @v1 @critical @api
    And tag order should be preserved
    And other tags should remain unchanged

  Scenario: Rename tag validates Gherkin after changes
    Given a project root tempdir with 20 feature files using @legacy-api
    When I dispatch retag with from='@legacy-api' and to='@legacy'
    Then the dispatcher returns success=true
    And all 20 files should have valid Gherkin syntax
    And the result message contains "All modified files validated successfully"

  Scenario: Rename tag with multiple occurrences in single file
    Given a project root tempdir with a feature file tagged @temp at feature level
    And 3 scenarios in that file tagged @temp at scenario level
    When I dispatch retag with from='@temp' and to='@temporary'
    Then the dispatcher returns success=true
    And all 4 occurrences should be renamed
    And the result message contains "4 occurrence(s)"
