@done
@rust
@cli
@RPC-252
Feature: Port list-virtual-hooks command to Rust
  """
  New impl file at rust/fspec-core/src/commands/list_virtual_hooks.rs replaces the NotYetPorted stub. The module exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` with the same signature shape as list_work_units::run. Args struct deserializes `{workUnitId: String, format?: 'text'|'json'}` with `#[serde(default)]` on format only — workUnitId is REQUIRED and parse failures surface as InvalidArgs.

  The shell-facing CLI bridge is now delivered as part of RPC-252 — see rust/fspec/src/list_virtual_hooks.rs and spec/features/list-virtual-hooks-cli-subcommand.feature. Both front doors (LLM tool-call dispatcher and clap shell subcommand) call the same `list_virtual_hooks::run` function defined in this port.

  The command reads `spec/work-units.json` via the shared `crate::io::ensure::ensure_work_units_file` helper (parity with `list-work-units` Rust port — auto-creates the file with empty store if missing). It then looks up the requested work unit by id; if not present, returns `FspecCoreError::InvalidArgs { reason: "Work unit '<id>' does not exist" }` (the dispatcher converts this to success=false with that error string).

  When the work unit exists, hooks are read from `wu.extra["virtualHooks"]` (a `serde_json::Value::Array`) — we do NOT add `virtualHooks` to the typed `WorkUnit` struct because the rest of the Rust port reuses the existing `WorkUnit` shape via the `extra` flatten map. The hooks list is iterated in insertion order, grouped into `hooksByEvent: IndexMap<String, Vec<VirtualHook>>` keyed by the `event` field, preserving both event-introduction order AND within-event hook order.

  Output shape: `{hooks: VirtualHook[], hooksByEvent: {event -> VirtualHook[]}}`. JSON format uses 2-space indent. Text format renders `No virtual hooks configured for <workUnitId>` for the empty case and `Virtual Hooks for <workUnitId>:` + per-event sections with `[blocking]`/`[non-blocking]` + optional `[git-context]` badges otherwise.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rust dispatcher route for list-virtual-hooks replaces the NotYetPorted stub
  #   2. workUnitId is a required argument; missing → InvalidArgs
  #   3. Unknown workUnitId → success=false with error "Work unit '<id>' does not exist"
  #   4. Missing or empty virtualHooks → hooks=[], hooksByEvent={}, success=true
  #   5. Hooks grouped by event field, preserving insertion order within each event
  #   6. VirtualHook fields: {name, event, command, blocking, gitContext?}
  #   7. JSON format = 2-space-indented {hooks:[...], hooksByEvent:{...}}
  #   8. Text format empty = exact sentinel "No virtual hooks configured for <id>"
  #   9. Text format populated = header + per-event sections + badges
  #   10. Default format (no format key) is text
  #   11. CLI delegates to single source of truth in fspec_core
  #   12. ensureWorkUnitsFile auto-creates work-units.json on missing — but lookup
  #       still fails for the requested id because the new store is empty
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-virtual-hooks from the agent loop AND invoke `fspec list-virtual-hooks <workUnitId>` from a shell
    So that I can audit work-unit-scoped virtual hooks grouped by event, sharing one source-of-truth between the LLM dispatcher and the CLI without going through Node.js

  Scenario: Returns error when the requested work unit does not exist
    Given spec/work-units.json contains AUTH-001 with no virtualHooks
    When I dispatch list-virtual-hooks with workUnitId='AUTH-999' and format='json'
    Then the dispatcher returns success=false
    Then the error message contains the exact substring "Work unit 'AUTH-999' does not exist"

  Scenario: Returns error when spec/work-units.json is auto-created and the requested id is not in the empty store
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=false
    Then the error message contains the exact substring "Work unit 'AUTH-001' does not exist"
    Then spec/work-units.json exists after the call

  Scenario: Returns empty hooks and empty hooksByEvent when work unit has no virtualHooks field
    Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has hooks array of length 0
    Then the parsed JSON has hooksByEvent as an empty object

  Scenario: Returns empty hooks and empty hooksByEvent when virtualHooks is an empty array
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[]
    When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has hooks array of length 0
    Then the parsed JSON has hooksByEvent as an empty object

  Scenario: Groups hooks by event preserving insertion order across and within events
    Given spec/work-units.json contains AUTH-001 with virtualHooks in order: 'lint' (post-implementing, blocking=true), 'test' (post-implementing, blocking=false), 'eslint' (pre-validating, blocking=true, gitContext=true)
    When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has hooks array of length 3 in the order lint, test, eslint
    Then hooksByEvent contains key 'post-implementing' with hook names ['lint','test'] in that order
    Then hooksByEvent contains key 'pre-validating' with hook names ['eslint']
    Then hooksByEvent key order is 'post-implementing' then 'pre-validating'

  Scenario: Each VirtualHook entry includes name, event, command, blocking and optional gitContext
    Given spec/work-units.json contains AUTH-001 with one virtualHook {name:'eslint', event:'pre-validating', command:'eslint .', blocking:true, gitContext:true}
    When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the first hook has name='eslint' and event='pre-validating' and command='eslint .' and blocking=true and gitContext=true

  Scenario: JSON format emits two-space indented payload
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[]
    When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    Then the DispatchResult.data starts with the exact string "{\n  \"hooks\": [],\n"
    Then the DispatchResult.data contains the exact substring "\"hooksByEvent\": {}"

  Scenario: Text format renders the empty sentinel including the work unit id
    Given spec/work-units.json contains AUTH-001 with no virtualHooks
    When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data is exactly the string "No virtual hooks configured for AUTH-001"

  Scenario: Text format renders the populated case with header, event sections, and badges
    Given spec/work-units.json contains AUTH-001 with virtualHooks in order: 'lint' (post-implementing, blocking=true), 'test' (post-implementing, blocking=false), 'eslint' (pre-validating, blocking=true, gitContext=true)
    When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the exact substring "Virtual Hooks for AUTH-001:"
    Then the substring 'post-implementing:' appears before 'pre-validating:' in the output
    Then the DispatchResult.data contains the substring "[blocking]"
    Then the DispatchResult.data contains the substring "[non-blocking]"
    Then the DispatchResult.data contains the substring "[git-context]"
    Then the DispatchResult.data contains the substring "lint"
    Then the DispatchResult.data contains the substring "test"
    Then the DispatchResult.data contains the substring "eslint"

  Scenario: Default format (no format key supplied) is text
    Given spec/work-units.json contains AUTH-001 with no virtualHooks
    When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and no format key
    Then the dispatcher returns success=true
    Then the DispatchResult.data is exactly the string "No virtual hooks configured for AUTH-001"

  Scenario: Missing workUnitId argument is rejected as InvalidArgs
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch list-virtual-hooks with an empty args object {}
    Then the dispatcher returns success=false
    Then the error message indicates that workUnitId is required
