@done
@rust
@foundation-management
@cli
@RPC-226
Feature: Port discover-foundation command to Rust
  """
  Core impl at rust/fspec-core/src/commands/discover_foundation.rs. Signature changes from stub run(args_json) to run(args_json, project_root). Args: {finalize:bool, output:Option<String>, draftPath:Option<String>, autoGenerateMd:bool(default true), force:bool}. Returns a JSON envelope String (mirroring update_foundation.rs) that the CLI bridge decodes to render exact TS stdout/stderr lines.
  Reuse existing shared helpers: io::ensure::{ensure_foundation_file, ensure_work_units_file}, io::locked_file::{write_json_atomic} (2-space indent, no trailing newline = JSON.stringify null,2), commands::generate_foundation_md::regenerate(project_root), generators::foundation_schema::validate_foundation(&Value)->Result<(),Vec<SchemaError>> (SchemaError has instance_path+message only). Draft read = std::fs::read_to_string + serde_json::from_str. Draft write/dir = std::fs::create_dir_all + write_json_atomic.
  Field-by-field reminder logic (scanDraftForNextField + generateFieldReminder + extract_detected_value + agent_supports_meta_cognition + is_known_agent) is ALREADY ported privately inside update_foundation.rs. SHARED-FILE REQUEST to supervisor: promote these into a shared module (e.g. commands/foundation_reminder.rs or a fn in io/) so discover_foundation.rs can reuse them instead of duplicating ~150 LOC. If the supervisor prefers, I will copy them into discover_foundation.rs (isolated, parallel-safe) as a fallback.
  FOUND auto-unit: no core create_task fn exists (only create_story). PROPOSAL (isolated, no shared-file change): inline-build the FOUND task object in discover_foundation.rs the way create_story.rs does — check existing FOUND- id for idempotency, ensure FOUND prefix (reuse create_prefix::run, swallow already-exists), build {id,title,type:'task',status:'backlog',createdAt,updatedAt,description} + states.backlog push + prefixCounters. Entire block best-effort (swallow all errors) per TS try/catch. Supervisor: confirm inline vs a shared create-work-unit helper.
  SHARED-FILE REQUESTS to supervisor (wiring, Phase C): (1) canonical.rs PORTED_COMMANDS add 'discover-foundation'; (2) dispatch.rs move from run_stub branch to run_ported with run(args_json, project_root) — current dispatch.rs:634 calls run(args_json) only; (3) main.rs Mode::DiscoverFoundation clap variant {finalize,output,draft_path,auto_generate_md,force} + forward! arm + intercept_ts_help arm + mod discover_foundation; (4) help/configs/mod.rs register discover_foundation::CONFIG. Worker owns: commands/discover_foundation.rs (rewrite stub), help/configs/discover_foundation.rs, fspec/src/discover_foundation.rs bridge, tests, fixture.
  NO real async / no child-process / no network. All file IO is std::fs blocking; create_prefix/create_story/generate_foundation_md/validate_foundation are sync. Safe under poll_sync_future (single-poll). CLI surface intentionally exposes only finalize/output/draftPath/autoGenerateMd/force — scanOnly/detectManualEdit/lastKnownState are TS-internal (used by update-foundation chaining) and are OUT of port scope.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Default mode (no --finalize) creates spec/foundation.json.draft with version 2.0.0 and [QUESTION:]/[DETECTED:] placeholders, written as JSON.stringify(...,null,2) (2-space indent)
  #   2. Without --force, if the draft already exists the command fails (valid=false) emitting a wrapped <system-reminder> 'ERROR: foundation.json.draft already exists!' with three next-step options, and the CLI exits 1 with '✗ Failed to create draft'
  #   3. Without --force, if the draft is absent but spec/foundation.json already exists the command fails (valid=false) emitting a wrapped <system-reminder> 'ERROR: foundation.json already exists!' and the CLI exits 1
  #   4. With --force, an existing draft is overwritten: a force-overwrite warning line is prepended to the systemReminder and the fresh draft is regenerated from scratch
  #   5. On successful draft creation the returned systemReminder embeds a field-by-field reminder for the FIRST unfilled placeholder (Field 1/8: project.name) and an agent-aware ULTRATHINK vs 'think a lot' thinking instruction; the CLI prints the systemReminder, then '✓ Generated <draftPath>' and two yellow 'Next steps' lines
  #   6. Finalize mode (--finalize) reads+parses spec/foundation.json.draft and scans for unfilled placeholder fields; if any remain it fails (valid=false) with 'Cannot finalize: draft still has unfilled placeholder fields...' naming the offending field, and the CLI exits 1 with '✗ Foundation validation failed'
  #   7. Finalize mode with all placeholders filled validates the draft against the generic-foundation schema; on schema failure it returns 'Schema validation failed.' with per-error messages (Missing required / minItems / minLength / maxLength / enum) and exits 1 without writing foundation.json
  #   8. Finalize success writes spec/foundation.json (2-space indent), deletes the draft, auto-creates an idempotent FOUND-prefixed task work unit (Foundation Event Storm), and when autoGenerateMd is true regenerates spec/FOUNDATION.md; the CLI prints '✓ Generated <finalPath>', optionally '✓ Generated spec/FOUNDATION.md', '✓ Foundation discovered and validated successfully', and the FOUND work-unit lines
  #   9. The FOUND work-unit auto-creation is best-effort: if it fails or a FOUND- unit already exists no error surfaces (the finalize still succeeds), matching the TS try/catch and idempotency check
  #   10. Two-front-doors: the CLI bridge marshals only {finalize, output, draftPath, autoGenerateMd, force} to JSON; both the dispatcher and the standalone binary converge on commands::discover_foundation::run(args_json, project_root)
  #
  # EXAMPLES:
  #   1. Fresh discovery: in an empty project root, dispatching discover-foundation creates spec/foundation.json.draft containing version 2.0.0 and the [QUESTION:]/[DETECTED:] placeholders, and returns a systemReminder whose embedded field reminder is 'Field 1/8: project.name'
  #   2. Re-run blocked: dispatching discover-foundation again when spec/foundation.json.draft already exists (no --force) returns valid=false and a systemReminder containing 'ERROR: foundation.json.draft already exists!'; the draft on disk is unchanged
  #   3. Overwrite with --force: dispatching discover-foundation --force when a draft already exists regenerates the draft from scratch (placeholders restored) and prepends a force-overwrite warning to the systemReminder
  #   4. Finalize success: with a fully-filled valid draft, dispatching discover-foundation --finalize writes spec/foundation.json, deletes the draft, creates a FOUND- task work unit, regenerates spec/FOUNDATION.md, and returns valid=true with a completion message 'Discovery complete!'
  #   5. Finalize blocked by placeholders: dispatching discover-foundation --finalize on a draft that still has [QUESTION:] placeholders returns valid=false with validationErrors 'Cannot finalize: draft still has unfilled placeholder fields...' and does NOT write foundation.json or delete the draft
  #   6. Finalize schema failure: dispatching discover-foundation --finalize on a draft whose placeholders are filled but which violates the schema (e.g. empty solutionSpace.capabilities) returns valid=false with validationErrors starting 'Schema validation failed.' and a 'Missing required: ...' line
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to port the discover-foundation command to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can both create and finalize the foundation draft without falling back to TypeScript

  Scenario: Fresh discovery creates the draft with placeholders and a first-field reminder
    Given an empty project root tempdir with no spec/foundation.json.draft and no spec/foundation.json
    When I dispatch discover-foundation with no flags
    Then the dispatcher returns valid=true
    And spec/foundation.json.draft exists on disk
    And the draft on disk contains the version "2.0.0"
    And the draft on disk contains the placeholder "[QUESTION: What is the project name?]"
    And the draft on disk contains the placeholder "[DETECTED: cli-tool]"
    And the returned systemReminder contains "Field 1/8: project.name"

  Scenario: Re-running without force when a draft already exists is blocked
    Given a project root tempdir that already has a spec/foundation.json.draft
    When I dispatch discover-foundation with no flags
    Then the dispatcher returns valid=false
    And the returned systemReminder contains "ERROR: foundation.json.draft already exists!"
    And spec/foundation.json.draft on disk is byte-equal to its pre-call contents

  Scenario: Running without force when foundation.json already exists is blocked
    Given a project root tempdir that has a spec/foundation.json but no spec/foundation.json.draft
    When I dispatch discover-foundation with no flags
    Then the dispatcher returns valid=false
    And the returned systemReminder contains "ERROR: foundation.json already exists!"
    And no spec/foundation.json.draft is created

  Scenario: Force overwrite regenerates the draft from scratch with a warning
    Given a project root tempdir that already has a spec/foundation.json.draft with custom content
    When I dispatch discover-foundation with force=true
    Then the dispatcher returns valid=true
    And the draft on disk contains the placeholder "[QUESTION: What is the project name?]"
    And the returned systemReminder contains a force-overwrite warning

  Scenario: Finalize blocked when the draft still has placeholder fields
    Given a project root tempdir whose spec/foundation.json.draft still has [QUESTION:] placeholders
    When I dispatch discover-foundation with finalize=true
    Then the dispatcher returns valid=false
    And the returned validationErrors contains "Cannot finalize: draft still has unfilled placeholder fields"
    And spec/foundation.json is not created
    And spec/foundation.json.draft still exists on disk

  Scenario: Finalize blocked when the filled draft violates the schema
    Given a project root tempdir whose spec/foundation.json.draft has no placeholders but empty solutionSpace.capabilities
    When I dispatch discover-foundation with finalize=true
    Then the dispatcher returns valid=false
    And the returned validationErrors starts with "Schema validation failed."
    And spec/foundation.json is not created

  Scenario: Finalize success writes foundation.json, deletes the draft, and creates the FOUND work unit
    Given a project root tempdir whose spec/foundation.json.draft is fully filled and schema-valid
    When I dispatch discover-foundation with finalize=true and autoGenerateMd=true
    Then the dispatcher returns valid=true
    And the returned completionMessage contains "Discovery complete!"
    And spec/foundation.json exists on disk
    And spec/foundation.json.draft no longer exists on disk
    And spec/FOUNDATION.md exists on disk
    And spec/work-units.json contains a FOUND-prefixed work unit
    And the envelope reports workUnitCreated=true with workUnitId "FOUND-001"
    And the FOUND task carries a stateHistory array, no children array, and the file has no prefixCounters key

  Scenario: Finalize FOUND auto-creation is idempotent when a FOUND unit already exists
    Given a project root tempdir whose spec/foundation.json.draft is fully filled and a FOUND-001 work unit already exists
    When I dispatch discover-foundation with finalize=true
    Then the dispatcher returns valid=true
    And spec/work-units.json still has exactly one FOUND-prefixed work unit
    And the envelope reports workUnitCreated=false reusing workUnitId "FOUND-001"
