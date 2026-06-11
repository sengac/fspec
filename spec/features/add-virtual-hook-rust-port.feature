@done
@cli
@rust
@RPC-195
Feature: Port add-virtual-hook command to Rust

  """
  Two-front-doors invariant (RPC-003 §7/§11): both the LLM-facing dispatcher and the clap shell subcommand invoke fspec_core::commands::add_virtual_hook::run with a JSON args string and &Path project_root.
  virtualHooks lives in workUnit.extra (not a typed field on WorkUnit) — parity with the list_virtual_hooks port (RPC-252). The mutator reads/initialises wu.extra["virtualHooks"] as Value::Array and appends a typed VirtualHook serialised back via serde_json::to_value.
  Script generation is inlined as a private helper in add_virtual_hook.rs (mirror of src/hooks/script-generation.ts:34-105) — it does not warrant a shared module yet because clear-virtual-hooks (RPC-205) and copy-virtual-hooks are still stubs. Helper signature: generate_virtual_hook_script(work_unit_id, hook_name, command, project_root) -> Result<PathBuf>.
  Atomic write: single ensure_work_units_file load → in-memory mutation → single write_json_atomic. No fileManager.transaction wrapper needed — the Rust port already gives at-most-once write semantics via the locked_file helper.
  File permission 0o755 set via std::os::unix::fs::PermissionsExt — this command will not build on Windows targets. Mirrors TS chmod(scriptPath, 0o755) which is also Unix-only in practice (Node's fs.chmod is a no-op on Windows).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rust dispatcher route for add-virtual-hook replaces the NotYetPorted stub
  #   2. workUnitId, event, command are required arguments; missing → InvalidArgs
  #   3. Unknown workUnitId → success=false with error "Work unit '<id>' does not exist"
  #   4. Hook name auto-derived as command.split(' ')[0].split('/').pop() || 'hook'
  #   5. When --git-context is set, a bash script is generated at spec/hooks/.virtual/<workUnitId>-<hookName>.sh (mode 0o755) and the stored command becomes the relative script path
  #   6. Stored VirtualHook shape: {name, event, command, blocking} with optional gitContext serialised only when true
  #   7. Hooks are appended to wu.extra["virtualHooks"] preserving insertion order; new array is created when absent
  #   8. blocking defaults to false and gitContext defaults to false when their JSON keys are omitted
  #   9. On success, workUnit.updatedAt is bumped to the current ISO8601 timestamp and spec/work-units.json is rewritten atomically
  #   10. DispatchResult.data shape: {"success":true,"hookCount":<n>} (camelCase hookCount)
  #   11. CLI delegates to single source of truth in fspec_core::commands::add_virtual_hook::run
  #
  # EXAMPLES:
  #   1. Adding a blocking lint hook to AUTH-001 at post-implementing returns hookCount=1 and appends {name:'eslint',event:'post-implementing',command:'eslint src/',blocking:true} to virtualHooks
  #   2. Adding a second hook to AUTH-001 increments hookCount to 2 and preserves insertion order in virtualHooks
  #   3. Adding a hook with --git-context generates spec/hooks/.virtual/AUTH-001-eslint.sh (0o755) and stores command as the relative script path
  #   4. Dispatching with workUnitId='AUTH-999' that does not exist returns success=false with message "Work unit 'AUTH-999' does not exist"
  #   5. Dispatching with empty args object {} returns success=false and the error mentions missing workUnitId
  #   6. Hook name derivation: command 'npm run lint' → name='npm'; command 'eslint src/' → name='eslint'; command '/usr/bin/node script.js' → name='node'
  #   7. blocking defaults to false when --blocking is omitted; gitContext field is omitted from JSON when --git-context is not passed
  #   8. Adding a hook to a work unit that already has virtualHooks=[] correctly appends and returns hookCount=1
  #   9. spec/work-units.json file is auto-created on first invocation when missing; lookup still fails for the requested id because the new store is empty
  #   10. CLI subcommand: `fspec add-virtual-hook AUTH-001 post-implementing 'npm test' --blocking` prints '✓ Virtual hook added to AUTH-001' and '  Total virtual hooks: 1' and exits 0
  #   11. CLI subcommand with unknown work unit prints '✗ Failed to add virtual hook: Work unit ... does not exist' to stderr and exits 1
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch add-virtual-hook from the agent loop AND invoke `fspec add-virtual-hook <workUnitId> <event> <command>` from a shell
    So that I can attach a work-unit-scoped virtual hook (with optional git context script generation) to spec/work-units.json using one source of truth shared by the dispatcher and the CLI, without going through Node.js

  Scenario: Adds a blocking hook to a work unit with no prior virtualHooks
    Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='eslint src/' blocking=true
    Then the dispatcher returns success=true
    And the parsed JSON has hookCount=1
    And the on-disk virtualHooks array has length 1
    And the first stored hook has name='eslint' event='post-implementing' command='eslint src/' blocking=true
    And the stored hook does NOT contain a gitContext key

  Scenario: Appends a second hook preserving insertion order
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'lint',event:'post-implementing',command:'npm run lint',blocking:true}]
    When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='npm test' blocking=false
    Then the dispatcher returns success=true
    And the parsed JSON has hookCount=2
    And the stored virtualHooks names in order are ['lint','npm']

  Scenario: gitContext=true generates a shell script and stores the relative script path
    Given an empty project root directory with an AUTH-001 work unit
    When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='eslint src/' blocking=true gitContext=true
    Then the dispatcher returns success=true
    And the file spec/hooks/.virtual/AUTH-001-eslint.sh exists
    And the file spec/hooks/.virtual/AUTH-001-eslint.sh has Unix permission bits 0o755
    And the stored hook command is 'spec/hooks/.virtual/AUTH-001-eslint.sh'
    And the stored hook has gitContext=true

  Scenario: Unknown work unit id returns InvalidArgs with the canonical message
    Given spec/work-units.json contains AUTH-001
    When I dispatch add-virtual-hook with workUnitId='AUTH-999' event='post-implementing' command='npm test'
    Then the dispatcher returns success=false
    And the error message contains the exact substring "Work unit 'AUTH-999' does not exist"

  Scenario: Empty args object is rejected as InvalidArgs mentioning the missing workUnitId
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-virtual-hook with an empty args object {}
    Then the dispatcher returns success=false
    And the error message indicates that workUnitId is required

  Scenario: Hook name derivation strips path prefix and trailing arguments
    Given spec/work-units.json contains AUTH-001 with no virtualHooks
    When I dispatch add-virtual-hook with command='npm run lint'
    Then the stored hook has name='npm'
    When I dispatch add-virtual-hook with command='eslint src/'
    Then the stored hook has name='eslint'
    When I dispatch add-virtual-hook with command='/usr/bin/node script.js'
    Then the stored hook has name='node'

  Scenario: blocking and gitContext default to false when omitted
    Given spec/work-units.json contains AUTH-001 with no virtualHooks
    When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-validating' command='npm audit'
    Then the dispatcher returns success=true
    And the stored hook has blocking=false
    And the stored hook JSON does NOT include the key 'gitContext'

  Scenario: Adding to an existing empty virtualHooks array appends and returns hookCount=1
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[]
    When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='npm test'
    Then the dispatcher returns success=true
    And the parsed JSON has hookCount=1
    And the on-disk virtualHooks array has length 1

  Scenario: spec/work-units.json is auto-created when missing but lookup still fails
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='npm test'
    Then the dispatcher returns success=false
    And the error message contains the exact substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json exists after the call

  Scenario: Result JSON uses camelCase hookCount key
    Given spec/work-units.json contains AUTH-001 with no virtualHooks
    When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='npm test'
    Then the DispatchResult.data parses to a JSON object containing the key 'hookCount'
    And the DispatchResult.data does NOT contain the key 'hook_count'
    And the DispatchResult.data contains 'success' equal to true
