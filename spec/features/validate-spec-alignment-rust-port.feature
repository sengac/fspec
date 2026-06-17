@done
@validation
@cli
@RPC-323
Feature: Port validate-spec-alignment command to Rust

  """
  Core impl codelet/fspec-core/src/commands/validate_spec_alignment.rs: Args {work_unit_id: Option<String>, fix: Option<bool>} (camelCase workUnitId). work_unit_id None -> InvalidArgs. Reads spec/work-units.json directly as serde_json::Value (NOT ensure_*) so only .get('workUnits').get(id) is needed; ENOENT/parse wrap into 'Failed to validate spec alignment:' message.
  Feature globbing: reuse crate::io::feature_glob::glob_feature_files but map FspecCoreError::DirectoryNotFound to an empty Vec locally (TS glob returns [] when spec/features absent). Tag scan: for each file split lines, line.trim().contains('@<id>') && next.trim().starts_with('Scenario:') increments count.
  Result envelope {valid:bool, warnings?:Vec<String>} as JSON string. CLI bridge codelet/fspec/src/validate_spec_alignment.rs: valid -> stdout '✓ ...' exit 0; invalid -> warnings to stderr exit 1; error -> stderr 'Error:' exit 1. Help config codelet/fspec-core/src/help/configs/validate_spec_alignment.rs. Two-front-doors; dispatcher passes args_json verbatim.
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

  Scenario: Reports valid when at least one scenario is tagged with the work unit id
    Given spec/work-units.json contains AUTH-001
    And a feature file has a line '@AUTH-001' immediately followed by a line starting with 'Scenario:'
    When I dispatch validate-spec-alignment with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    And the returned JSON has valid=true
    And the returned JSON has no warnings field

  Scenario: Reports invalid with a warning when no scenario is tagged with the work unit id
    Given spec/work-units.json contains AUTH-001
    And no feature scenario is tagged '@AUTH-001'
    When I dispatch validate-spec-alignment with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    And the returned JSON has valid=false
    And the returned JSON has warnings equal to ['No scenarios for AUTH-001']

  Scenario: Errors when the work unit does not exist
    Given spec/work-units.json contains AUTH-001 but not MISSING-999
    When I dispatch validate-spec-alignment with workUnitId='MISSING-999'
    Then the dispatcher returns success=false with an error message containing 'Failed to validate spec alignment: Work unit MISSING-999 not found'

  Scenario: A missing spec/features directory yields zero scenarios and an invalid result
    Given spec/work-units.json contains AUTH-001
    And the spec/features directory does not exist
    When I dispatch validate-spec-alignment with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    And the returned JSON has valid=false
    And the returned JSON has warnings equal to ['No scenarios for AUTH-001']

  Scenario: A tag substring on a line not followed by Scenario does not count
    Given spec/work-units.json contains AUTH-001
    And a feature file has a line containing '@AUTH-001' that is NOT immediately followed by a 'Scenario:' line
    When I dispatch validate-spec-alignment with workUnitId='AUTH-001'
    Then the returned JSON has valid=false
    And the returned JSON has warnings equal to ['No scenarios for AUTH-001']

  Scenario: Errors when workUnitId is omitted
    Given spec/work-units.json contains AUTH-001
    When I dispatch validate-spec-alignment with no workUnitId
    Then the dispatcher returns success=false with an error message indicating the work unit id is required

  Scenario: Errors when work-units.json is malformed
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch validate-spec-alignment with workUnitId='AUTH-001'
    Then the dispatcher returns success=false with an error message containing 'Failed to validate spec alignment:'
