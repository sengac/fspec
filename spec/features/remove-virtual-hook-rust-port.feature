@done
@cli
@rust
@RPC-283
Feature: Port remove-virtual-hook command to Rust
  """
  Two-front-doors invariant (RPC-003 §7/§11): both the LLM-facing dispatcher and the clap shell subcommand invoke fspec_core::commands::remove_virtual_hook::run with a JSON args string and &Path project_root.
  Removal uses Vec::retain on the wu.extra["virtualHooks"] array (no clone needed). After retain, compare new length vs initial — if equal, return InvalidArgs("Virtual hook '...' not found in ...").
  Script cleanup helper inlined as cleanup_virtual_hook_script(work_unit_id, hook_name, project_root) — uses std::fs::remove_file and silently swallows ALL errors (parity with TS try/catch {} wrapper).
  Atomic write: single ensure_work_units_file load → in-memory retain → script cleanup (best-effort) → single write_json_atomic. Script cleanup happens BEFORE the work-units write — matching TS source-of-truth order.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rust dispatcher route for remove-virtual-hook replaces the NotYetPorted stub
  #   2. workUnitId and hookName are required arguments; missing → InvalidArgs
  #   3. Unknown workUnitId → success=false with error "Work unit '<id>' does not exist"
  #   4. When virtualHooks is missing or empty → success=false with error "No virtual hooks configured for <workUnitId>"
  #   5. Removes all hooks whose name === hookName (filter semantics, not just first match) — parity with TS .filter() call
  #   6. When no hook is removed (length unchanged) → success=false with error "Virtual hook '<hookName>' not found in <workUnitId>"
  #   7. Best-effort cleanup of spec/hooks/.virtual/<workUnitId>-<hookName>.sh — all I/O errors silently swallowed
  #   8. On success, workUnit.updatedAt is bumped to the current ISO8601 timestamp and spec/work-units.json is rewritten atomically
  #   9. DispatchResult.data shape: {"success":true,"remainingCount":<n>} (camelCase remainingCount)
  #   10. CLI delegates to single source of truth in fspec_core::commands::remove_virtual_hook::run
  #
  # EXAMPLES:
  #   1. Removing 'eslint' from AUTH-001 which has [{name:'eslint',event:'post-implementing'}] yields remainingCount=0 and an empty virtualHooks array
  #   2. Removing 'lint' from AUTH-001 which has [lint,test,eslint] preserves [test,eslint] in order and returns remainingCount=2
  #   3. Removing 'lint' when two hooks share that name removes BOTH (filter semantics) and the work unit ends with remainingCount = original − 2
  #   4. Removing a hook that has an associated script at spec/hooks/.virtual/AUTH-001-eslint.sh deletes that script file
  #   5. Removing a hook whose script file does not exist succeeds silently (no error from missing script)
  #   6. Dispatching with workUnitId='AUTH-999' returns success=false with message "Work unit 'AUTH-999' does not exist"
  #   7. Dispatching for a work unit with no virtualHooks field returns success=false with "No virtual hooks configured for AUTH-001"
  #   8. Dispatching with a hookName that does not match any entry returns success=false with "Virtual hook 'missing' not found in AUTH-001"
  #   9. Dispatching with empty args object {} returns success=false and the error mentions missing workUnitId
  #   10. CLI subcommand: `fspec remove-virtual-hook AUTH-001 eslint` prints '✓ Removed virtual hook ...' and '  Remaining virtual hooks: 0' and exits 0
  #   11. CLI subcommand with empty virtualHooks prints '✗ Failed to remove virtual hook: No virtual hooks configured ...' to stderr and exits 1
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch remove-virtual-hook from the agent loop AND invoke `fspec remove-virtual-hook <workUnitId> <hookName>` from a shell
    So that I can detach a named virtual hook (and clean up any associated git-context script file) from spec/work-units.json using one source of truth shared by the dispatcher and the CLI, without going through Node.js

  Scenario: Removes the only hook and returns remainingCount=0
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',event:'post-implementing',command:'eslint .',blocking:true}]
    When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    Then the dispatcher returns success=true
    And the parsed JSON has remainingCount=0
    And the on-disk virtualHooks array has length 0

  Scenario: Removing a middle entry preserves the order of remaining hooks
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[lint,test,eslint] in that order
    When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='lint'
    Then the dispatcher returns success=true
    And the parsed JSON has remainingCount=2
    And the on-disk virtualHooks names in order are ['test','eslint']

  Scenario: Removing a hook with duplicate names removes ALL matches (filter semantics)
    Given spec/work-units.json contains AUTH-001 with two virtualHooks both named 'lint' plus one named 'test'
    When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='lint'
    Then the dispatcher returns success=true
    And the parsed JSON has remainingCount=1
    And the on-disk virtualHooks names in order are ['test']

  Scenario: Removing a hook with an associated script deletes the script file
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',event:'post-implementing',command:'spec/hooks/.virtual/AUTH-001-eslint.sh',blocking:true,gitContext:true}]
    And the file spec/hooks/.virtual/AUTH-001-eslint.sh exists on disk
    When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    Then the dispatcher returns success=true
    And the file spec/hooks/.virtual/AUTH-001-eslint.sh no longer exists

  Scenario: Removing a hook whose script file does not exist succeeds silently
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',event:'post-implementing',command:'eslint .',blocking:true}]
    And spec/hooks/.virtual/AUTH-001-eslint.sh does NOT exist on disk
    When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    Then the dispatcher returns success=true
    And the parsed JSON has remainingCount=0

  Scenario: Unknown work unit returns InvalidArgs with the canonical message
    Given spec/work-units.json contains AUTH-001
    When I dispatch remove-virtual-hook with workUnitId='AUTH-999' hookName='eslint'
    Then the dispatcher returns success=false
    And the error message contains the exact substring "Work unit 'AUTH-999' does not exist"

  Scenario: Work unit without virtualHooks field returns InvalidArgs
    Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    Then the dispatcher returns success=false
    And the error message contains the exact substring "No virtual hooks configured for AUTH-001"

  Scenario: Work unit with empty virtualHooks array also returns InvalidArgs
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[]
    When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    Then the dispatcher returns success=false
    And the error message contains the exact substring "No virtual hooks configured for AUTH-001"

  Scenario: Non-matching hookName returns InvalidArgs naming both the hook and the work unit
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',event:'post-implementing',command:'eslint .',blocking:true}]
    When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='missing'
    Then the dispatcher returns success=false
    And the error message contains the exact substring "Virtual hook 'missing' not found in AUTH-001"

  Scenario: Empty args object is rejected as InvalidArgs mentioning missing workUnitId
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch remove-virtual-hook with an empty args object {}
    Then the dispatcher returns success=false
    And the error message indicates that workUnitId is required

  Scenario: Result JSON uses camelCase remainingCount key
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',event:'post-implementing',command:'eslint .',blocking:true}]
    When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    Then the DispatchResult.data parses to a JSON object containing the key 'remainingCount'
    And the DispatchResult.data does NOT contain the key 'remaining_count'
    And the DispatchResult.data contains 'success' equal to true
