@done
@querying
@cli
@rust
@RPC-205
Feature: Port clear-virtual-hooks command to Rust
  """
  New impl file at rust/fspec-core/src/commands/clear_virtual_hooks.rs replaces the NotYetPorted stub. Module exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` mirroring the list_virtual_hooks::run signature.

  The shell-facing CLI bridge is delivered alongside this port — see rust/fspec/src/clear_virtual_hooks.rs and spec/features/clear-virtual-hooks-cli-subcommand.feature. Both front doors (LLM tool-call dispatcher and clap shell subcommand) call the same `clear_virtual_hooks::run` function defined in this port.

  The command reads `spec/work-units.json` via the shared `crate::io::ensure::ensure_work_units_file` helper (auto-creates an empty store on ENOENT, parity with the TS `ensureWorkUnitsFile` helper). It then looks up the requested work unit by id; if not present, returns `FspecCoreError::InvalidArgs { reason: "Work unit '<id>' does not exist" }` (the dispatcher converts this to success=false with that error string).

  When the work unit exists, virtual hooks are read from `wu.extra["virtualHooks"]` as a `serde_json::Value::Array`. For each entry, a best-effort `std::fs::remove_file(spec/hooks/.virtual/<workUnitId>-<hookName>.sh)` is attempted — any error (ENOENT or otherwise) is silently ignored, mirroring the TS try/catch. Once script cleanup has been attempted, `extra["virtualHooks"]` is REPLACED with `Value::Array(Vec::new())` (an empty array, NOT removed), `wu.updated_at` is bumped via `crate::io::time::iso8601_now()`, and the state is persisted with `crate::io::locked_file::write_json_atomic(spec/work-units.json, &data)`.

  Output shape (JSON): `{"success": true, "clearedCount": <n>}` — serialized via `#[derive(Serialize)]` struct so insertion order is preserved on the wire. Text format returned by `run`: `"✓ Cleared <n> virtual hook(s) from <id>"` (no trailing newline; the CLI bridge prints it and appends a newline).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rust dispatcher route for clear-virtual-hooks replaces the NotYetPorted stub
  #   2. workUnitId is required; missing or empty surfaces as InvalidArgs from the args parse
  #   3. Unknown workUnitId returns success=false with exact error "Work unit '<id>' does not exist"
  #   4. Missing or empty virtualHooks counts as clearedCount=0 with success=true (idempotent)
  #   5. Populated virtualHooks array is replaced with [] (empty array, NOT removed)
  #   6. For each hook in the original list, best-effort unlink spec/hooks/.virtual/<id>-<name>.sh; ignore ENOENT and other errors
  #   7. Work unit updatedAt is bumped to ISO-8601 now on success
  #   8. Persistence uses a single write_json_atomic over spec/work-units.json after mutations
  #   9. CLI delegates to single source of truth in fspec_core via the two-front-doors pattern
  #   10. CLI stdout success: "✓ Cleared <n> virtual hook(s) from <id>"; failure stderr: "✗ Failed to clear virtual hooks: <message>" exit 1
  #   11. Help intercept produces byte-exact output matching node dist/index.js clear-virtual-hooks --help
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch clear-virtual-hooks from the agent loop AND invoke `fspec clear-virtual-hooks <workUnitId>` from a shell
    So that I can wipe all work-unit-scoped virtual hooks and clean up their generated scripts from one source of truth

  Scenario: Returns error when the requested work unit does not exist
    Given spec/work-units.json contains AUTH-001 with no virtualHooks
    When I dispatch clear-virtual-hooks with workUnitId='AUTH-999'
    Then the dispatcher returns success=false
    Then the error message contains the exact substring "Work unit 'AUTH-999' does not exist"

  Scenario: Returns error when spec/work-units.json is auto-created and the requested id is not in the empty store
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    Then the dispatcher returns success=false
    Then the error message contains the exact substring "Work unit 'AUTH-001' does not exist"
    Then spec/work-units.json exists after the call

  Scenario: Clears all hooks from a work unit with three virtualHooks
    Given spec/work-units.json contains AUTH-001 with virtualHooks in order: 'lint' (post-implementing), 'test' (post-implementing), 'eslint' (pre-validating)
    When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the parsed JSON has clearedCount=3
    Then spec/work-units.json AUTH-001 virtualHooks is an empty array
    Then spec/work-units.json AUTH-001 updatedAt is a valid ISO-8601 timestamp newer than before

  Scenario: Clearing a work unit with no virtualHooks succeeds with clearedCount=0
    Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the parsed JSON has clearedCount=0
    Then spec/work-units.json AUTH-001 virtualHooks is an empty array

  Scenario: Clearing a work unit with an empty virtualHooks array succeeds with clearedCount=0
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[]
    When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the parsed JSON has clearedCount=0
    Then spec/work-units.json AUTH-001 virtualHooks is an empty array

  Scenario: Script files in spec/hooks/.virtual/ are unlinked for each cleared hook
    Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint' and 'test'
    Given spec/hooks/.virtual/AUTH-001-lint.sh exists
    Given spec/hooks/.virtual/AUTH-001-test.sh exists
    When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then spec/hooks/.virtual/AUTH-001-lint.sh no longer exists
    Then spec/hooks/.virtual/AUTH-001-test.sh no longer exists

  Scenario: Missing script files are silently ignored
    Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint' and 'test'
    Given spec/hooks/.virtual/ does not contain any script files
    When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the parsed JSON has clearedCount=2

  Scenario: Missing workUnitId argument is rejected as InvalidArgs
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch clear-virtual-hooks with an empty args object {}
    Then the dispatcher returns success=false
    Then the error message indicates that workUnitId is required

  Scenario: Result JSON shape preserves field order success then clearedCount
    Given spec/work-units.json contains AUTH-001 with no virtualHooks
    When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses to a JSON object whose first key is "success" and whose second key is "clearedCount"
