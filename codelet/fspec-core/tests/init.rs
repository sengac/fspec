#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/init-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `init` (RPC-239).
// Each scenario maps to exactly one #[test] function with @step comments
// mirroring the Gherkin steps verbatim.
//
// RED PHASE: the current core stub is 1-arg `run(args_json)` -> NotYetPorted,
// so every dispatch of `init` returns success=false with the NotYetPorted
// message. These tests assert the REAL ported behaviour, so they FAIL now —
// that is the correct red-phase state.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, agents: &[&str]) -> DispatchRequest {
    DispatchRequest {
        command: "init".to_string(),
        args_json: json!({ "agent": agents }).to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn read_to_string(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn read_config(root: &Path) -> Value {
    let raw = read_to_string(root, "spec/fspec-config.json");
    serde_json::from_str(&raw).expect("config is valid JSON")
}

/// Collect the `filesInstalled` array (as Strings) from the dispatcher data.
fn files_installed(data: &Value) -> Vec<String> {
    data["filesInstalled"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

// ---------- scenarios ----------

#[test]
fn installs_claude_agent_files_and_writes_the_config() {
    // Scenario: Installs claude agent files and writes the config

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the init command against that project root with agent list ['claude']
    let result = dispatch_command(req(tmp.path(), &["claude"]));

    // @step Then the dispatcher returns success=true and cancelled=false
    assert!(result.success, "expected success=true; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert_eq!(data["cancelled"].as_bool(), Some(false));

    // @step Then spec/CLAUDE.md exists in the project root
    assert!(tmp.path().join("spec/CLAUDE.md").exists());

    // @step Then .claude/commands/fspec.md exists in the project root
    assert!(tmp.path().join(".claude/commands/fspec.md").exists());

    // @step Then spec/fspec-config.json contains the agent field 'claude'
    assert_eq!(read_config(tmp.path())["agent"].as_str(), Some("claude"));

    // @step Then the filesInstalled array contains 'spec/CLAUDE.md' and '.claude/commands/fspec.md'
    let files = files_installed(&data);
    assert!(files.contains(&"spec/CLAUDE.md".to_string()), "got: {files:?}");
    assert!(
        files.contains(&".claude/commands/fspec.md".to_string()),
        "got: {files:?}"
    );
}

#[test]
fn installs_a_toml_format_agent_slash_command_file() {
    // Scenario: Installs a TOML-format agent slash command file

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch init against that project root with agent list ['gemini']
    let result = dispatch_command(req(tmp.path(), &["gemini"]));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then spec/GEMINI.md exists in the project root
    assert!(tmp.path().join("spec/GEMINI.md").exists());

    // @step Then .gemini/commands/fspec.toml exists in the project root
    assert!(tmp.path().join(".gemini/commands/fspec.toml").exists());

    // @step Then the file .gemini/commands/fspec.toml starts with the substring '[command]'
    let toml = read_to_string(tmp.path(), ".gemini/commands/fspec.toml");
    assert!(toml.starts_with("[command]"), "got:\n{toml}");
}

#[test]
fn strips_system_reminder_blocks_to_visible_instructions_for_non_claude_agents() {
    // Scenario: Strips system-reminder blocks to visible instructions for non-claude agents

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch init against that project root with agent list ['gemini']
    let result = dispatch_command(req(tmp.path(), &["gemini"]));
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the doc file spec/GEMINI.md does NOT contain the substring '<system-reminder>'
    let doc = read_to_string(tmp.path(), "spec/GEMINI.md");
    assert!(
        !doc.contains("<system-reminder>"),
        "doc must not contain raw system-reminder tags"
    );

    // @step Then the doc file spec/GEMINI.md contains the substring '**IMPORTANT:**'
    assert!(
        doc.contains("**IMPORTANT:**"),
        "doc must render visible IMPORTANT instructions"
    );
}

#[test]
fn replaces_all_template_placeholders_with_agent_specific_values() {
    // Scenario: Replaces all template placeholders with agent-specific values

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch init against that project root with agent list ['claude']
    let result = dispatch_command(req(tmp.path(), &["claude"]));
    assert!(result.success, "expected success=true; got {result:?}");

    let doc = read_to_string(tmp.path(), "spec/CLAUDE.md");

    // @step Then the doc file spec/CLAUDE.md does NOT contain the substring '{{AGENT_NAME}}'
    assert!(!doc.contains("{{AGENT_NAME}}"), "AGENT_NAME placeholder leaked");

    // @step Then the doc file spec/CLAUDE.md does NOT contain the substring '{{DOC_TEMPLATE}}'
    assert!(
        !doc.contains("{{DOC_TEMPLATE}}"),
        "DOC_TEMPLATE placeholder leaked"
    );

    // @step Then the doc file spec/CLAUDE.md does NOT contain the substring '{{SLASH_COMMAND_PATH}}'
    assert!(
        !doc.contains("{{SLASH_COMMAND_PATH}}"),
        "SLASH_COMMAND_PATH placeholder leaked"
    );
}

#[test]
fn installs_multiple_agents_in_order_and_records_only_the_first_in_config() {
    // Scenario: Installs multiple agents in order and records only the first in config

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch init against that project root with agent list ['claude', 'cursor']
    let result = dispatch_command(req(tmp.path(), &["claude", "cursor"]));
    assert!(result.success, "expected success=true; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then spec/CLAUDE.md and spec/CURSOR.md both exist in the project root
    assert!(tmp.path().join("spec/CLAUDE.md").exists());
    assert!(tmp.path().join("spec/CURSOR.md").exists());

    // @step Then the filesInstalled array contains all four installed paths
    let files = files_installed(&data);
    assert_eq!(files.len(), 4, "expected 4 installed paths; got: {files:?}");
    assert!(files.contains(&"spec/CLAUDE.md".to_string()));
    assert!(files.contains(&"spec/CURSOR.md".to_string()));

    // @step Then spec/fspec-config.json contains the agent field 'claude'
    assert_eq!(read_config(tmp.path())["agent"].as_str(), Some("claude"));
}

#[test]
fn rejects_an_unknown_agent_id_with_the_valid_id_listing() {
    // Scenario: Rejects an unknown agent id with the valid-id listing

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch init against that project root with agent list ['bogus']
    let result = dispatch_command(req(tmp.path(), &["bogus"]));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step Then the error message begins with the substring 'Unknown agent: bogus.'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Unknown agent: bogus."),
        "missing unknown-agent text; got: {msg}"
    );

    // @step Then the error message contains the substring 'Valid agent IDs:'
    assert!(
        msg.contains("Valid agent IDs:"),
        "missing valid-id listing; got: {msg}"
    );
}

#[test]
fn rejects_an_empty_agent_list_because_headless_selection_is_unsupported() {
    // Scenario: Rejects an empty agent list because headless selection is unsupported

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch init against that project root with an empty agent list
    let result = dispatch_command(req(tmp.path(), &[]));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step Then the error message contains the substring 'Interactive mode requires a TTY. Use --agent flag instead:'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Interactive mode requires a TTY. Use --agent flag instead:"),
        "missing TTY-guard text; got: {msg}"
    );
}

#[test]
fn preserves_existing_config_keys_when_overwriting_the_agent_field() {
    // Scenario: Preserves existing config keys when overwriting the agent field

    // @step Given a project root whose spec/fspec-config.json contains agent='cursor' and an extra key foo='bar'
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec")).expect("mkdir spec");
    fs::write(
        tmp.path().join("spec/fspec-config.json"),
        json!({ "agent": "cursor", "foo": "bar" }).to_string(),
    )
    .expect("write config");

    // @step When I dispatch init against that project root with agent list ['claude']
    let result = dispatch_command(req(tmp.path(), &["claude"]));

    // @step Then the dispatcher returns success=true and cancelled=false
    assert!(result.success, "expected success=true; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert_eq!(data["cancelled"].as_bool(), Some(false));

    // @step Then spec/fspec-config.json contains the agent field 'claude'
    let config = read_config(tmp.path());
    assert_eq!(config["agent"].as_str(), Some("claude"));

    // @step Then spec/fspec-config.json still contains the key foo with value 'bar'
    assert_eq!(config["foo"].as_str(), Some("bar"));
}

#[test]
fn shares_one_implementation_between_the_dispatcher_and_the_cli_bridge() {
    // Scenario: Shares one implementation between the dispatcher and the CLI bridge

    // @step Given the codelet/fspec-core crate is built
    // (Enforced at compile time.)

    // @step When I inspect codelet/fspec-core/src/commands/init.rs
    let core_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/init.rs");
    let core_src = fs::read_to_string(&core_path).expect("init.rs readable");

    // @step Then init::run scaffolds files via blocking std::fs and contains the inlined agent registry table
    assert!(
        core_src.contains("std::fs") || core_src.contains("create_dir_all"),
        "core impl must use blocking std::fs scaffolding"
    );
    assert!(
        core_src.contains("CLAUDE.md") && core_src.contains("GEMINI.md"),
        "core impl must inline the agent registry table"
    );

    // @step Then the CLI bridge codelet/fspec/src/init.rs delegates to init::run and contains no inline scaffolding or registry logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fspec/src/init.rs");
    let bridge_src = fs::read_to_string(&bridge_path).expect("CLI bridge init.rs readable");
    assert!(
        bridge_src.contains("init::run") || bridge_src.contains("commands::init"),
        "bridge must delegate to init::run"
    );
    for forbidden in ["CLAUDE.md", "create_dir_all", "AGENT_REGISTRY", "docTemplate"] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

#[test]
fn writes_the_codex_slash_command_under_an_injectable_home_directory() {
    // Scenario: Writes the codex slash command under an injectable HOME directory

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step Given the HOME environment variable points at a separate temporary directory
    let home = TempDir::new().expect("home tempdir");
    // The dispatcher reads HOME from the process env (injectable source, never a
    // hard-coded path). Tests in this file that exercise codex set + restore it.
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());

    // @step When I dispatch init against that project root with agent list ['codex']
    let result = dispatch_command(req(tmp.path(), &["codex"]));

    // Restore HOME before asserting so a failure cannot leak into other tests.
    match prev_home {
        Some(v) => std::env::set_var("HOME", v),
        None => std::env::remove_var("HOME"),
    }

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then spec/AGENTS.md exists in the project root
    assert!(tmp.path().join("spec/AGENTS.md").exists());

    // @step Then the file .codex/prompts/fspec.md exists under the injected HOME directory
    assert!(
        home.path().join(".codex/prompts/fspec.md").exists(),
        "codex slash command must be written under the injected HOME"
    );

    // @step Then the filesInstalled array contains '~/.codex/prompts/fspec.md'
    let files = files_installed(&data);
    assert!(
        files.contains(&"~/.codex/prompts/fspec.md".to_string()),
        "filesInstalled must report the ~/.codex path; got: {files:?}"
    );
}

/// Doc bodies must be byte-for-byte identical to the live TypeScript
/// `node dist/index.js init --agent=<id>` output. The 19 agents collapse to 4
/// prose groups; one representative per group is checked against a captured
/// fixture. The fixtures keep the `<test-command>` / `<quality-check-commands>`
/// placeholders intact, so HOME is redirected to an empty sandbox to suppress
/// any developer `~/.fspec/fspec-config.json` substitution (parity with TS,
/// which would equally substitute from a present user config).
#[test]
fn generated_docs_match_typescript_byte_for_byte() {
    let cases: &[(&str, &str, &str)] = &[
        ("claude", "spec/CLAUDE.md", include_str!("fixtures/init_docs/claude.md")),
        ("cursor", "spec/CURSOR.md", include_str!("fixtures/init_docs/cursor.md")),
        ("cline", "spec/CLINE.md", include_str!("fixtures/init_docs/cline.md")),
        ("aider", "spec/AIDER.md", include_str!("fixtures/init_docs/aider.md")),
        ("gemini", "spec/GEMINI.md", include_str!("fixtures/init_docs/gemini.md")),
    ];

    // Redirect HOME at an empty sandbox so no real ~/.fspec/fspec-config.json
    // leaks tool-command substitutions into the byte comparison.
    let home_sandbox = TempDir::new().expect("home sandbox");
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home_sandbox.path());

    let mut failures: Vec<String> = Vec::new();
    for (agent, rel, expected) in cases {
        let tmp = TempDir::new().expect("tempdir");
        let result = dispatch_command(req(tmp.path(), &[agent]));
        if !result.success {
            failures.push(format!("{agent}: expected success; got {result:?}"));
            continue;
        }
        let doc = read_to_string(tmp.path(), rel);
        if doc != *expected {
            failures.push(format!(
                "{agent}: generated doc not byte-identical to TS fixture (gen={} exp={} bytes)",
                doc.len(),
                expected.len()
            ));
        }
    }

    // Restore HOME before asserting so a failure cannot leak into other tests.
    match prev_home {
        Some(v) => std::env::set_var("HOME", v),
        None => std::env::remove_var("HOME"),
    }

    assert!(failures.is_empty(), "doc parity failures:\n{}", failures.join("\n"));
}
