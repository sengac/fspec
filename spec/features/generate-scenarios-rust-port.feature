@done
@rust
@feature-management
@cli
@RPC-234
Feature: Port generate-scenarios command to Rust

  """
  File layout: core impl codelet/fspec-core/src/commands/generate_scenarios.rs (rewrite stub, signature gains project_root); help config codelet/fspec-core/src/help/configs/generate_scenarios.rs; CLI bridge codelet/fspec/src/generate_scenarios.rs; core tests codelet/fspec-core/tests/generate_scenarios.rs; CLI tests codelet/fspec/tests/cli_generate_scenarios.rs; help fixture codelet/fspec/tests/fixtures/help/generate-scenarios.txt. Two feature files: generate-scenarios-rust-port.feature (dispatcher contract) + generate-scenarios-cli-subcommand.feature (clap surface).
  Reuse existing shared modules: io::ensure (ensureWorkUnitsFile parity), io::feature_glob::glob_feature_files, io::gherkin::parse_feature_lenient. Example-mapping fields (userStory, rules, examples, questions, assumptions, architectureNotes) are read out of WorkUnit.extra as serde_json::Value, mirroring TS untyped access — no new shared type module.
  NEW helper logic (step-extraction heuristics, the 5-algorithm hybrid similarity matcher ~430 LOC, prefill detection, and the verbatim system-reminder strings) is ported as PRIVATE modules inside the owned command file(s) to avoid touching shared types/mod.rs or new shared dirs.
  Async assessment: NONE. Pure blocking std::fs + glob + in-process gherkin parsing + regex/string CPU. No network, no child process, no real tokio .await — fully compatible with poll_sync_future. Only divergence: TS output.log warnings in the --ignore-possible-duplicates branch are folded into the returned String rather than written to stdout separately.
  SHARED-FILE CHANGES (supervisor, Phase C): (1) dispatch.rs generate-scenarios arm -> commands::generate_scenarios::run(args_json, project_root).await, remove run_stub arm; (2) canonical.rs add 'generate-scenarios' to PORTED_COMMANDS; (3) help/configs/mod.rs register pub mod generate_scenarios; (4) main.rs add mod generate_scenarios, Mode::GenerateScenarios{work_unit_id, feature, ignore_possible_duplicates} clap variant (positional <workUnitId>, --feature <name>, --ignore-possible-duplicates bool), forward! arm, and --help intercept arm; (5) commands/mod.rs stub already registered — verify only.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The command takes a required <workUnitId>, optional --feature <name>, and a boolean --ignore-possible-duplicates flag
  #   2. It auto-creates spec/work-units.json (ensureWorkUnitsFile) and throws "Work unit '<id>' does not exist" when the work unit is absent
  #   3. If any question is unanswered (!deleted && !selected) it throws "Cannot generate scenarios: N unanswered question(s) found" with singular/plural agreement
  #   4. If the work unit has neither rules nor examples it throws "Cannot generate scenarios: No Example Mapping data found"
  #   5. If there are no active (non-deleted) examples it throws "Work unit <id> has no examples to generate scenarios from"
  #   6. It scans existing spec/features/**/*.feature, parses each, and compares every active example (via heuristic step-extraction) against existing scenarios using an adaptive-threshold similarity matcher (base 0.7)
  #   7. When duplicate matches are found and --ignore-possible-duplicates is NOT set, it throws "Cannot generate scenarios: N duplicate scenarios detected above threshold" with a DUPLICATE SCENARIOS DETECTED system-reminder; with the flag set it proceeds and emits ignored-refactor warnings
  #   8. The feature file path is spec/features/<name>.feature where name is --feature (with any .feature suffix stripped) or the work unit title kebab-cased; if no title and no --feature it throws "Cannot determine feature file name"
  #   9. If the target feature file already exists it throws "Feature file <path> already exists" — generate-scenarios never overwrites
  #   10. The created file is context-only: @<id> tag, Feature: <title>, an architecture docstring (from categorized architectureNotes or a TODO placeholder), a # EXAMPLE MAPPING CONTEXT comment block, and a Background user story — with ZERO Scenario blocks (scenariosCount is always 0)
  #   11. After writing, it runs prefill detection on the file and returns systemReminders: a CONTEXT-ONLY FEATURE FILE CREATED reminder, a post-generation reminder, and a prefill reminder if placeholders remain — all merged via consolidateReminders into a single systemReminder block
  #   12. The CLI wrapper prints "✓ Created context-only feature file: <path>" then "  Contains example mapping context as comments (NO scenarios yet)" then the consolidated systemReminder, exiting 0; on a thrown error it prints "✗ Failed to generate scenarios:" with the message to stderr and exits 1
  #   13. Both front doors (LLM dispatcher JSON and standalone clap CLI) converge on the same fspec_core::commands::generate_scenarios::run(args_json, project_root) function; the CLI bridge does JSON marshalling only
  #   14. The Background is built from the userStory role/action/benefit when present, otherwise it emits placeholder tokens for those three fields that trigger prefill detection
  #
  # EXAMPLES:
  #   1. Dispatching generate-scenarios for a work unit with rules, examples and a user story creates a context-only feature file and returns success with scenariosCount 0
  #   2. Dispatching generate-scenarios for a missing work unit returns success=false with error containing "does not exist"
  #   3. Dispatching generate-scenarios for a work unit with an unanswered question returns success=false with error containing "unanswered question"
  #   4. Dispatching generate-scenarios for a work unit with no rules and no examples returns success=false with error containing "No Example Mapping data found"
  #   5. Dispatching generate-scenarios when the target feature file already exists returns success=false with error containing "already exists"
  #   6. Dispatching generate-scenarios when an active example matches an existing scenario above threshold (without --ignore-possible-duplicates) returns success=false with a DUPLICATE SCENARIOS DETECTED reminder
  #   7. Dispatching generate-scenarios with --feature=login writes spec/features/login.feature regardless of the work unit title
  #   8. Running `fspec generate-scenarios RPC-001 --feature=user-auth` from a shell prints "✓ Created context-only feature file:" and exits 0
  #   9. Running `fspec generate-scenarios MISSING-001` from a shell prints "✗ Failed to generate scenarios:" to stderr and exits 1
  #   10. Running `fspec generate-scenarios --help` prints help that is byte-for-byte identical to the captured TS fixture
  #   11. A work unit with a user story produces a Background populated from role, action and benefit, while a work unit without one emits placeholder tokens and a prefill reminder
  #
  # ========================================

  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to run `generate-scenarios <workUnitId>` in the Rust binary and have it behave byte-for-byte like the TypeScript command
    So that the context-only feature scaffold, duplicate detection, and system-reminders are identical across both front doors

  Scenario: Dispatch creates a context-only feature file for a complete work unit
    Given a project root tempdir whose work unit WU-1 has rules, an active example, and a user story
    When I dispatch generate-scenarios with workUnitId="WU-1"
    Then the dispatcher returns success=true
    Then a feature file is created under spec/features for WU-1 containing zero Scenario blocks
    Then the rendered output contains the substring "Created context-only feature file"
    Then the rendered output contains the substring "ZERO scenarios"


  Scenario: Dispatch fails for a missing work unit
    Given a project root tempdir with an empty work-units store
    When I dispatch generate-scenarios with workUnitId="MISSING-1"
    Then the dispatcher returns success=false
    Then the error message contains the substring "does not exist"


  Scenario: Dispatch fails when a question is unanswered
    Given a project root tempdir whose work unit WU-1 has an unanswered question
    When I dispatch generate-scenarios with workUnitId="WU-1"
    Then the dispatcher returns success=false
    Then the error message contains the substring "unanswered question"


  Scenario: Dispatch fails when there is no Example Mapping data
    Given a project root tempdir whose work unit WU-1 has no rules and no examples
    When I dispatch generate-scenarios with workUnitId="WU-1"
    Then the dispatcher returns success=false
    Then the error message contains the substring "No Example Mapping data found"


  Scenario: Dispatch fails when there are no active examples
    Given a project root tempdir whose work unit WU-1 has a rule but only deleted examples
    When I dispatch generate-scenarios with workUnitId="WU-1"
    Then the dispatcher returns success=false
    Then the error message contains the substring "has no examples to generate scenarios from"


  Scenario: Dispatch fails when the target feature file already exists
    Given a project root tempdir whose work unit WU-1 is ready and spec/features/wu-1.feature already exists
    When I dispatch generate-scenarios with workUnitId="WU-1"
    Then the dispatcher returns success=false
    Then the error message contains the substring "already exists"


  Scenario: Dispatch blocks on a duplicate scenario without the override flag
    Given a project root tempdir whose work unit WU-1 has an example that matches an existing scenario above threshold
    When I dispatch generate-scenarios with workUnitId="WU-1"
    Then the dispatcher returns success=false
    Then the error message contains the substring "DUPLICATE SCENARIOS DETECTED"


  Scenario: Dispatch proceeds past duplicates with ignore-possible-duplicates
    Given a project root tempdir whose work unit WU-1 has an example that matches an existing scenario above threshold
    When I dispatch generate-scenarios with workUnitId="WU-1" and ignorePossibleDuplicates=true
    Then the dispatcher returns success=true
    Then a feature file is created under spec/features for WU-1 containing zero Scenario blocks


  Scenario: Dispatch honours an explicit feature name
    Given a project root tempdir whose work unit WU-1 is ready with title "Some Other Title"
    When I dispatch generate-scenarios with workUnitId="WU-1" and feature="login"
    Then the dispatcher returns success=true
    Then the file spec/features/login.feature exists on disk


  Scenario: Background falls back to placeholder tokens without a user story
    Given a project root tempdir whose work unit WU-1 is ready but has no user story
    When I dispatch generate-scenarios with workUnitId="WU-1"
    Then the dispatcher returns success=true
    Then the created feature file Background contains role, action and benefit placeholder tokens
    Then the rendered output contains a prefill reminder


  Scenario: CLI and dispatcher converge on the same fspec_core run function
    Given a project root tempdir whose work unit WU-1 is ready
    When I dispatch generate-scenarios with workUnitId="WU-1" and also run the CLI subcommand fspec generate-scenarios WU-1 against an equivalent project root
    Then both paths produce output containing "Created context-only feature file"
    Then the CLI bridge module codelet/fspec/src/generate_scenarios.rs contains no analysis, gap-detection, or rendering logic — its only computation is JSON arg marshalling
