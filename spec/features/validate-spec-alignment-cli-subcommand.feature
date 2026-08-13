@done
@validation
@cli
@RPC-323
Feature: validate-spec-alignment clap subcommand on the standalone fspec Rust binary
  """
  CLI surface for the `validate-spec-alignment` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern:
  - Shell argv         → clap → rust/fspec/src/validate_spec_alignment.rs → fspec_core::commands::validate_spec_alignment::run
  - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::validate_spec_alignment::run
  Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS `process.cwd()` default).
  The clap subcommand exposes a required positional <workUnitId> and an accepted-but-no-op --fix (mirroring the real exported contract; the TS CLI action handler is broken).
  Valid → stdout '✓ ...' exit 0; invalid → warnings to stderr, exit 1; error → stderr prefixed 'Error:' exit 1.
  --help is byte-for-byte identical to the captured TS fixture at rust/fspec/tests/fixtures/help/validate-spec-alignment.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The exported validateSpecAlignment({workUnitId, cwd}) is the real tested contract (returns {valid, warnings?}); the TS CLI action handler is broken (calls it with no workUnitId and reads non-existent result.aligned), so the Rust port mirrors the exported function and the clap surface needs a workUnitId arg
  #   2. work-units.json is read DIRECTLY via readFile + JSON.parse (NOT ensureWorkUnitsFile); ENOENT and parse errors are caught and re-thrown wrapped as 'Failed to validate spec alignment: <msg>'
  #   3. If data.workUnits[workUnitId] is missing, throws 'Work unit <id> not found' (wrapped by the catch into 'Failed to validate spec alignment: Work unit <id> not found')
  #   4. Globs spec/features/**/*.feature; a scenario counts toward the work unit when a line trim-contains '@<workUnitId>' and the immediately following line trim-starts-with 'Scenario:'
  #   5. Returns {valid:true} when scenariosFound > 0; returns {valid:false, warnings:['No scenarios for <id>']} when scenariosFound === 0
  #   6. A missing spec/features directory yields zero scenarios (TS glob returns [] when the directory is absent — no throw); the Rust port must treat a missing features dir as an empty file list
  #   7. CLI exit codes: 0 when valid, 1 when invalid (warnings printed to stderr) or on any error
  #
  # EXAMPLES:
  #   1. work-units.json has AUTH-001 and one feature file with '@AUTH-001' on the line before a 'Scenario:' -> {valid:true}
  #   2. work-units.json has AUTH-001 but no feature scenario tagged @AUTH-001 -> {valid:false, warnings:['No scenarios for AUTH-001']}
  #   3. workUnitId='MISSING-999' not present in work-units.json -> error 'Failed to validate spec alignment: Work unit MISSING-999 not found'
  #   4. spec/features absent but AUTH-001 exists -> zero scenarios found -> {valid:false, warnings:['No scenarios for AUTH-001']}
  #
  # QUESTIONS (ANSWERED):
  #   Q: @supervisor: TS help advertises positional [feature-files...] + --fix but the EXPORTED/tested function takes {workUnitId}. Confirm the Rust clap surface should expose a positional <workUnitId> (mirroring the real exported contract) rather than the broken help-advertised shape. Also confirm whether a soft glob helper (returns [] when spec/features missing) is acceptable to add locally in this command vs a shared io/feature_glob helper.
  #   A: Working assumption pending supervisor confirmation: the clap surface exposes a required positional <workUnitId> (mirroring the real exported contract), with --fix accepted but no-op (parity with TS). A soft glob is handled locally by mapping DirectoryNotFound to an empty Vec; no new shared io helper is required.
  #
  # ASSUMPTIONS:
  #   1. Working assumption pending supervisor confirmation: the clap surface exposes a required positional <workUnitId> (mirroring the real exported contract), with --fix accepted but no-op (parity with TS). A soft glob is handled locally by mapping DirectoryNotFound to an empty Vec; no new shared io helper is required.
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust port of validate-spec-alignment wired through both the LLM dispatcher and the clap subcommand
    So that the standalone Rust binary and the daemon share one spec-alignment check with byte-parity to the TS exported function

  Scenario: Clap exposes validate-spec-alignment as a subcommand and prints --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec validate-spec-alignment --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'validate-spec-alignment'

  Scenario: CLI exits 0 and prints success when the work unit has tagged scenarios
    Given a workspace whose spec/work-units.json contains AUTH-001 and a feature file with '@AUTH-001' before a 'Scenario:' line
    When I run `./rust/target/release/fspec validate-spec-alignment AUTH-001` from that workspace
    Then the command exits 0
    And stdout contains the substring '✓'

  Scenario: CLI exits 1 and prints the warning when no scenarios are tagged
    Given a workspace whose spec/work-units.json contains AUTH-001 and no scenario tagged '@AUTH-001'
    When I run `./rust/target/release/fspec validate-spec-alignment AUTH-001` from that workspace
    Then the command exits with code 1
    And stderr contains the substring 'No scenarios for AUTH-001'

  Scenario: CLI exits 1 when the work unit does not exist
    Given a workspace whose spec/work-units.json contains AUTH-001 but not MISSING-999
    When I run `./rust/target/release/fspec validate-spec-alignment MISSING-999` from that workspace
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Work unit MISSING-999 not found'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a workspace whose spec/work-units.json contains AUTH-001 and a feature file with '@AUTH-001' before a 'Scenario:' line
    When I dispatch validate-spec-alignment through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' against that workspace
    And I run `./rust/target/release/fspec validate-spec-alignment AUTH-001` against the same workspace
    Then both invocations agree the work unit is valid
    And the CLI bridge module rust/fspec/src/validate_spec_alignment.rs contains NO inline scan logic — its only computation is JSON arg marshalling and stdout/stderr printing

  Scenario: validate-spec-alignment --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec validate-spec-alignment --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/validate-spec-alignment.txt
    And stdout starts with a blank line followed by 'VALIDATE-SPEC-ALIGNMENT'
