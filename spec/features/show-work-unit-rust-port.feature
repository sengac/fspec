@done
@RPC-308
@cli
@querying
Feature: Port show-work-unit command to Rust
  """
  File layout: rust/fspec-core/src/commands/show_work_unit.rs (impl, replaces stub) + rust/fspec-core/src/help/configs/show_work_unit.rs (help config) + rust/fspec-core/tests/show_work_unit.rs (dispatcher tests) + rust/fspec/src/show_work_unit.rs (CLI bridge) + rust/fspec/tests/cli_show_work_unit.rs (CLI shell tests) + rust/fspec/tests/fixtures/help/show-work-unit.txt (TS help fixture)
  Reuses shared modules: gherkin crate (already in fspec-core Cargo.toml) for parsing feature files; io::feature_glob::glob_feature_files for walking spec/features/ (errors silently swallowed to match TS); does NOT use ensure_work_units_file because TS uses bare readFile that escalates ENOENT. WorkUnit.rules/examples/questions/architectureNotes are read inline from wu.extra (parity with show_deleted) so the shared WorkUnit type stays minimal and parallel-port-safe
  Two-front-doors per RPC-003 §7/§11: shell argv → clap → rust/fspec/src/show_work_unit.rs → codelet_fspec_core::commands::show_work_unit::run; LLM tool call JSON → fspec_core::dispatch::dispatch_command → codelet_fspec_core::commands::show_work_unit::run. Both call sites pass JSON-encoded args and project_root: &Path. CLI bridge marshals workUnitId + optional --format into the JSON shape and emits NO business logic
  JSON structured shape (dispatcher path with format=json): declaration-order fields { id, title, type, status, description?, estimate?, epic?, parent?, children?, blocks?, blockedBy?, dependsOn?, relatesTo?, rules?, deletedRules?, examples?, questions?, assumptions?, architectureNotes?, attachments?, virtualHooks?, createdAt, updatedAt, linkedFeatures, systemReminders?, systemReminder? }. Use #[derive(Serialize)] with explicit #[serde(skip_serializing_if='Option::is_none')] on optional fields
  System-reminder generation: helper module within show_work_unit.rs mirrors the five reminder functions from src/utils/system-reminder.ts (getMissingEstimateReminder, getEmptyExampleMappingReminder, getLongDurationReminder, getLargeEstimateReminder, soft-delete count notice). The FSPEC_DISABLE_REMINDERS=1 environment gate is honoured. consolidateReminders strips <system-reminder> wrappers and re-wraps a single block
  linkedFeatures implementation mirrors show_feature::extract_work_unit_tags() but ALWAYS returns an empty array on any error (missing spec/features/, gherkin parse failure, I/O)
  Shared-file wiring needed from supervisor AFTER worker impl lands: (1) help/configs/mod.rs; (2) dispatch.rs (move show-work-unit from run_stub to run_ported); (3) canonical.rs PORTED_COMMANDS; (4) fspec/src/main.rs (Mode::ShowWorkUnit + dispatch + intercept_ts_help branch)
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch show-work-unit and run it from a shell so I get the same detailed work-unit dump (Example Mapping data, dependencies, linked features, system reminders) as the TS implementation
    So that I have a single source of truth for inspecting a work unit across the LLM dispatcher and shell front doors

  Scenario: Returns a minimal work unit with declaration-order fields and an empty linkedFeatures array
    Given a tempdir whose spec/work-units.json contains AUTH-001 with title='Login', status='backlog', no rules/examples/questions/notes, and no estimate
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses as JSON with id='AUTH-001', title='Login', type='story', status='backlog'
    Then the JSON payload's linkedFeatures field is an empty array
    Then the JSON payload omits both systemReminders and systemReminder (backlog status suppresses the missing-estimate reminder)

  Scenario: Returns success=false when spec/work-units.json is absent
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch show-work-unit with workUnitId='AUTH-001'
    Then the dispatcher returns success=false
    Then the error message does NOT contain the substring "does not exist" (TS bare readFile escalates ENOENT; the Rust port surfaces a structured I/O error and does NOT auto-create)
    Then spec/work-units.json was NOT created in the directory

  Scenario: Returns success=false with the canonical missing-work-unit message
    Given spec/work-units.json contains AUTH-001 (any minimal shape)
    When I dispatch show-work-unit with workUnitId='UNKNOWN-999' and format='json'
    Then the dispatcher returns success=false
    Then the error message contains the exact substring "Work unit 'UNKNOWN-999' does not exist"

  Scenario: Projects active rules and omits soft-deleted entries in non-verbose mode
    Given spec/work-units.json contains AUTH-001 with rules=[{id:0,text:'A',deleted:false},{id:1,text:'B',deleted:true},{id:2,text:'C',deleted:false}] and status='implementing'
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON payload's rules array equals ["[0] A", "[2] C"]
    Then the JSON payload does NOT contain a deletedRules field

  Scenario: Projects active examples and omits soft-deleted entries
    Given spec/work-units.json contains AUTH-001 with examples=[{id:0,text:'E1',deleted:false},{id:1,text:'gone',deleted:true}] and status='implementing'
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON payload's examples array equals ["[0] E1"]

  Scenario: Filters questions by both deleted and selected flags
    Given spec/work-units.json contains AUTH-001 with questions=[{id:0,text:'Q1',deleted:false,selected:false},{id:1,text:'answered',deleted:false,selected:true},{id:2,text:'gone',deleted:true,selected:false}] and status='implementing'
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON payload's questions array equals ["[0] Q1"]

  Scenario: Projects active architecture notes and omits soft-deleted entries
    Given spec/work-units.json contains AUTH-001 with architectureNotes=[{id:0,text:'N1',deleted:false},{id:1,text:'N2',deleted:false},{id:2,text:'gone',deleted:true}] and status='implementing'
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON payload's architectureNotes array equals ["[0] N1", "[1] N2"]

  Scenario: Rejects legacy bare-string question entries with a canonical error
    Given spec/work-units.json contains AUTH-001 with questions=["bare string"] (legacy format)
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=false
    Then the error message contains the exact substring "Invalid question format. Questions must be QuestionItem objects."

  Scenario: Emits the bare soft-delete count notice when rules has both active and deleted entries
    Given spec/work-units.json contains AUTH-001 with status='implementing', estimate=3, and rules=[{id:0,text:'a',deleted:false},{id:1,text:'b',deleted:false},{id:2,text:'c',deleted:false},{id:3,text:'d',deleted:true}]
    Given the environment variable FSPEC_DISABLE_REMINDERS is unset
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON payload's systemReminders array contains the bare string "3 active items (1 deleted)"
    Then the JSON payload's systemReminder field is a single <system-reminder>…</system-reminder> block containing the substring "3 active items (1 deleted)"

  Scenario: Inherits feature-level work-unit tags onto scenarios that lack their own override
    Given spec/work-units.json contains AUTH-001
    Given spec/features/auth.feature has '@AUTH-001' as a feature-level tag, a 'Login' scenario with NO scenario-level work-unit tag, and a 'Logout' scenario carrying its own '@AUTH-002' override
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON payload's linkedFeatures array has exactly one entry whose file ends with 'spec/features/auth.feature'
    Then that entry's scenarios array references only the 'Login' scenario (the Logout scenario is excluded because of its own @AUTH-002 override)

  Scenario: Silently degrades when spec/features/ does not exist
    Given spec/work-units.json contains AUTH-001
    Given there is no spec/features/ directory in the project root
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON payload's linkedFeatures field is an empty array (the missing directory is NOT escalated)

  Scenario: Consolidates multiple system reminders into a single wrapped block
    Given spec/work-units.json contains AUTH-001 with status='specifying', no estimate, no rules, no examples
    Given the environment variable FSPEC_DISABLE_REMINDERS is unset
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON payload's systemReminders array contains at least two entries (missing-estimate AND empty-example-mapping)
    Then the JSON payload's systemReminder field is a single <system-reminder>…</system-reminder> block whose body joins the reminders with a blank line

  Scenario: Emits the large-estimate reminder with the create-feature-file-first branch
    Given spec/work-units.json contains AUTH-001 with type='story', estimate=21, status='implementing'
    Given there is no spec/features/ directory or no feature file tagged @AUTH-001
    Given the environment variable FSPEC_DISABLE_REMINDERS is unset
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON payload's systemReminders array contains a reminder whose body contains the exact substring "LARGE ESTIMATE WARNING"
    Then that same reminder contains the exact substring "CREATE FEATURE FILE FIRST"

  Scenario: Honours the FSPEC_DISABLE_REMINDERS=1 environment gate
    Given the environment variable FSPEC_DISABLE_REMINDERS is set to "1"
    Given spec/work-units.json contains AUTH-001 with status='specifying', no estimate, no rules, no examples
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON payload omits both systemReminders and systemReminder

  Scenario: Text format renders a multi-section dump with type status epic and dependency lines
    Given spec/work-units.json contains AUTH-001 with title='Login', description='Implement auth', epic='auth', parent='RPC-003', blocks=['X-1','X-2'], rules=[{id:0,text:'must be 8+ chars',deleted:false}], examples=[{id:0,text:'happy path',deleted:false}], attachments=['spec/attachments/AUTH-001/diagram.png'], status='backlog'
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the line "AUTH-001"
    Then the DispatchResult.data contains the line "Type: story"
    Then the DispatchResult.data contains the line "Status: backlog"
    Then the DispatchResult.data contains the substring "Epic: auth"
    Then the DispatchResult.data contains the substring "Parent: RPC-003"
    Then the DispatchResult.data contains the substring "Blocks: X-1, X-2"
    Then the DispatchResult.data contains the line "Rules:"
    Then the DispatchResult.data contains the line "  [0] must be 8+ chars"
    Then the DispatchResult.data contains the line "Examples:"
    Then the DispatchResult.data contains the line "  [0] happy path"

  Scenario: JSON format emits a 2-space indented payload with the canonical field set
    Given spec/work-units.json contains AUTH-001 with title='x', status='backlog'
    When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the DispatchResult.data uses 2-space indentation
    Then the DispatchResult.data parses as JSON whose root contains id, title, type, status, createdAt, updatedAt, linkedFeatures

  Scenario: Defaults to text format when the format argument is omitted
    Given spec/work-units.json contains AUTH-001 with title='x', status='backlog'
    When I dispatch show-work-unit with workUnitId='AUTH-001' and no format field supplied
    Then the dispatcher returns success=true
    Then the DispatchResult.data starts with a section that contains the line "AUTH-001"

  Scenario: Returns a structured error when workUnitId is missing from the dispatcher args
    Given an empty project root directory
    When I dispatch show-work-unit with an empty args object
    Then the dispatcher returns success=false
    Then the error message describes the missing workUnitId argument

  Scenario: Shared infrastructure delegation
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/commands/show_work_unit.rs
    Then the file does NOT contain the substring "FspecCoreError::NotYetPorted"
    Then the file uses the shared gherkin crate to parse feature files (mirroring show_feature.rs)
