@validation
@validator
@rust
@wip
@RPC-325
Feature: Port validate-work-units command to Rust
  """
  File layout: core impl rust/fspec-core/src/commands/validate_work_units.rs (rewrite stub); help config rust/fspec-core/src/help/configs/validate_work_units.rs; CLI bridge rust/fspec/src/validate_work_units.rs; core test rust/fspec-core/tests/validate_work_units.rs; CLI test rust/fspec/tests/cli_validate_work_units.rs; help fixture rust/fspec/tests/fixtures/help/validate-work-units.txt
  Implement validation over RAW serde_json::Value (mirror TS untyped JSON.parse) because every check inspects ad-hoc fields and TS keeps invalid status as a raw string. SHARED-FILE REQUEST to supervisor: add io::ensure helper to load spec/work-units.json as raw Value with ensure semantics (auto-create default, escalate parse) — proposed ensure_work_units_value(cwd) -> Result<Value> — OR confirm we can call ensure_work_units_file then serde_json::to_value (note: typed WorkUnitsData parse would REJECT an invalid status before Check 4 can run, breaking parity, so a raw loader is preferred). Supervisor also wires canonical.rs PORTED_COMMANDS, dispatch.rs run_ported, help/configs/mod.rs, main.rs Mode+intercept+forward.
  OPEN DECISION for supervisor: validate-work-units-help.ts lists a '--fix' OPTION but the TS Commander registration implements NO flags. Capture the byte-exact --help fixture in PHASE B from `node dist/index.js validate-work-units --help` to settle whether the help OPTIONS section shows --fix (rich help) or nothing (bare Commander). The clap surface exposes no functional flag regardless; --fix is documented-only.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Load work-units.json via ensure semantics (auto-create default if missing, escalate parse error). Validation runs over the untyped JSON so it mirrors TS dynamic access of ad-hoc fields
  #   2. Result shape: { valid: bool, checks: [schema, uniqueIds, parentChild, exampleMapping, dependencies], errors?: [..] }. valid = errors empty; errors omitted when empty
  #   3. Schema check reports 'Invalid work units data structure: missing or invalid workUnits field' and the same for the states field when those keys are absent or not objects
  #   4. Parent/child check: a parent referenced but absent reports 'Work unit <id> references non-existent parent: <parent>'; a parent that does not list the child reports 'Work unit <id> has parent <parent>, but parent does not list it as a child'; a missing child reports 'Work unit <id> references non-existent child: <childId>'; a child whose parent field mismatches reports 'Work unit <id> lists <childId> as child, but child does not have it as parent'
  #   5. Invalid status check: a work unit whose status is outside {backlog,specifying,testing,implementing,validating,done,blocked} reports 'Invalid status value for <id>: <status>. Allowed values: <comma-joined list>'
  #   6. State-index consistency: a work unit not present in states.<status> reports 'State consistency error: Work unit <id> has status ... but is not in states.<status> array. Run ...'; a work unit present in a DIFFERENT state array reports 'State consistency error: Work unit <id> has status ... but is in ... array. Run ...'
  #   7. Example-mapping arrays: rules/examples/assumptions must be arrays of non-empty strings (else 'must be an array' or '<field> array contains empty strings or non-strings at index <i>'); questions must be an array of objects each with non-empty string text, boolean selected, and optional non-empty string answer (else the matching QuestionItem messages)
  #   8. Dependency arrays: blocks/blockedBy/dependsOn/relatesTo must each be an array of non-empty strings, else '<id>: <field> must be an array' or '<id>: <field> array contains empty strings or non-strings at index <i>'
  #   9. CLI output: valid -> stdout '✓ All work units are valid', exit 0; invalid -> stderr '✗ Found <N> validation errors' then '  - <error>' per error, exit 1; unexpected exception -> stderr '✗ Failed to validate work units: <msg>', exit 1
  #   10. Two front doors: clap subcommand exposes NO functional flags (TS Commander registration declares none); both dispatcher and CLI call fspec_core::commands::validate_work_units::run. --help is byte-for-byte identical to TS formatCommandHelp (custom validate-work-units-help.ts which lists a --fix option in its OPTIONS section even though the command does not implement --fix)
  #
  # EXAMPLES:
  #   1. A clean work-units.json with consistent parent/child links, valid statuses, and matching state arrays validates with zero errors
  #   2. A work unit whose parent is absent from workUnits produces a 'references non-existent parent' error
  #   3. A work unit with status 'review' (not an allowed state) produces an 'Invalid status value' error
  #   4. A work unit with status 'done' that is not listed in states.done produces a 'State consistency error' mentioning repair-work-units
  #   5. A work unit with a rules array containing an empty string produces a 'rules array contains empty strings or non-strings at index' error
  #   6. A work unit with a blockedBy value that is not an array produces a 'blockedBy must be an array' error
  #   7. Running validate-work-units on a clean store prints '✓ All work units are valid' and exits 0; on a corrupt store prints '✗ Found N validation errors' with bullet lines and exits 1
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to run `fspec validate-work-units` to check work-unit data integrity (schema, unique IDs, parent/child links, valid statuses, state-index consistency, example-mapping arrays, dependency arrays)
    So that I can detect corrupted or inconsistent work-units.json and know to run repair-work-units before continuing

  Scenario: Dispatcher reports a clean store as valid
    Given spec/work-units.json contains consistent work units with matching parent/child links, valid statuses, and correct state arrays
    When I dispatch the validate-work-units command against that project root
    Then the dispatcher returns success=true
    Then the result reports valid=true with no errors
    Then the result checks list includes schema, uniqueIds, parentChild, exampleMapping, and dependencies

  Scenario: Dispatcher flags a work unit whose parent is missing
    Given spec/work-units.json contains AUTH-002 with parent AUTH-001 but no AUTH-001 entry
    When I dispatch the validate-work-units command against that project root
    Then the dispatcher returns failure with the message 'Cannot read properties of undefined (reading 'children')'

  Scenario: Dispatcher flags a parent that does not list its child
    Given spec/work-units.json contains AUTH-001 with no children and AUTH-002 whose parent is AUTH-001
    When I dispatch the validate-work-units command against that project root
    Then the result reports valid=false
    Then the errors include "Work unit AUTH-002 has parent AUTH-001, but parent doesn't list it as a child"

  Scenario: Dispatcher flags a missing child reference
    Given spec/work-units.json contains AUTH-001 whose children list AUTH-099 which does not exist
    When I dispatch the validate-work-units command against that project root
    Then the result reports valid=false
    Then the errors include 'Work unit AUTH-001 references non-existent child: AUTH-099'

  Scenario: Dispatcher flags an invalid status value
    Given spec/work-units.json contains AUTH-001 with status 'review'
    When I dispatch the validate-work-units command against that project root
    Then the result reports valid=false
    Then the errors include a message starting with 'Invalid status value for AUTH-001: review'

  Scenario: Dispatcher flags a work unit absent from its state array
    Given spec/work-units.json contains AUTH-001 with status 'done' but states.done does not include AUTH-001
    When I dispatch the validate-work-units command against that project root
    Then the result reports valid=false
    Then the errors include a message containing 'has status' and 'is not in states.done array'

  Scenario: Dispatcher flags a work unit listed in the wrong state array
    Given spec/work-units.json contains AUTH-001 with status 'done' that also appears in states.backlog
    When I dispatch the validate-work-units command against that project root
    Then the result reports valid=false
    Then the errors include a message containing "is in 'backlog' array"

  Scenario: Dispatcher flags an empty string in the rules array
    Given spec/work-units.json contains AUTH-001 whose rules array contains an empty string at index 0
    When I dispatch the validate-work-units command against that project root
    Then the result reports valid=false
    Then the errors include 'AUTH-001: rules array contains empty strings or non-strings at index 0'

  Scenario: Dispatcher flags a malformed questions item
    Given spec/work-units.json contains AUTH-001 whose questions array contains a string instead of an object
    When I dispatch the validate-work-units command against that project root
    Then the result reports valid=false
    Then the errors include a message containing 'questions[0] must be a QuestionItem object'

  Scenario: Dispatcher flags a non-array dependency field
    Given spec/work-units.json contains AUTH-001 whose blockedBy field is a string instead of an array
    When I dispatch the validate-work-units command against that project root
    Then the result reports valid=false
    Then the errors include 'AUTH-001: blockedBy must be an array'

  Scenario: Dispatcher reports schema errors for a missing workUnits field
    Given spec/work-units.json contains a states object but no workUnits field
    When I dispatch the validate-work-units command against that project root
    Then the dispatcher returns failure with the message 'Cannot convert undefined or null to object'

  Scenario: Dispatcher auto-creates and validates an empty store when work-units.json is missing
    Given an empty project root with no spec/work-units.json
    When I dispatch the validate-work-units command against that project root
    Then the dispatcher returns success=true
    Then the result reports valid=true with no errors
