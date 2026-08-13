#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/report-bug-to-github-rust-port.feature
//
// Dispatcher-contract tests for the Rust port of `report-bug-to-github`
// (RPC-285, DETERMINISTIC-CORE scope). Each scenario maps to exactly one
// #[test] with @step comments mirroring the Gherkin steps verbatim.
//
// RED PHASE: the command is still a stub returning NotYetPorted, so these
// tests FAIL now. They assert the real expected behaviour the Phase C
// implementation must satisfy.
//
// SCOPE (supervisor ruling, RPC-285): gather environment + git context (git
// via BLOCKING std::process::Command::output(), NO in-command network), format
// the issue body, build the GitHub issue URL, and return it. The browser launch
// and interactive stdin prompts are DEFERRED (dispatcher-only, not
// implemented) — every path returns the url with browserOpened=false.
//
// ENVIRONMENT ASSUMPTION: these tests assume a clean environment (no relevant
// env vars). The fspec version is a pinned const ("0.9.3", parity with
// init.rs FSPEC_VERSION) and the OS comes from std::env::consts::OS, so the
// Environment-line assertions are behaviour-level (present + well-formed),
// NOT a byte-exact reproduction of the Node `process.version` string.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "report-bug-to-github".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

/// Parse the dispatcher data envelope.
fn envelope(result_data: &str) -> Value {
    serde_json::from_str(result_data).expect("parse data json")
}

fn markdown_of(data: &Value) -> String {
    data["markdown"].as_str().unwrap_or_default().to_string()
}

fn url_of(data: &Value) -> String {
    data["url"].as_str().unwrap_or_default().to_string()
}

/// Write a `spec/work-units.json` containing a single non-done work unit.
fn write_work_unit(project_root: &Path, id: &str, title: &str, status: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let data = json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-17T00:00:00.000Z" },
        "workUnits": {
            id: {
                "id": id,
                "type": "bug",
                "title": title,
                "status": status,
                "createdAt": "2026-06-17T00:00:00.000Z",
                "updatedAt": "2026-06-17T10:00:00.000Z"
            }
        },
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": [id], "validating": [], "done": [], "blocked": []
        }
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&data).expect("ser work-units"),
    )
    .expect("write work-units.json");
}

fn write_feature_tagged(project_root: &Path, file_name: &str, tag: &str) {
    let features = project_root.join("spec").join("features");
    fs::create_dir_all(&features).expect("mkdir features");
    let content = format!(
        "{tag}\nFeature: User login\n\n  Scenario: placeholder\n    Given a step\n    Then a result\n"
    );
    fs::write(features.join(file_name), content).expect("write feature");
}

// ---------- scenarios ----------

#[test]
fn default_flow_produces_complete_report_with_all_sections() {
    // @step Given an empty project root tempdir with no git repo and no work units
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch report-bug-to-github with bug-description "crash on save"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "bugDescription": "crash on save" }),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");
    let data = envelope(&result.data);
    let md = markdown_of(&data);

    // @step And the report markdown contains the section "## Description"
    assert!(md.contains("## Description"), "md={md}");
    // @step And the report markdown contains the section "## Expected Behavior"
    assert!(md.contains("## Expected Behavior"), "md={md}");
    // @step And the report markdown contains the section "## Actual Behavior"
    assert!(md.contains("## Actual Behavior"), "md={md}");
    // @step And the report markdown contains the section "## Steps to Reproduce"
    assert!(md.contains("## Steps to Reproduce"), "md={md}");
    // @step And the report markdown contains the section "## Environment"
    assert!(md.contains("## Environment"), "md={md}");
    // @step And the report markdown contains the section "## Additional Context"
    assert!(md.contains("## Additional Context"), "md={md}");

    // @step And the report title starts with "Bug: crash on save"
    let title = data["title"].as_str().unwrap_or_default();
    assert!(title.starts_with("Bug: crash on save"), "title={title}");
}

#[test]
fn environment_section_reports_pinned_version_and_os() {
    // @step Given an empty project root tempdir with no git repo and no work units
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch report-bug-to-github with bug-description "crash on save"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "bugDescription": "crash on save" }),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");
    let md = markdown_of(&envelope(&result.data));

    // @step And the report markdown contains "fspec version: 0.9.3"
    assert!(md.contains("fspec version: 0.9.3"), "md={md}");

    // @step And the report markdown contains the current OS platform line
    assert!(
        md.contains(std::env::consts::OS),
        "md must mention OS '{}': {md}",
        std::env::consts::OS
    );
}

#[test]
fn constructed_url_targets_sengac_fspec_with_encoded_labels() {
    // @step Given an empty project root tempdir with no git repo and no work units
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch report-bug-to-github with bug-description "crash on save"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "bugDescription": "crash on save" }),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");
    let url = url_of(&envelope(&result.data));

    // @step And the result url starts with "https://github.com/sengac/fspec/issues/new?title="
    assert!(
        url.starts_with("https://github.com/sengac/fspec/issues/new?title="),
        "url={url}"
    );

    // @step And the result url contains "labels=bug%2Cneeds-triage"
    assert!(url.contains("labels=bug%2Cneeds-triage"), "url={url}");
}

#[test]
fn url_encoding_escapes_spaces_and_special_characters() {
    // @step Given an empty project root tempdir with no git repo and no work units
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch report-bug-to-github with bug-description "fix #42 now"
    let result = dispatch_command(req(tmp.path(), json!({ "bugDescription": "fix #42 now" })));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");
    let url = url_of(&envelope(&result.data));

    // @step And the result url contains "%23" for the hash character
    assert!(url.contains("%23"), "url must encode '#' as %23: {url}");

    // @step And the result url contains no raw space characters
    assert!(!url.contains(' '), "url must not contain raw spaces: {url}");
}

#[test]
fn work_unit_context_included_when_non_done_work_unit_exists() {
    // @step Given a project root tempdir whose work-units.json has an in-progress work unit "AUTH-001" titled "Login"
    let tmp = TempDir::new().expect("tempdir");
    write_work_unit(tmp.path(), "AUTH-001", "Login", "implementing");

    // @step And a feature file spec/features/user-login.feature tagged "@AUTH-001"
    write_feature_tagged(tmp.path(), "user-login.feature", "@AUTH-001");

    // @step When I dispatch report-bug-to-github with bug-description "login broken"
    let result = dispatch_command(req(tmp.path(), json!({ "bugDescription": "login broken" })));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");
    let md = markdown_of(&envelope(&result.data));

    // @step And the report markdown contains "AUTH-001"
    assert!(md.contains("AUTH-001"), "md={md}");

    // @step And the report markdown contains "**Feature File**: spec/features/user-login.feature"
    assert!(
        md.contains("**Feature File**: spec/features/user-login.feature"),
        "md={md}"
    );
}

#[test]
fn gathering_context_faithfully_replicates_work_units_side_effect() {
    // @step Given an empty project root tempdir with no spec subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(
        !tmp.path().join("spec").exists(),
        "precondition: no spec dir"
    );

    // @step When I dispatch report-bug-to-github with bug-description "crash on save"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "bugDescription": "crash on save" }),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");
    let data = envelope(&result.data);

    // @step And spec/work-units.json is created in the project root with the canonical initial structure
    let wu_path = tmp.path().join("spec/work-units.json");
    assert!(
        wu_path.exists(),
        "work-units.json must be created (faithful TS side-effect)"
    );
    let on_disk: Value =
        serde_json::from_str(&fs::read_to_string(&wu_path).expect("read work-units"))
            .expect("parse");
    assert_eq!(on_disk["version"], json!("0.7.1"), "canonical version");
    assert!(
        on_disk["states"]["backlog"].is_array(),
        "canonical states present"
    );

    // @step And the dispatcher does not return an error
    assert!(result.error.is_none(), "no error expected: {result:?}");

    // @step And the result reports browserOpened as false
    assert_eq!(data["browserOpened"], json!(false), "browser must not open");
}

#[test]
fn both_front_doors_converge_on_same_function() {
    // @step Given an empty project root tempdir with no git repo and no work units
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch report-bug-to-github with bug-description "crash on save" via the dispatcher
    // (the binary front-door is exercised in cli_report_bug_to_github.rs; both call
    //  commands::report_bug_to_github::run)
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "bugDescription": "crash on save" }),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the result url starts with "https://github.com/sengac/fspec/issues/new?title="
    let url = url_of(&envelope(&result.data));
    assert!(
        url.starts_with("https://github.com/sengac/fspec/issues/new?title="),
        "url={url}"
    );
}
