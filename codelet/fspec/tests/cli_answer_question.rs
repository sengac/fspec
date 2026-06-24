//! CLI surface for the `answer-question` subcommand on the standalone fspec
//! Rust binary — RPC-196.
//!
//! Feature: spec/features/answer-question-cli-subcommand.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

fn run_aq(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("answer-question");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec answer-question");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn seed_unit(id: &str, status: &str) -> serde_json::Value {
    let mut states_obj = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr: Vec<serde_json::Value> = if *st == status {
            vec![serde_json::Value::String(id.to_string())]
        } else {
            vec![]
        };
        states_obj.insert((*st).to_string(), serde_json::Value::Array(arr));
    }
    serde_json::json!({
        "version": "0.7.1",
        "workUnits": {
            id: {
                "id": id,
                "title": "title",
                "type": "story",
                "status": status,
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": states_obj
    })
}

fn pretty(v: &serde_json::Value) -> String {
    serde_json::to_string_pretty(v).unwrap()
}

fn q(id: u64, text: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "text": text,
        "deleted": false,
        "createdAt": "2026-06-01T00:00:00.000Z"
    })
}

const TS_HELP_FIXTURE_AQ: &str = include_str!("fixtures/help/answer-question.txt");

#[test]
fn scenario_answer_question_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec answer-question --help`
    let output = Command::new(fspec_bin())
        .arg("answer-question")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn answer-question --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "answer-question --help must exit 0; stderr={stderr}"
    );

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/answer-question.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_AQ);

    // @step And stdout starts with a blank line followed by 'ANSWER-QUESTION'
    assert!(
        stdout.starts_with("\nANSWER-QUESTION\n"),
        "got start: {:?}",
        &stdout[..stdout.len().min(40)]
    );
}

#[test]
fn scenario_cli_successfully_answers_question_with_add_to_rule_and_prints_success_lines() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Should we support OAuth?',deleted:false,createdAt:'x'}] and nextRuleId=0
    let ws = tempfile::tempdir().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] =
        serde_json::json!([q(0, "Should we support OAuth?")]);
    pre["workUnits"]["AUTH-001"]["nextRuleId"] = serde_json::json!(0);
    write_work_units(ws.path(), &pretty(&pre));

    // @step When I run `fspec answer-question AUTH-001 0 --answer "Yes, Google OAuth" --add-to rule` in that tempdir
    let (code, stdout, stderr) = run_aq(
        ws.path(),
        &[
            "AUTH-001",
            "0",
            "--answer",
            "Yes, Google OAuth",
            "--add-to",
            "rule",
        ],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Answered question: "Should we support OAuth?"'
    assert!(
        stdout.contains(r#"✓ Answered question: "Should we support OAuth?""#),
        "stdout must contain success line; got:\n{stdout}"
    );
    // @step And stdout contains the substring 'Answer: "Yes, Google OAuth"'
    assert!(
        stdout.contains(r#"Answer: "Yes, Google OAuth""#),
        "got: {stdout}"
    );
    // @step And stdout contains the substring 'Added to rules: "Yes, Google OAuth"'
    assert!(
        stdout.contains(r#"Added to rules: "Yes, Google OAuth""#),
        "got: {stdout}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].text='Yes, Google OAuth'
    let v = read_work_units(ws.path());
    assert_eq!(
        v["workUnits"]["AUTH-001"]["rules"][0]["text"].as_str(),
        Some("Yes, Google OAuth")
    );
    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].id=0
    assert_eq!(
        v["workUnits"]["AUTH-001"]["rules"][0]["id"].as_u64(),
        Some(0)
    );
}

#[test]
fn scenario_cli_defaults_add_to_to_none_no_rule_or_assumption_added() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    let ws = tempfile::tempdir().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] = serde_json::json!([q(0, "Q?")]);
    write_work_units(ws.path(), &pretty(&pre));

    // @step When I run `fspec answer-question AUTH-001 0 --answer "Maybe"` in that tempdir
    let (code, stdout, stderr) = run_aq(ws.path(), &["AUTH-001", "0", "--answer", "Maybe"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Answered question: "Q?"'
    assert!(
        stdout.contains(r#"✓ Answered question: "Q?""#),
        "got: {stdout}"
    );
    // @step And stdout contains the substring 'Answer: "Maybe"'
    assert!(stdout.contains(r#"Answer: "Maybe""#), "got: {stdout}");
    // @step And stdout does NOT contain the substring 'Added to'
    assert!(!stdout.contains("Added to"), "got: {stdout}");

    let v = read_work_units(ws.path());
    // @step And spec/work-units.json on disk shows AUTH-001 has no rules added
    assert!(v["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .is_none_or(Vec::is_empty));
    // @step And spec/work-units.json on disk shows AUTH-001 has no assumptions added
    assert!(v["workUnits"]["AUTH-001"]["assumptions"]
        .as_array()
        .is_none_or(Vec::is_empty));
}

#[test]
fn scenario_cli_rejects_non_specifying_status_with_exit_1() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    let ws = tempfile::tempdir().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "backlog");
    pre["workUnits"]["AUTH-001"]["questions"] = serde_json::json!([q(0, "Q?")]);
    write_work_units(ws.path(), &pretty(&pre));
    let pre_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();

    // @step When I run `fspec answer-question AUTH-001 0 --answer "Yes" --add-to rule` in that tempdir
    let (code, _stdout, stderr) = run_aq(
        ws.path(),
        &["AUTH-001", "0", "--answer", "Yes", "--add-to", "rule"],
    );

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");
    // @step And stderr contains the substring '✗ Failed to answer question:'
    assert!(
        stderr.contains("✗ Failed to answer question:"),
        "got: {stderr}"
    );
    // @step And stderr contains the substring "Can only answer questions during discovery/specification phase. AUTH-001 is in 'backlog' state."
    assert!(
        stderr.contains("Can only answer questions during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "got: {stderr}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn scenario_cli_rejects_out_of_range_index_with_exit_1() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q1',deleted:false,createdAt:'x'}]
    let ws = tempfile::tempdir().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] = serde_json::json!([q(0, "Q1")]);
    write_work_units(ws.path(), &pretty(&pre));

    // @step When I run `fspec answer-question AUTH-001 99 --answer "X" --add-to rule` in that tempdir
    let (code, _stdout, stderr) = run_aq(
        ws.path(),
        &["AUTH-001", "99", "--answer", "X", "--add-to", "rule"],
    );

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");
    // @step And stderr contains the substring '✗ Failed to answer question:'
    assert!(
        stderr.contains("✗ Failed to answer question:"),
        "got: {stderr}"
    );
    // @step And stderr contains the substring 'Invalid question index 99'
    assert!(
        stderr.contains("Invalid question index 99"),
        "got: {stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    let ws = tempfile::tempdir().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] = serde_json::json!([q(0, "Q?")]);
    write_work_units(ws.path(), &pretty(&pre));

    // @step When I dispatch answer-question via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' index=0 answer='Yes' addTo='rule'
    let req = codelet_fspec_core::DispatchRequest {
        command: "answer-question".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","index":0,"answer":"Yes","addTo":"rule"}"#
            .to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec answer-question AUTH-001 0 --answer "Twice" --add-to rule` afterwards exits 0
    let (code, stdout, stderr) = run_aq(
        ws.path(),
        &["AUTH-001", "0", "--answer", "Twice", "--add-to", "rule"],
    );
    assert_eq!(
        code, 0,
        "follow-up CLI invocation must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.rules has length 2
    let v = read_work_units(ws.path());
    let rules = v["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .expect("rules array");
    assert_eq!(rules.len(), 2);

    // @step And the CLI bridge module codelet/fspec/src/answer_question.rs contains NO inline question lookup, status guard, RuleItem construction, or file-write logic — its only computation is clap parsing + JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/answer_question.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/answer_question.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    let stripped = common::strip_comments(&bridge_src);
    for forbidden in [
        "RuleItem",
        "nextRuleId",
        "ensure_work_units_file",
        "write_json_atomic",
        "iso8601_now",
        "Can only answer questions",
        "Invalid question index",
        "Question format is invalid",
        "questions[",
    ] {
        assert!(
            !stripped.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{stripped}"
        );
    }
}
