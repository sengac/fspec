@done
@rust
@foundation-management
@cli
@RPC-226
Feature: Port discover-foundation command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/discover_foundation.rs. Signature changes from stub run(args_json) to run(args_json, project_root). Args: {finalize:bool, output:Option<String>, draftPath:Option<String>, autoGenerateMd:bool(default true), force:bool}. Returns a JSON envelope String (mirroring update_foundation.rs) that the CLI bridge decodes to render exact TS stdout/stderr lines.
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

  Scenario: CLI creates the draft and prints the next-steps guidance
    Given an empty project root tempdir
    When I run `fspec discover-foundation` in that directory
    Then the command exits with code 0
    And stdout contains "✓ Generated spec/foundation.json.draft"
    And stdout contains "Next steps:"
    And stdout contains "1. Use fspec update-foundation commands to fill [QUESTION: ...] placeholders"
    And stdout contains "2. When complete, run: fspec discover-foundation --finalize"

  Scenario: CLI fails when the draft already exists without force
    Given a project root tempdir that already has a spec/foundation.json.draft
    When I run `fspec discover-foundation` in that directory
    Then the command exits with code 1
    And stderr contains "✗ Failed to create draft"
    And stdout contains "ERROR: foundation.json.draft already exists!"

  Scenario: CLI finalize success prints the generated-foundation lines
    Given a project root tempdir whose spec/foundation.json.draft is fully filled and schema-valid
    When I run `fspec discover-foundation --finalize` in that directory
    Then the command exits with code 0
    And stdout contains "✓ Generated spec/foundation.json"
    And stdout contains "✓ Foundation discovered and validated successfully"

  Scenario: CLI finalize failure on incomplete draft exits 1 with validation errors
    Given a project root tempdir whose spec/foundation.json.draft still has [QUESTION:] placeholders
    When I run `fspec discover-foundation --finalize` in that directory
    Then the command exits with code 1
    And stderr contains "✗ Foundation validation failed"
    And stderr contains "Cannot finalize: draft still has unfilled placeholder fields"

  Scenario: discover-foundation --help matches the TS formatCommandHelp reference
    Given the standalone fspec binary
    When I run `fspec discover-foundation --help`
    Then stdout is byte-for-byte identical to tests/fixtures/help/discover-foundation.txt

  Scenario: CLI bridge delegates to the same fspec-core function as the dispatcher
    Given a project root tempdir with no spec/foundation.json.draft
    When I dispatch discover-foundation via the dispatcher and via the standalone binary with identical flags
    Then both invocations produce the same draft content on disk

