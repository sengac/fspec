//! Feature: spec/features/fspec-binary-cargo-shape-rpc010.feature
use super::*;

#[test]
fn scenario_codelet_fspec_is_registered_as_a_workspace_member() {
    // @step Given the file `rust/Cargo.toml` exists
    let cargo = codelet_root().join("Cargo.toml");
    assert!(cargo.is_file(), "rust/Cargo.toml must exist");
    let body = std::fs::read_to_string(&cargo).expect("read rust/Cargo.toml");

    // @step When the test parses `[workspace].members`
    let members_section =
        extract_section(&body, "[workspace]").expect("[workspace] section must exist");
    let members_list = extract_members(&members_section).expect("[workspace].members must exist");

    // @step Then the members list contains the string `fspec`
    assert!(
        members_list.iter().any(|s| s == "fspec"),
        "members must contain `fspec`; got {members_list:?}"
    );

    // @step And `fspec` appears between `core` and `fspec-core` in the members list (preserving alphabetical order: agent-loop, cli, common, core, fspec, fspec-core, fspec-tui, git, graph, napi, providers, rpc, rpc-embedded, rpc-server, rpc-types, sessions, test-helpers, tools, tui)
    let expected = [
        // RPC-072: NAPI-free FspecAgentHooks + agent_loop. Sorts ahead
        // of `cli` alphabetically.
        "agent-loop",
        // Attachment viewer HTTP crate (axum); sorts between `agent-loop` and `cli`.
        "attachment-viewer",
        "cli",
        "common",
        "core",
        "fspec",
        // TOOL-019 / RPC-003: pure-Rust home for the future port of the
        // TypeScript fspec CLI commands.
        "fspec-core",
        // RPC-334: vendored+trimmed serde_json caret-diagnostic formatter.
        // Sorts between `fspec-core` and `fspec-tui`.
        "fspec-json-error",
        "fspec-tui",
        "git",
        // Knowledge-graph crate; sorts between `git` and `providers`.
        "graph",
        "providers",
        "rpc",
        "rpc-embedded",
        "rpc-server",
        "rpc-types",
        // RPC-038/044: NAPI-free home for SessionManager + BackgroundSession.
        "sessions",
        // RPC-067: shared no-codelet-napi dependency-rule helpers.
        "test-helpers",
        "tools",
        "tui",
    ];
    assert_eq!(
        members_list, expected,
        "members must be exactly the locked alphabetical sequence"
    );
}

#[test]
fn scenario_cargo_toml_declares_a_single_bin_named_fspec() {
    // @step Given the file `rust/fspec/Cargo.toml` exists
    let cargo = fspec_crate_root().join("Cargo.toml");
    assert!(cargo.is_file(), "rust/fspec/Cargo.toml must exist");
    let body = std::fs::read_to_string(&cargo).expect("read fspec/Cargo.toml");

    // @step Then it contains a `[[bin]]` table with `name = "fspec"`
    assert!(
        body.contains("[[bin]]"),
        "Cargo.toml must declare a [[bin]] table"
    );
    let bin_section = extract_section(&body, "[[bin]]").expect("[[bin]] section");
    assert!(
        bin_section.contains(r#"name = "fspec""#),
        "[[bin]] must set name = \"fspec\"; got:\n{bin_section}"
    );

    // @step And the same `[[bin]]` table sets `path = "src/main.rs"`
    assert!(
        bin_section.contains(r#"path = "src/main.rs""#),
        "[[bin]] must set path = \"src/main.rs\"; got:\n{bin_section}"
    );

    // @step And no other `[[bin]]` table is declared
    let bin_header_lines = body.lines().filter(|l| l.trim() == "[[bin]]").count();
    assert_eq!(
        bin_header_lines, 1,
        "exactly one [[bin]] section header is permitted in rust/fspec/Cargo.toml"
    );
}
