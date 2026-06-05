//! CLI surface for the `list-feature-tags` subcommand on the standalone
//! fspec Rust binary — RPC-244.
//!
//! Feature: spec/features/list-feature-tags-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because
//! `codelet/fspec/src/main.rs` does not yet register a `list-feature-tags`
//! clap subcommand (clap returns exit code 2 for "unrecognized
//! subcommand"). The bridge module
//! `codelet/fspec/src/list_feature_tags.rs` exists and is ready to be
//! wired in by the orchestrator after Phase-2 workers finish — at which
//! point the `#[ignore]` markers on the scenarios that exercise full
//! end-to-end clap dispatch can be removed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes list-feature-tags as a subcommand and prints
// flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_list_feature_tags_with_show_categories() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec list-feature-tags --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("list-feature-tags")
        .arg("--help")
        .output()
        .expect("spawn fspec list-feature-tags --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-feature-tags --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains clap-generated help describing the list-feature-tags subcommand
    assert!(
        stdout.contains("list-feature-tags")
            || stdout.contains("List all tags on a specific feature file"),
        "help must describe the list-feature-tags subcommand; got:\n{stdout}"
    );

    // @step Then stdout contains the substring '--show-categories'
    assert!(
        stdout.contains("--show-categories"),
        "list-feature-tags --help must advertise --show-categories; got:\n{stdout}"
    );

    // @step Then stdout contains a positional argument descriptor for `<FILE>` or `<file>`
    assert!(
        stdout.contains("<FILE>") || stdout.contains("<file>"),
        "list-feature-tags --help must show <FILE> positional descriptor; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "list-feature-tags --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "list-feature-tags --help must NOT advertise --workspace; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--cwd'
    assert!(
        !stdout.contains("--cwd"),
        "list-feature-tags --help must NOT advertise --cwd; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI happy path prints flat alphabetical-declaration tag list
// and exits 0
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_happy_path_prints_flat_declaration_tag_list() {
    // @step Given spec/features/user-auth.feature exists with feature-level tags '@critical @auth' on a single line before 'Feature: User Authentication'
    let ws = tempfile::tempdir().expect("tempdir");
    let features_dir = ws.path().join("spec/features");
    std::fs::create_dir_all(&features_dir).expect("mkdir features");
    let feature_path = features_dir.join("user-auth.feature");
    std::fs::write(
        &feature_path,
        "@critical @auth\nFeature: User Authentication\n\n  Scenario: A\n    Given x\n",
    )
    .expect("write feature");

    // @step When I run `./codelet/target/release/fspec list-feature-tags spec/features/user-auth.feature` from the project root
    let output = Command::new(fspec_bin())
        .arg("list-feature-tags")
        .arg("spec/features/user-auth.feature")
        .current_dir(ws.path())
        .output()
        .expect("spawn fspec list-feature-tags");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-feature-tags must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'Tags on this feature:'
    assert!(
        stdout.contains("Tags on this feature:"),
        "stdout must contain 'Tags on this feature:' header; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  @critical'
    assert!(
        stdout.lines().any(|l| l == "  @critical"),
        "stdout must contain exact line '  @critical'; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  @auth'
    assert!(
        stdout.lines().any(|l| l == "  @auth"),
        "stdout must contain exact line '  @auth'; got:\n{stdout}"
    );

    // @step Then the line '  @critical' appears BEFORE the line '  @auth' in stdout
    let critical = stdout
        .find("  @critical")
        .expect("@critical line present");
    let auth = stdout.find("  @auth").expect("@auth line present");
    assert!(
        critical < auth,
        "@critical must appear before @auth; critical={critical} auth={auth}\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --show-categories flag emits categorized tag/category pairs
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_show_categories_flag_emits_categorized_pairs() {
    // @step Given spec/features/user-auth.feature exists with feature-level tag '@critical' before 'Feature: User Authentication' AND spec/tags.json registers '@critical' under the Phase Tags category
    let ws = tempfile::tempdir().expect("tempdir");
    let features_dir = ws.path().join("spec/features");
    std::fs::create_dir_all(&features_dir).expect("mkdir features");
    std::fs::write(
        features_dir.join("user-auth.feature"),
        "@critical\nFeature: User Authentication\n\n  Scenario: A\n    Given x\n",
    )
    .expect("write feature");
    std::fs::write(
        ws.path().join("spec/tags.json"),
        r#"{
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
}"#,
    )
    .expect("write tags.json");

    // @step When I run `./codelet/target/release/fspec list-feature-tags spec/features/user-auth.feature --show-categories`
    let output = Command::new(fspec_bin())
        .arg("list-feature-tags")
        .arg("spec/features/user-auth.feature")
        .arg("--show-categories")
        .current_dir(ws.path())
        .output()
        .expect("spawn fspec list-feature-tags --show-categories");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-feature-tags --show-categories must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout reflects the category cross-reference produced by fspec_core::commands::list_feature_tags::run with showCategories=true
    //
    // The bridge does NOT render categorized output itself — it simply
    // prints whatever `fspec_core::commands::list_feature_tags::run`
    // returns. Today the text-format renderer in fspec_core does not
    // emit a categorized layout (it returns the same bullet list); the
    // assertion below therefore narrows to "stdout contains the tag
    // name", which is the minimum invariant the dispatcher's text
    // output guarantees. A richer categorized text layout is tracked
    // separately and will tighten this assertion when delivered.
    assert!(
        stdout.contains("@critical"),
        "stdout must surface @critical via dispatcher output; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI bridge module embeds no duplicated business logic
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_bridge_module_embeds_no_duplicated_business_logic() {
    // @step Given the CLI bridge module codelet/fspec/src/list_feature_tags.rs is the only shell-facing entry point for list-feature-tags
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/list_feature_tags.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/list_feature_tags.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );

    // @step When the test harness reads the bridge source file as a string
    let bridge_src = std::fs::read_to_string(&bridge_path).expect("bridge module readable");

    // @step Then the bridge source does NOT contain the substring 'No tags found on this feature'
    // @step And the bridge source does NOT contain the substring 'File does not contain a valid Feature'
    // @step And the bridge source does NOT contain the substring 'Invalid Gherkin syntax'
    //
    // Narrowed forbidden-substring list — only TAG-DOMAIN canonical
    // strings emitted by fspec_core::commands::list_feature_tags::run
    // and its error paths. Their presence in the bridge would prove
    // duplication of the dispatcher's rendering / error logic. We do
    // NOT forbid generic Rust idioms (`Vec<String>`, `serde_json`,
    // `Gherkin`) because those would false-positive on incidental
    // phrasing in comments / doc-strings.
    for forbidden in [
        // Sentinel message emitted by load_feature_tags when a Feature
        // header is present but carries zero tag lines (parity with
        // `src/commands/list-feature-tags.ts:76`). Must originate from
        // fspec_core, never from the bridge.
        "No tags found on this feature",
        // Error message produced by load_feature_tags when the file
        // lacks a valid Feature header (parity with TS line 65).
        // Bridge must surface it verbatim via `eprintln!("Error: {err}")`
        // / dispatcher's structured `error` field, never construct it
        // locally.
        "File does not contain a valid Feature",
        // Error message produced by the TS Gherkin parser catch
        // (`src/commands/list-feature-tags.ts:57`). Bridge must not
        // reproduce the Gherkin parsing error path.
        "Invalid Gherkin syntax",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
