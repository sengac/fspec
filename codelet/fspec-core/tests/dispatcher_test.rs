#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/fspec-tool-rust-dispatcher.feature
//
// This test file validates the acceptance criteria for the synchronous
// Rust dispatcher introduced by TOOL-019. Scenarios map directly to the
// Gherkin scenarios in the feature file above.

use std::path::PathBuf;

use codelet_fspec_core::{dispatch_command, DispatchRequest};

/// Helper — build a DispatchRequest with empty args targeting a throwaway
/// project root. Phase 1 stubs never touch the filesystem, so the path is
/// irrelevant.
fn req(command: &str) -> DispatchRequest {
    DispatchRequest {
        command: command.to_string(),
        args_json: "{}".to_string(),
        project_root: PathBuf::from("/tmp/fspec-core-dispatcher-test"),
    }
}

#[test]
fn dispatcher_returns_not_yet_ported_for_known_unported_command() {
    // Scenario: Standalone Rust binary returns NotYetPorted error for a known unported command

    // @step Given the agent session is running inside the standalone fspec Rust binary
    // (no setup needed — dispatch_command is the standalone path by construction)

    // @step And the NAPI chunk callback is NOT registered (is_global_chunk_callback_registered returns false)
    // (precondition satisfied by reaching dispatch_command — agent_loop only delegates here on the false branch)

    // @step And the dispatcher's command map records "add-rule" as ported by RPC-XXX
    // (verified indirectly via the canonical_list test; here we trust the lookup.
    //  We deliberately pick a command that is still in the stub state — `audit-coverage`
    //  maps to RPC-197 in the per-command mapping and is NOT in PORTED_COMMANDS, so its
    //  stub still returns NotYetPorted. This test asserts the stub path; once
    //  `audit-coverage` itself is ported, swap to another unported canonical command.)
    let stub_command = "audit-coverage";
    let stub_rpc = "RPC-197";

    // @step When the LLM emits Fspec with command="add-rule" and any args_json
    let start = std::time::Instant::now();
    let result = dispatch_command(req(stub_command));
    let elapsed = start.elapsed();

    // @step Then the dispatcher returns FspecResult with success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    let msg = result
        .error
        .as_ref()
        .expect("expected an error message for unported command");

    // @step And the error message contains the literal substring "add-rule"
    assert!(
        msg.contains(stub_command),
        "missing '{stub_command}' in error message: {msg}"
    );

    // @step And the error message contains the literal substring "not yet ported"
    assert!(
        msg.contains("not yet ported"),
        "missing 'not yet ported' in error message: {msg}"
    );

    // @step And the error message contains the porting work unit ID "RPC-XXX"
    let rpc_pattern = regex::Regex::new(r"RPC-\d+").expect("valid regex");
    assert!(
        rpc_pattern.is_match(msg),
        "missing RPC-### work unit ID in error message: {msg}"
    );
    assert!(
        msg.contains(stub_rpc),
        "expected {stub_command}'s mapped work unit {stub_rpc} in error message: {msg}"
    );

    // @step And the error message contains the substring "standalone fspec binary"
    assert!(
        msg.contains("standalone fspec binary"),
        "missing 'standalone fspec binary' in error message: {msg}"
    );

    // @step And the agent loop emits a normal Done chunk for the turn within 1 second
    // (proxy assertion: dispatch_command itself returns synchronously well under 1s.
    //  The Done-chunk emission is the agent_loop's responsibility after this result
    //  is converted to FspecHandlerResult — verified by the agent_loop integration
    //  test in tests/agent_loop_wiring_test.rs.)
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "dispatch_command took {elapsed:?}, expected <1s"
    );

    // @step And the agent loop does NOT hang waiting for a JS callback
    // (proxy assertion: dispatch_command returned without blocking on any external
    //  callback channel — there is no JS callback path in this code path at all.)
    assert!(
        result.error.is_some(),
        "result must be a structured error, not a blocking sentinel"
    );
}

#[test]
fn dispatcher_returns_unknown_command_for_unrecognized_name() {
    // Scenario: Standalone Rust binary returns UnknownCommand error for a name not in the map

    // @step Given the agent session is running inside the standalone fspec Rust binary
    // @step And the NAPI chunk callback is NOT registered
    // @step And the dispatcher's command map has no entry for "totally-made-up-command"

    // @step When the LLM emits Fspec with command="totally-made-up-command"
    let result = dispatch_command(req("totally-made-up-command"));

    // @step Then the dispatcher returns FspecResult with success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    let msg = result
        .error
        .as_ref()
        .expect("expected an error message for unknown command");

    // @step And the error message contains the literal substring "Unknown fspec command"
    assert!(
        msg.contains("Unknown fspec command"),
        "missing 'Unknown fspec command' in error message: {msg}"
    );

    // @step And the error message contains the literal substring "totally-made-up-command"
    assert!(
        msg.contains("totally-made-up-command"),
        "missing offending command name in error message: {msg}"
    );

    // @step And the error message does NOT contain the substring "not yet ported"
    assert!(
        !msg.contains("not yet ported"),
        "UnknownCommand message must not mention 'not yet ported': {msg}"
    );

    // @step And the agent loop emits a normal Done chunk for the turn
    // (proxy assertion: dispatch_command returned synchronously without blocking)
    // — see the agent_loop integration test for the wire-up.
}

#[test]
#[ignore = "@ignore — NAPI chunk-callback delegation path is wired in the agent_loop integration step (out of scope for TOOL-019 scaffolding)"]
fn napi_node_hosted_cli_delegates_back_to_typescript_transparently() {
    // Scenario: NAPI Node-hosted CLI delegates back to TypeScript transparently
    //
    // @step Given the agent session is running inside the NAPI Node-hosted CLI
    // @step And the NAPI chunk callback IS registered (is_global_chunk_callback_registered returns true)
    // @step When the LLM emits Fspec with command="add-rule" and valid args_json
    // @step Then the dispatcher delegates to the existing chunk-callback protocol unchanged
    // @step And the FspecResult returned to the agent has success=true with the TypeScript command output
    // @step And the dispatcher does NOT consult the stub command map for the delegated path
    unimplemented!("delegation wired in agent_loop closure; verified by agent_loop wiring test");
}

#[test]
#[ignore = "@ignore — pre_tool_use hook short-circuit happens at the FspecToolFacadeWrapper layer above the dispatcher (out of scope for TOOL-019 scaffolding)"]
fn pre_tool_use_hook_short_circuits_dispatch() {
    // Scenario: pre_tool_use hook short-circuits dispatch
    //
    // @step Given the agent session has a pre_tool_use hook registered that denies "delete-work-unit"
    // @step And the agent session is running inside the standalone fspec Rust binary
    // @step When the LLM emits Fspec with command="delete-work-unit"
    // @step Then FspecToolFacadeWrapper::call returns Err(ToolError::Blocked) with the hook's reason
    // @step And the dispatcher is never invoked for this turn
    unimplemented!("verified at the FspecToolFacadeWrapper layer");
}

// ===========================================================================
// Below: canonical-list and source-shape tests merged from the original
// canonical_list_test.rs (1 feature → 1 test file invariant for ACDD).
// ===========================================================================

use std::fs;
use std::path::Path;

use codelet_fspec_core::canonical::{is_ported, lookup, CANONICAL_COMMANDS};
use serde_json::Value;

// ---------- path helpers ----------

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn attachment_path() -> PathBuf {
    workspace_root()
        .join("spec")
        .join("attachments")
        .join("TOOL-019")
        .join("canonical-commands.json")
}

fn mapping_path() -> PathBuf {
    workspace_root()
        .join("spec")
        .join("attachments")
        .join("TOOL-019")
        .join("command-to-rpc-mapping.json")
}

fn work_units_json_path() -> PathBuf {
    workspace_root().join("spec").join("work-units.json")
}

fn commands_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("commands")
}

fn agent_loop_path() -> PathBuf {
    workspace_root()
        .join("codelet")
        .join("agent-loop")
        .join("src")
        .join("agent_loop.rs")
}

fn agent_loop_cargo_path() -> PathBuf {
    workspace_root()
        .join("codelet")
        .join("agent-loop")
        .join("Cargo.toml")
}

fn snake_case_module(name: &str) -> String {
    name.replace('-', "_")
}

// ---------- tests ----------

#[test]
fn canonical_commands_count_matches_attachment() {
    // Scenario lineage: "Every command stub file exists with the canonical template"
    // + canonical list source-of-truth invariant.

    // @step Given the canonical command list contains 162 names
    let attachment = attachment_path();
    let raw = fs::read_to_string(&attachment)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", attachment.display()));
    let json: Value = serde_json::from_str(&raw).expect("attachment JSON should parse");

    let commands = json
        .get("commands")
        .and_then(Value::as_array)
        .expect("attachment JSON should have a top-level 'commands' array");

    assert_eq!(
        commands.len(),
        162,
        "attachment should contain exactly 162 commands, found {}",
        commands.len()
    );
    assert_eq!(
        CANONICAL_COMMANDS.len(),
        162,
        "CANONICAL_COMMANDS should contain exactly 162 entries, found {}",
        CANONICAL_COMMANDS.len()
    );

    for entry in commands {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .expect("each command entry should have a 'name' field");
        assert!(
            lookup(name).is_some(),
            "canonical command '{name}' from attachment is missing from CANONICAL_COMMANDS"
        );
    }
}

#[test]
fn every_canonical_command_has_a_module_or_is_stubbed() {
    // Scenario: Every command stub file exists with the canonical template

    // @step Given the canonical command list contains 162 names
    let dir = commands_dir();
    let mod_rs = dir.join("mod.rs");
    let mod_src = fs::read_to_string(&mod_rs)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", mod_rs.display()));

    let mut missing_files: Vec<String> = Vec::new();
    let mut missing_modules: Vec<String> = Vec::new();
    let mut missing_run_fn: Vec<String> = Vec::new();
    let mut missing_not_yet_ported: Vec<String> = Vec::new();
    let mut wrong_work_unit: Vec<String> = Vec::new();

    // Load the canonical mapping (command → RPC-XXX) so we can verify each
    // stub embeds the right work-unit ID.
    let mapping_raw = fs::read_to_string(mapping_path()).expect("mapping JSON readable");
    let mapping_json: Value = serde_json::from_str(&mapping_raw).expect("mapping JSON parses");
    let mapping = mapping_json
        .get("mapping")
        .and_then(Value::as_object)
        .expect("mapping JSON has 'mapping' object");

    // @step When I list files matching codelet/fspec-core/src/commands/*.rs
    for cmd in CANONICAL_COMMANDS {
        let module = snake_case_module(cmd.name);
        let path = dir.join(format!("{module}.rs"));

        // @step Then exactly 162 files exist (one per command, file name = command name with hyphens replaced by underscores)
        if !path.exists() {
            missing_files.push(cmd.name.to_string());
            continue;
        }

        // @step And every file is registered in codelet/fspec-core/src/commands/mod.rs
        if !mod_src.contains(&format!("pub mod {module};")) {
            missing_modules.push(cmd.name.to_string());
        }

        let body = fs::read_to_string(&path).expect("stub file readable");

        // Ported commands no longer have to satisfy stub-shape invariants —
        // they ship a real `pub async fn run` returning real data and
        // legitimately don't contain `NotYetPorted`. The canonical list of
        // ported commands lives in `crate::canonical::PORTED_COMMANDS`.
        if is_ported(cmd.name) {
            // Sanity check: ported commands MUST still have a `pub async fn run`,
            // it just has a different signature/body. The shape contract is
            // verified by each command's own integration test file.
            if !body.contains("pub async fn run") {
                missing_run_fn.push(cmd.name.to_string());
            }
            continue;
        }

        // @step And each file declares a "pub async fn run" returning Result<String, FspecCoreError>
        if !body.contains("pub async fn run") || !body.contains("Result<String, FspecCoreError>") {
            missing_run_fn.push(cmd.name.to_string());
        }

        // @step And each file's body returns Err(FspecCoreError::NotYetPorted { command, work_unit }) with the matching child work unit ID
        if !body.contains("FspecCoreError::NotYetPorted") {
            missing_not_yet_ported.push(cmd.name.to_string());
        }

        let expected_rpc = mapping
            .get(cmd.name)
            .and_then(Value::as_str)
            .unwrap_or("RPC-PENDING");
        if !body.contains(expected_rpc) {
            wrong_work_unit.push(format!("{}: expected {expected_rpc}", cmd.name));
        }
    }

    assert!(
        missing_files.is_empty(),
        "{} stub file(s) missing under {}: {missing_files:?}",
        missing_files.len(),
        dir.display()
    );
    assert!(
        missing_modules.is_empty(),
        "{} stub(s) not registered in commands/mod.rs: {missing_modules:?}",
        missing_modules.len()
    );
    assert!(
        missing_run_fn.is_empty(),
        "{} stub(s) missing pub async fn run -> Result<String, FspecCoreError>: {missing_run_fn:?}",
        missing_run_fn.len()
    );
    assert!(
        missing_not_yet_ported.is_empty(),
        "{} stub(s) missing FspecCoreError::NotYetPorted body: {missing_not_yet_ported:?}",
        missing_not_yet_ported.len()
    );
    assert!(
        wrong_work_unit.is_empty(),
        "{} stub(s) embed the wrong RPC work-unit ID: {wrong_work_unit:?}",
        wrong_work_unit.len()
    );
}

#[test]
fn every_canonical_command_has_a_child_work_unit_under_rpc_003() {
    // Scenario: Every TypeScript command name has a child work unit under RPC-003

    // @step Given the canonical command list extracted from src/cli/program.ts contains 162 names
    let mapping_raw = fs::read_to_string(mapping_path()).expect("mapping JSON readable");
    let mapping_json: Value = serde_json::from_str(&mapping_raw).expect("mapping JSON parses");
    let mapping = mapping_json
        .get("mapping")
        .and_then(Value::as_object)
        .expect("mapping JSON has 'mapping' object");
    assert_eq!(
        mapping.len(),
        162,
        "mapping should contain 162 entries, got {}",
        mapping.len()
    );

    // @step When I query work units with parent="RPC-003" and epic="rust-cli-port"
    let wu_raw = fs::read_to_string(work_units_json_path())
        .unwrap_or_else(|e| panic!("failed to read spec/work-units.json: {e}"));
    let wu_json: Value = serde_json::from_str(&wu_raw).expect("work-units.json parses");
    let work_units = wu_json
        .get("workUnits")
        .and_then(Value::as_object)
        .expect("work-units.json has 'workUnits' map");

    // Collect all child cards under the RPC IDs the mapping points at.
    let mut by_command: Vec<(String, &Value)> = Vec::with_capacity(162);
    for (command, rpc_value) in mapping {
        let rpc_id = rpc_value.as_str().expect("rpc-id string");
        let unit = work_units.get(rpc_id).unwrap_or_else(|| {
            panic!("expected work unit {rpc_id} for command {command} to exist")
        });
        by_command.push((command.clone(), unit));
    }

    // @step Then exactly one child work unit exists for each of the 162 command names
    assert_eq!(by_command.len(), 162);

    let mut bad_title: Vec<String> = Vec::new();
    let mut bad_description: Vec<String> = Vec::new();
    let mut bad_status: Vec<String> = Vec::new();
    let mut bad_estimate: Vec<String> = Vec::new();

    for (command, unit) in &by_command {
        let title = unit.get("title").and_then(Value::as_str).unwrap_or("");
        let description = unit
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let status = unit.get("status").and_then(Value::as_str).unwrap_or("");
        let estimate = unit.get("estimate");

        // @step And every child work unit's title matches the pattern "Port <command> command to Rust"
        let expected_title = format!("Port {command} command to Rust");
        if title != expected_title {
            bad_title.push(format!("{command}: title='{title}'"));
        }

        // @step And every child work unit's description references the TypeScript source file path
        let canonical = lookup(command).expect("canonical entry");
        if !description.contains(canonical.ts_file) {
            bad_description.push(format!(
                "{command}: description missing {}",
                canonical.ts_file
            ));
        }

        // Ported commands (PORTED_COMMANDS) are exempt from the
        // "status=backlog, estimate=null" invariants — once a port is in
        // progress / done, those values legitimately change. They're still
        // required to match title / description shape above.
        if is_ported(command) {
            continue;
        }

        // @step And every child work unit's status is "backlog"
        if status != "backlog" {
            bad_status.push(format!("{command}: status='{status}'"));
        }

        // @step And no child work unit has an estimate set
        if let Some(v) = estimate {
            if !v.is_null() {
                bad_estimate.push(format!("{command}: estimate={v}"));
            }
        }
    }

    assert!(
        bad_title.is_empty(),
        "{} card(s) with wrong title: {bad_title:?}",
        bad_title.len()
    );
    assert!(
        bad_description.is_empty(),
        "{} card(s) with description missing TS source path: {bad_description:?}",
        bad_description.len()
    );
    assert!(
        bad_status.is_empty(),
        "{} card(s) with non-backlog status: {bad_status:?}",
        bad_status.len()
    );
    assert!(
        bad_estimate.is_empty(),
        "{} card(s) with an estimate set: {bad_estimate:?}",
        bad_estimate.len()
    );
}

#[test]
fn agent_loop_fallback_wires_codelet_fspec_core_dispatch_command() {
    // Scenario: agent_loop falls back to the Rust dispatcher when the NAPI chunk callback is absent

    // @step Given the agent_loop.rs source after this work unit is applied
    let src = fs::read_to_string(agent_loop_path()).expect("agent_loop.rs readable");

    // @step When I inspect the fspec_handler closure passed to set_fspec_handler_for_session
    // (we look at the source verbatim — full agent_loop e2e is out of scope here)

    // @step Then the closure delegates to codelet_fspec_core::dispatch_command on the !is_global_chunk_callback_registered() branch
    assert!(
        src.contains("codelet_fspec_core::dispatch_command"),
        "agent_loop.rs must call codelet_fspec_core::dispatch_command after TOOL-019"
    );
    assert!(
        src.contains("!is_global_chunk_callback_registered()"),
        "agent_loop.rs must keep the !is_global_chunk_callback_registered() branch as the standalone-binary fallback"
    );

    // @step And the closure does NOT return the legacy string "Global chunk callback not registered"
    assert!(
        !src.contains("Global chunk callback not registered"),
        "agent_loop.rs must no longer emit the legacy 'Global chunk callback not registered' error string after TOOL-019"
    );

    // @step And the codelet-agent-loop crate's Cargo.toml declares codelet-fspec-core as a workspace dependency
    let cargo = fs::read_to_string(agent_loop_cargo_path()).expect("Cargo.toml readable");
    assert!(
        cargo.contains("codelet-fspec-core.workspace = true")
            || cargo.contains("codelet-fspec-core = { workspace = true }"),
        "codelet-agent-loop Cargo.toml must declare codelet-fspec-core as a workspace dep"
    );
}
