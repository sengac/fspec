//! CLI surface for the `show-event-storm` subcommand on the standalone
//! fspec Rust binary — RPC-303.
//!
//! Feature: spec/features/show-event-storm-cli-subcommand.feature
//! Feature: spec/features/show-event-storm-rust-port.feature
//!
//! Red phase: until the clap subcommand is wired (Phase C), these tests
//! exercise the binary and expect either a NotYetPorted error path or
//! a missing-subcommand failure. Once the subcommand is wired, the
//! green-phase assertions take over.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_ses(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("show-event-storm");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec show-event-storm");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn dispatch_ses(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    let req = codelet_fspec_core::DispatchRequest {
        command: "show-event-storm".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    };
    codelet_fspec_core::dispatch_command(req)
}

// =========================================================================
// Scenarios from show-event-storm-cli-subcommand.feature
// =========================================================================

#[test]
fn scenario_clap_exposes_show_event_storm_with_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // @step When I run `./rust/target/release/fspec show-event-storm --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("show-event-storm")
        .arg("--help")
        .output()
        .expect("spawn fspec show-event-storm --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-event-storm --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'show-event-storm'
    assert!(
        stdout.contains("show-event-storm") || stdout.contains("SHOW-EVENT-STORM"),
        "help must describe the subcommand; got:\n{stdout}"
    );

    // @step Then stdout advertises the required positional <work-unit-id> argument
    assert!(
        stdout.contains("work-unit-id")
            || stdout.contains("WORK-UNIT-ID")
            || stdout.contains("workUnitId"),
        "help must advertise <work-unit-id>; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "help must NOT advertise --format; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_empty_workspace_exits_1_with_not_found() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec show-event-storm AUTH-001` from that directory
    let (code, stdout, stderr) = run_ses(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Work unit AUTH-001 not found'
    assert!(
        stderr.contains("Work unit AUTH-001 not found"),
        "stderr must contain not-found message; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_exits_1_when_no_event_storm_data() {
    // @step Given spec/work-units.json contains AUTH-001 with no eventStorm field
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I run `./rust/target/release/fspec show-event-storm AUTH-001` from that directory
    let (code, stdout, stderr) = run_ses(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Work unit AUTH-001 has no Event Storm data'
    assert!(
        stderr.contains("Work unit AUTH-001 has no Event Storm data"),
        "stderr must contain 'no Event Storm data' message; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_prints_json_array_of_active_items_on_success() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0, deleted=false), event(id=1, deleted=true), command(id=2, deleted=false)]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm": {"items":[
        {"id":0,"type":"event","text":"x","deleted":false},
        {"id":1,"type":"event","text":"y","deleted":true},
        {"id":2,"type":"command","text":"z","deleted":false}
      ]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I run `./rust/target/release/fspec show-event-storm AUTH-001` from that directory
    let (code, stdout, stderr) = run_ses(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout parses as a JSON array of length 2
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must be valid JSON: {e}\nstdout:\n{stdout}"));
    assert_eq!(parsed.as_array().map(Vec::len), Some(2));

    // @step Then the parsed array[0] has id=0
    assert_eq!(parsed[0]["id"].as_u64(), Some(0));

    // @step Then the parsed array[1] has id=2
    assert_eq!(parsed[1]["id"].as_u64(), Some(2));
}

#[test]
fn scenario_cli_exits_1_when_malformed_work_units() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I run `./rust/target/release/fspec show-event-storm AUTH-001`
    let (code, stdout, stderr) = run_ses(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Failed to parse work-units.json'
    assert!(
        stderr.contains("Failed to parse work-units.json"),
        "stderr must contain parse error; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0, deleted=false)]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm": {"items":[{"id":0,"type":"event","text":"x","deleted":false}]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);
    assert!(result.success, "dispatcher must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as a JSON array of length 1
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed.as_array().map(Vec::len), Some(1));

    // @step Then the CLI bridge module rust/fspec/src/show_event_storm.rs contains NO inline filter or rendering logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/show_event_storm.rs");
    assert!(
        bridge_path.exists(),
        "bridge module must exist: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "eventStorm",
        "has no Event Storm data",
        "Work unit",
        "deleted",
        ".filter(",
        "active_items",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE_SES: &str = include_str!("fixtures/help/show-event-storm.txt");

#[test]
fn scenario_show_event_storm_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // @step When I run `./rust/target/release/fspec show-event-storm --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("show-event-storm")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-event-storm --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/show-event-storm.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SES);

    // @step Then stdout starts with a blank line followed by 'SHOW-EVENT-STORM'
    assert!(stdout.starts_with("\nSHOW-EVENT-STORM\n"));
}

// =========================================================================
// Scenarios from show-event-storm-rust-port.feature (dispatcher path)
// =========================================================================

#[test]
fn scenario_returns_work_unit_not_found_when_empty_workspace() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 not found'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Work unit AUTH-001 not found"),
        "error must contain canonical message; got {result:?}"
    );

    // @step Then spec/work-units.json exists after the call (auto-created by ensure_work_units_file)
    assert!(
        ws.path().join("spec/work-units.json").exists(),
        "spec/work-units.json must be auto-created"
    );
}

#[test]
fn scenario_returns_work_unit_not_found_when_id_missing() {
    // @step Given spec/work-units.json contains BUG-001 but not AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "BUG-001": {"id":"BUG-001","title":"B","status":"backlog","createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["BUG-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 not found'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Work unit AUTH-001 not found"),
        "error must contain canonical message; got {result:?}"
    );
}

#[test]
fn scenario_returns_no_event_storm_data_when_no_event_storm_field() {
    // @step Given spec/work-units.json contains AUTH-001 with no eventStorm field
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 has no Event Storm data'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Work unit AUTH-001 has no Event Storm data"),
        "error must contain canonical message; got {result:?}"
    );
}

#[test]
fn scenario_returns_no_event_storm_data_when_no_items_array() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm={} (no items field)
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","eventStorm":{}}
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 has no Event Storm data'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Work unit AUTH-001 has no Event Storm data"),
        "error must contain canonical message; got {result:?}"
    );
}

#[test]
fn scenario_returns_active_items_as_pretty_printed_json_array() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0, deleted=false), command(id=1, deleted=false)]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[
        {"id":0,"type":"event","text":"x","deleted":false},
        {"id":1,"type":"command","text":"y","deleted":false}
      ]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as a JSON array of length 2
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed.as_array().map(Vec::len), Some(2));

    // @step Then the parsed array[0] has id=0 and type='event'
    assert_eq!(parsed[0]["id"].as_u64(), Some(0));
    assert_eq!(parsed[0]["type"].as_str(), Some("event"));

    // @step Then the parsed array[1] has id=1 and type='command'
    assert_eq!(parsed[1]["id"].as_u64(), Some(1));
    assert_eq!(parsed[1]["type"].as_str(), Some("command"));

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result.data.contains("\n  "),
        "data must use 2-space indentation; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_filters_out_soft_deleted_items() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0, deleted=false), event(id=1, deleted=true), command(id=2, deleted=false)]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[
        {"id":0,"type":"event","text":"x","deleted":false},
        {"id":1,"type":"event","text":"y","deleted":true},
        {"id":2,"type":"command","text":"z","deleted":false}
      ]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as a JSON array of length 2
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed.as_array().map(Vec::len), Some(2));

    // @step Then the parsed array[0] has id=0
    assert_eq!(parsed[0]["id"].as_u64(), Some(0));

    // @step Then the parsed array[1] has id=2
    assert_eq!(parsed[1]["id"].as_u64(), Some(2));
}

#[test]
fn scenario_treats_missing_deleted_field_as_retained() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0) (no deleted field)]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[{"id":0,"type":"event","text":"x"}]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as a JSON array of length 1
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed.as_array().map(Vec::len), Some(1));
}

#[test]
fn scenario_returns_empty_array_when_event_storm_items_empty() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as the empty JSON array
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed.as_array().map(Vec::len), Some(0));
}

#[test]
fn scenario_preserves_every_field_on_every_retained_item() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[policy(id=0, deleted=false, when='UserRegistered', then='SendWelcomeEmail', color='purple', type='policy', text='Send welcome email')]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[
        {"id":0,"type":"policy","text":"Send welcome email","when":"UserRegistered","then":"SendWelcomeEmail","color":"purple","deleted":false}
      ]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the parsed array[0] has type='policy', text='Send welcome email', when='UserRegistered', then='SendWelcomeEmail', color='purple'
    assert_eq!(parsed[0]["type"].as_str(), Some("policy"));
    assert_eq!(parsed[0]["text"].as_str(), Some("Send welcome email"));
    assert_eq!(parsed[0]["when"].as_str(), Some("UserRegistered"));
    assert_eq!(parsed[0]["then"].as_str(), Some("SendWelcomeEmail"));
    assert_eq!(parsed[0]["color"].as_str(), Some("purple"));
}

#[test]
fn scenario_escalates_malformed_work_units_dispatcher() {
    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_ref()
            .map(|e| e.contains("Failed to parse work-units.json"))
            .unwrap_or(false),
        "error must mention parse failure; got {result:?}"
    );
}

#[test]
fn scenario_missing_work_unit_id_surfaces_invalid_args_error() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0)]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[{"id":0,"type":"event","text":"x"}]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with no workUnitId argument
    let result = dispatch_ses(ws.path(), r#"{}"#);

    // @step Then the dispatcher returns success=false with an error message containing the substring 'failed to parse args'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_ref()
            .map(|e| e.to_lowercase().contains("failed to parse args"))
            .unwrap_or(false),
        "error must mention args parse failure; got {result:?}"
    );
}
