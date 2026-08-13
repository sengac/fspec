@done
@agent-loop
@tool-execution
@bug-fix
@dispatch
@rust
@tools
@TOOL-019
Feature: FspecTool::call is a NAPI stub — hangs agent loop on every Fspec tool dispatch in standalone Rust binary
  """
  New crate rust/fspec-core is the future home of the Rust port; this work creates the empty skeleton with stub command modules and a single dispatcher entry point
  Dispatcher signature: fn dispatch_command(name: &str, args_json: &str, project_root: &Path) -> FspecResult — pure synchronous fn called from the registered FspecHandler closure
  Each command stub file follows the template: pub async fn run(args: &str, ctx: &Context) -> Result<String, FspecCoreError> { Err(FspecCoreError::NotYetPorted { command: "add-rule", work_unit: "RPC-XXX" }) }
  Command-name-to-stub mapping uses a static const phf::Map<&'static str, StubFn> or an exhaustive match arm. Phase 1 just maps name → (command, work_unit_id) for the canonical error message; future ports replace the stub with the real implementation.
  Child cards under RPC-003 share the epic 'rust-cli-port' and follow naming pattern 'Port <command> command to Rust'. Description references TS source file path and approximate LOC. Estimate based on rpc-003-feasibility.md complexity tiers: trivial (1-2pts), thin-wrapper (2-3pts), top-15 algorithmic (5-13pts).
  Source of truth for command list: src/cli/program.ts registerXxxCommand calls + actual .command('name') strings in each command file. 162 commands identified (see grep output and program.ts inspection).
  agent_loop.rs fspec_handler closure becomes a single delegation: codelet_fspec_core::dispatch_command(...). The dispatcher INSIDE codelet_fspec_core decides whether to use the existing chunk-callback path (NAPI present) or serve the stub (NAPI absent). The closure no longer contains the is_global_chunk_callback_registered() branch — it lives one level deeper.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every fspec command name registered in src/cli/program.ts MUST have a corresponding Rust stub function in rust/fspec-core/src/commands/<command-name>.rs
  #   2. Every fspec command MUST have a dedicated child work unit under RPC-003 (epic 'rust-cli-port') tracking its future port to Rust
  #   3. The stub error message MUST tell the agent (a) the command is not yet ported to Rust, (b) the work unit ID porting it, and (c) that the standalone fspec binary lacks the TypeScript runtime
  #   4. The agent loop MUST NOT hang waiting for a non-existent JS callback; FspecHandler invocations always return synchronously with a structured result
  #   5. Unknown command names (typos, removed commands) MUST return a distinct error variant identifying the command as unrecognized, separate from the 'not yet ported' message
  #   6. The Rust dispatcher MUST be registered as the unconditional FspecHandler for every session: when NAPI TSFN is present it delegates back into TypeScript via the existing chunk-callback protocol; when absent (standalone Rust binary), it serves stubs. Single code path, no per-binary branching at registration time.
  #
  # EXAMPLES:
  #   1. Agent runs in standalone fspec binary and emits Fspec{command='add-rule', args='{...}'}: dispatcher returns FspecResult{success=false, error='Command add-rule is not yet ported to Rust (tracked by RPC-XXX). The standalone fspec binary cannot execute TypeScript fspec commands.'}, agent loop completes the turn, LLM sees the error and adapts
  #   2. Agent emits Fspec{command='unknown-command'}: dispatcher returns FspecResult{success=false, error='Unknown fspec command: unknown-command'}, distinct from the not-yet-ported message
  #   3. Agent emits Fspec with a hook-blocked command: pre_tool_use hook short-circuits FIRST returning ToolError::Blocked, dispatcher is never consulted
  #   4. Developer queries 'fspec list-work-units --parent RPC-003': output lists exactly the ~162 child cards, one per fspec command, each pointing back at its TypeScript source file
  #   5. Developer opens rust/fspec-core/src/commands/add_rule.rs: file contains a single pub async fn stub returning the canonical NotYetPorted error with the work unit ID embedded; identical skeleton for all ~162 command files
  #   6. Agent runs in NAPI Node-hosted CLI and emits Fspec{command='add-rule'}: dispatcher detects chunk-callback present, delegates back into TypeScript via the existing protocol, command executes in TypeScript — agent receives normal success result
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the new rust/fspec-core crate sit at the workspace root alongside rust/fspec, or be nested somewhere else? Naming this 'fspec-core' is aligned with rpc-003-feasibility.md §7.
  #   A: Leave child cards unestimated at bulk-creation time; each gets estimated when picked up during its own specifying phase.
  #
  #   Q: Should the dispatcher also be wired through the existing FspecToolFacadeWrapper for the NAPI Node path (so the same Rust dispatcher serves both)? Or strictly only as a fallback when the chunk callback is absent? Current plan: only fallback.
  #   A: Always wired through a single Rust dispatcher; the dispatcher itself decides delegation vs stub based on chunk-callback presence
  #
  #   Q: For the per-command child cards, should each get an estimate based on rpc-003-feasibility.md complexity tiers, or leave estimates blank until each card is picked up?
  #   A: Child cards are created without estimates at bulk-creation; each gets its estimate during its own specifying phase
  #
  # ASSUMPTIONS:
  #   1. Always wired through a single Rust dispatcher; the dispatcher itself decides delegation vs stub based on chunk-callback presence
  #   2. Child cards are created without estimates at bulk-creation; each gets its estimate during its own specifying phase
  #
  # ========================================
  Background: User Story
    As a agent operator running the standalone fspec Rust binary
    I want to have Fspec tool dispatches return clean per-command errors instead of a generic 'callback not registered' failure that hangs the loop
    So that the LLM can recover, see exactly which command was attempted and which work unit ports it, and continue the session

  Scenario: NotYetPorted error renders the full agent-facing contract when constructed directly
    Given the TS to Rust port is complete so no canonical command reaches the NotYetPorted path via dispatch
    And the NotYetPorted error variant is retained as a safety mechanism in the public API
    When a NotYetPorted error is constructed directly with command "some-cmd" and work unit "RPC-999"
    Then rendering the error yields a message containing the literal substring "some-cmd"
    And the error message contains the literal substring "not yet ported"
    And the error message contains the porting work unit ID "RPC-999"
    And the error message contains the substring "standalone fspec binary"
    And the invariant holds that every command in CANONICAL_COMMANDS reports is_ported true

  Scenario: Standalone Rust binary returns UnknownCommand error for a name not in the map
    Given the agent session is running inside the standalone fspec Rust binary
    And the NAPI chunk callback is NOT registered
    And the dispatcher's command map has no entry for "totally-made-up-command"
    When the LLM emits Fspec with command="totally-made-up-command"
    Then the dispatcher returns FspecResult with success=false
    And the error message contains the literal substring "Unknown fspec command"
    And the error message contains the literal substring "totally-made-up-command"
    And the error message does NOT contain the substring "not yet ported"
    And the agent loop emits a normal Done chunk for the turn

  Scenario: NAPI Node-hosted CLI delegates back to TypeScript transparently
    Given the agent session is running inside the NAPI Node-hosted CLI
    And the NAPI chunk callback IS registered (is_global_chunk_callback_registered returns true)
    When the LLM emits Fspec with command="add-rule" and valid args_json
    Then the dispatcher delegates to the existing chunk-callback protocol unchanged
    And the FspecResult returned to the agent has success=true with the TypeScript command output
    And the dispatcher does NOT consult the stub command map for the delegated path

  Scenario: pre_tool_use hook short-circuits dispatch
    Given the agent session has a pre_tool_use hook registered that denies "delete-work-unit"
    And the agent session is running inside the standalone fspec Rust binary
    When the LLM emits Fspec with command="delete-work-unit"
    Then FspecToolFacadeWrapper::call returns Err(ToolError::Blocked) with the hook's reason
    And the dispatcher is never invoked for this turn

  Scenario: Every TypeScript command name has a child work unit under RPC-003
    Given the canonical command list extracted from src/cli/program.ts contains 162 names
    When I query work units with parent="RPC-003" and epic="rust-cli-port"
    Then exactly one child work unit exists for each of the 162 command names
    And every child work unit's title matches the pattern "Port <command> command to Rust"
    And every child work unit's description references the TypeScript source file path
    And every child work unit's status is "backlog"
    And no child work unit has an estimate set

  Scenario: Every command stub file exists with the canonical template
    Given the canonical command list contains 162 names
    When I list files matching rust/fspec-core/src/commands/*.rs
    Then exactly 162 files exist (one per command, file name = command name with hyphens replaced by underscores)
    And each file declares a "pub async fn run" returning Result<String, FspecCoreError>
    And each file's body returns Err(FspecCoreError::NotYetPorted { command, work_unit }) with the matching child work unit ID
    And every file is registered in rust/fspec-core/src/commands/mod.rs

  Scenario: agent_loop falls back to the Rust dispatcher when the NAPI chunk callback is absent
    Given the agent_loop.rs source after this work unit is applied
    When I inspect the fspec_handler closure passed to set_fspec_handler_for_session
    Then the closure delegates to codelet_fspec_core::dispatch_command on the !is_global_chunk_callback_registered() branch
    And the closure does NOT return the legacy string "Global chunk callback not registered"
    And the codelet-agent-loop crate's Cargo.toml declares codelet-fspec-core as a workspace dependency
