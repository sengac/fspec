//! CLI surface for the `list-tags` subcommand on the standalone fspec
//! Rust binary — RPC-251.
//!
//! Feature: spec/features/list-tags-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because:
//!   - `codelet/fspec/src/main.rs` does not yet register a `list-tags`
//!     clap subcommand (clap returns exit code 2 for "unrecognized
//!     subcommand").
//!   - `codelet/fspec-core/src/commands/list_tags.rs` is still a
//!     NotYetPorted stub.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_list_tags(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("list-tags");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec list-tags");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_tags_file(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), raw).expect("write tags file");
}

fn two_categories_json() -> String {
    r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "x",
      "required": true,
      "tags": [
        { "name": "@critical", "description": "Critical features" }
      ]
    },
    {
      "name": "Component Tags",
      "description": "x",
      "required": true,
      "tags": [
        { "name": "@cli", "description": "CLI surface" }
      ]
    }
  ]
}"#
    .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes list-tags as a subcommand and prints flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_list_tags_with_flag_aware_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec list-tags --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("list-tags")
        .arg("--help")
        .output()
        .expect("spawn fspec list-tags --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-tags --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains clap-generated help describing the list-tags subcommand
    assert!(
        stdout.contains("list-tags") || stdout.contains("List all registered tags"),
        "help must describe the list-tags subcommand; got:\n{stdout}"
    );

    // @step And stdout contains the substring '--category'
    assert!(
        stdout.contains("--category"),
        "list-tags --help must advertise --category; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "list-tags --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "list-tags --help must NOT advertise --workspace; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '--cwd'
    assert!(
        !stdout.contains("--cwd"),
        "list-tags --help must NOT advertise --cwd; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty directory auto-creates tags.json and prints all 9 default categories
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_empty_directory_auto_creates_tags_file() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec list-tags` from that directory
    let (code, stdout, stderr) = run_list_tags(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-tags must exit 0 on empty workspace; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring 'Phase Tags (0 tags)'
    assert!(
        stdout.contains("Phase Tags (0 tags)"),
        "stdout must contain 'Phase Tags (0 tags)' header; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Component Tags (0 tags)'
    assert!(
        stdout.contains("Component Tags (0 tags)"),
        "stdout must contain 'Component Tags (0 tags)' header; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Automation Tags (0 tags)'
    assert!(
        stdout.contains("Automation Tags (0 tags)"),
        "stdout must contain 'Automation Tags (0 tags)' header; got:\n{stdout}"
    );

    // @step And stdout contains the substring '  No tags registered'
    assert!(
        stdout.contains("  No tags registered"),
        "stdout must contain 'No tags registered' line; got:\n{stdout}"
    );

    // @step And spec/tags.json was created in the directory
    assert!(
        ws.path().join("spec").join("tags.json").exists(),
        "list-tags must auto-create spec tags file"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders alphabetically sorted tags per category
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_alphabetically_sorted_tags() {
    // @step Given spec/tags.json contains a Phase Tags category with tags '@zed' (description 'Z desc') and '@aaa' (description 'A desc') in that insertion order
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "x",
      "required": true,
      "tags": [
        { "name": "@zed", "description": "Z desc" },
        { "name": "@aaa", "description": "A desc" }
      ]
    }
  ]
}"#;
    write_tags_file(ws.path(), raw);

    // @step When I run `./codelet/target/release/fspec list-tags`
    let (code, stdout, stderr) = run_list_tags(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-tags must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'Phase Tags (2 tags)'
    assert!(
        stdout.contains("Phase Tags (2 tags)"),
        "stdout must contain 'Phase Tags (2 tags)' header; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  @aaa - A desc'
    assert!(
        stdout.lines().any(|l| l == "  @aaa - A desc"),
        "stdout must contain exact line '  @aaa - A desc'; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  @zed - Z desc'
    assert!(
        stdout.lines().any(|l| l == "  @zed - Z desc"),
        "stdout must contain exact line '  @zed - Z desc'; got:\n{stdout}"
    );

    // @step Then the line '  @aaa - A desc' appears BEFORE the line '  @zed - Z desc' in stdout
    let aaa = stdout
        .find("  @aaa - A desc")
        .expect("@aaa line present");
    let zed = stdout
        .find("  @zed - Z desc")
        .expect("@zed line present");
    assert!(
        aaa < zed,
        "@aaa must appear before @zed; aaa={aaa} zed={zed}\n{stdout}"
    );
    // @step And stdout contains the exact line ' @aaa - A desc'
    // @step And stdout contains the exact line ' @zed - Z desc'
    // @step And the line ' @aaa - A desc' appears BEFORE the line ' @zed - Z desc' in stdout
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --category filter restricts output to the matching category
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_category_filter_restricts_output() {
    // @step Given spec/tags.json contains Phase Tags (with '@critical') and Component Tags (with '@cli') categories
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags_file(ws.path(), &two_categories_json());

    // @step When I run `./codelet/target/release/fspec list-tags --category 'Phase Tags'`
    let (code, stdout, stderr) = run_list_tags(ws.path(), &["--category", "Phase Tags"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-tags --category 'Phase Tags' must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring 'Phase Tags'
    assert!(
        stdout.contains("Phase Tags"),
        "stdout must contain 'Phase Tags'; got:\n{stdout}"
    );

    // @step And stdout contains the substring '@critical'
    assert!(
        stdout.contains("@critical"),
        "stdout must contain '@critical'; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring 'Component Tags'
    assert!(
        !stdout.contains("Component Tags"),
        "filter must drop Component Tags; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '@cli'
    assert!(
        !stdout.contains("@cli"),
        "filter must drop @cli; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --category filter exits 1 and writes 'Category not found' to stderr for unknown category
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_category_filter_unknown_exits_1() {
    // @step Given spec/tags.json contains Phase Tags and Component Tags categories
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags_file(ws.path(), &two_categories_json());

    // @step When I run `./codelet/target/release/fspec list-tags --category 'No Such Category'`
    let (code, stdout, stderr) =
        run_list_tags(ws.path(), &["--category", "No Such Category"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec list-tags must exit 1 on unknown category; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Category not found: No Such Category. Available categories:'
    assert!(
        stderr.contains("Category not found: No Such Category. Available categories:"),
        "stderr must contain canonical error substring; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 and writes to stderr when tags.json is malformed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_malformed_tags_file_exits_1() {
    // @step Given spec/tags.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags_file(ws.path(), "{ this is not valid json");

    // @step When I run `./codelet/target/release/fspec list-tags`
    let (code, stdout, stderr) = run_list_tags(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec list-tags must exit 1 on malformed input; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Failed to parse tags.json'
    assert!(
        stderr.contains("Failed to parse tags.json"),
        "stderr must contain 'Failed to parse tags.json'; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_list_tags() {
    // @step Given the fspec Rust binary has list-tags registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    // (asserted by the help-listing check below)

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code, 0,
        "fspec --help must exit 0; got {code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists daemon, client, status, list-work-units, list-prefixes, and list-tags as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
        "list-tags",
    ] {
        assert!(
            help.contains(sub),
            "fspec --help must list `{sub}` subcommand; got:\n{help}"
        );
    }

    // @step Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document combined-mode default; got:\n{help}"
    );
    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/tags.json contains a Phase Tags category with '@critical' (description 'Critical features')
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "x",
      "required": true,
      "tags": [
        { "name": "@critical", "description": "Critical features" }
      ]
    }
  ]
}"#;
    write_tags_file(ws.path(), raw);

    // @step When I dispatch list-tags through fspec_core::dispatch::dispatch_command with format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "list-tags".to_string(),
        args_json: r#"{"format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher's DispatchResult.data parses to a structure whose Phase Tags entry contains '@critical' with description 'Critical features'
    let cats = dispatcher_data["categories"]
        .as_array()
        .expect("categories array");
    let phase_tags = cats
        .iter()
        .find(|c| c["name"].as_str() == Some("Phase Tags"))
        .expect("Phase Tags entry present");
    let tags = phase_tags["tags"].as_array().expect("tags array");
    let critical = tags
        .iter()
        .find(|t| t["tag"].as_str() == Some("@critical"))
        .expect("@critical entry present");
    assert_eq!(
        critical["description"].as_str(),
        Some("Critical features")
    );

    // @step Then the CLI bridge module codelet/fspec/src/list_tags.rs contains NO inline category-filter, tag-sorting, or rendering logic
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/list_tags.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/list_tags.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");

    // @step Then the bridge module's only computation is JSON arg marshalling and CWD resolution
    //
    // Narrowed forbidden-substring list — only TAG-DOMAIN strings that
    // would betray duplicated business logic. We deliberately do NOT
    // forbid generic Rust idioms (`tags)\n`, `.sort_by`, `localeCompare`)
    // because those would false-positive on incidental phrasing in
    // comments / doc-strings. The three substrings below are the
    // user-visible canonical strings emitted by `list_tags.rs::render_text`
    // and its error paths — their presence in the bridge would
    // prove duplication of the dispatcher's rendering / error logic.
    for forbidden in [
        // Canonical "empty category" sentinel emitted by render_text
        // when a category has zero tags (parity with TS
        // `src/commands/list-tags.ts:78`). Must originate from
        // fspec_core, never from the bridge.
        "No tags registered",
        // Header substring emitted by render_text on the
        // category-filter happy-path branch
        // (`src/commands/list-tags.ts:60-67`). Bridge must not
        // re-render headers.
        "Available categories",
        // Error message produced by fspec_core on the
        // `--category` miss branch
        // (`src/commands/list-tags.ts:48-54`). Bridge must surface
        // it verbatim via `eprintln!("Error: {err}")`, never
        // construct it locally.
        "Category not found",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
    // @step And the CLI bridge module codelet/fspec/src/list_tags.rs contains NO inline category-filter, tag-sorting, or rendering logic
    // @step And the bridge module's only computation is JSON arg marshalling and CWD resolution
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list-tags --help is byte-for-byte identical to TS (RPC-251)
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_LT: &str = include_str!("fixtures/help/list-tags.txt");

#[test]
fn scenario_list_tags_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec list-tags --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("list-tags")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn list-tags --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "list-tags --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/list-tags.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_LT);

    // @step And stdout starts with a blank line followed by 'LIST-TAGS'
    assert!(stdout.starts_with("\nLIST-TAGS\n"));

    // @step And stdout contains the section header 'OPTIONS' followed by '  --category <category>'
    assert!(stdout.contains("OPTIONS\n  --category <category>\n"));
}
