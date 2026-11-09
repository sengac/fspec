//! CLI-020 — one name for a work-unit reference on every surface:
//! `--work-unit`, `<workUnitId>`, `id` on a card object and `workUnitId` on
//! every reference, `doneAt`, `-p` for `--parent`, with no aliases.
//!
//! Feature: spec/features/one-name-for-a-work-unit-reference-on-every-surface.feature
//!
//! Every scenario runs the real fspec binary, the core dispatcher or the
//! MCP catalog on fixture boards in a temporary project (no clock reads in
//! assertions, no network, no sleeps). The guard scenario lives in
//! rust/fspec/src/list_work_units/work_unit_name_guard_tests.rs because it
//! needs the binary-private `Cli`.

use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

mod common;

use common::{fspec_bin, project_root};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const T: &str = "2026-10-01T00:00:00.000Z";

fn card(id: &str, extra: Value) -> Value {
    let mut card = json!({"id": id, "title": format!("{id} title"), "type": "story",
                          "status": "backlog", "createdAt": T, "updatedAt": T});
    if let (Some(card), Some(extra)) = (card.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            card.insert(key.clone(), value.clone());
        }
    }
    card
}

/// A project whose board holds `cards` (all in backlog).
fn project(cards: Vec<Value>) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let ids: Vec<Value> = cards.iter().map(|c| c["id"].clone()).collect();
    let mut units = serde_json::Map::new();
    for c in cards {
        if let Some(id) = c["id"].as_str() {
            units.insert(id.to_string(), c.clone());
        }
    }
    let board = json!({
        "version": "0.7.1",
        "meta": {"version": "1.0.0", "lastUpdated": T},
        "workUnits": units,
        "states": {"backlog": ids, "specifying": [], "testing": [], "implementing": [],
                   "validating": [], "done": [], "blocked": []},
    });
    let mut prefixes = serde_json::Map::new();
    for id in ids.iter().filter_map(Value::as_str) {
        let prefix = id.split('-').next().unwrap_or(id);
        prefixes.insert(
            prefix.to_string(),
            json!({"prefix": prefix, "description": prefix, "createdAt": T}),
        );
    }
    std::fs::create_dir_all(dir.path().join("spec"))?;
    std::fs::write(
        dir.path().join("spec/work-units.json"),
        serde_json::to_string_pretty(&board)?,
    )?;
    std::fs::write(
        dir.path().join("spec/prefixes.json"),
        serde_json::to_string_pretty(&json!({"prefixes": prefixes}))?,
    )?;
    Ok(dir)
}

fn fspec_in(root: &Path, args: &[&str]) -> Result<(i32, String, String), std::io::Error> {
    let out = Command::new(fspec_bin())
        .args(args)
        .current_dir(root)
        .env("TZ", "Australia/Brisbane")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()?;
    Ok((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

fn dispatch(root: &Path, command: &str, args: Value) -> codelet_fspec_core::DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

/// The lines of `help` between the line `heading` and the next blank line.
fn section<'a>(help: &'a str, heading: &str) -> Vec<&'a str> {
    help.lines()
        .skip_while(|line| *line != heading)
        .skip(1)
        .take_while(|line| !line.is_empty())
        .collect()
}

#[test]
fn list_work_units_selects_cards_with_work_unit_and_rejects_id() -> TestResult {
    // @step Given a board with the cards BUG-600, BUG-601, BUG-602 and BUG-603
    let dir = project(
        ["BUG-600", "BUG-601", "BUG-602", "BUG-603"]
            .iter()
            .map(|id| card(id, json!({})))
            .collect(),
    )?;
    // @step When I run "fspec list-work-units --work-unit BUG-600..BUG-602 --format ids"
    let (code, stdout, stderr) = fspec_in(
        dir.path(),
        &[
            "list-work-units",
            "--work-unit",
            "BUG-600..BUG-602",
            "--format",
            "ids",
        ],
    )?;
    // @step Then the output is BUG-600, BUG-601 and BUG-602, one per line
    assert_eq!(code, 0, "stderr={stderr}");
    assert_eq!(stdout, "BUG-600\nBUG-601\nBUG-602\n");
    // @step When I run "fspec list-work-units --id BUG-600"
    let (code, _, stderr) = fspec_in(dir.path(), &["list-work-units", "--id", "BUG-600"])?;
    // @step Then the command fails with exactly "error: unknown option '--id'"
    assert_ne!(code, 0);
    assert_eq!(stderr.trim_end(), "error: unknown option '--id'");
    Ok(())
}

#[test]
fn query_metrics_takes_work_unit_and_rejects_work_unit_id() -> TestResult {
    // @step Given a board with the card AUTH-001
    let history = json!([{"state": "backlog", "timestamp": T},
                         {"state": "specifying", "timestamp": "2026-10-01T05:00:00.000Z"}]);
    let dir = project(vec![card("AUTH-001", json!({"stateHistory": history}))])?;
    // @step When I run "fspec query-metrics --work-unit AUTH-001 --format json"
    let (code, stdout, stderr) = fspec_in(
        dir.path(),
        &[
            "query-metrics",
            "--work-unit",
            "AUTH-001",
            "--format",
            "json",
        ],
    )?;
    // @step Then the output holds the metrics of AUTH-001
    assert_eq!(code, 0, "stderr={stderr}");
    let metrics: Value = serde_json::from_str(stdout.trim_end())?;
    assert!(
        metrics.get("cycleTime").is_some(),
        "single-card metrics: {stdout}"
    );
    let core = dispatch(
        dir.path(),
        "query-metrics",
        json!({"workUnitId": "AUTH-001", "format": "json"}),
    );
    assert!(core.success, "{core:?}");
    assert_eq!(stdout, format!("{}\n", core.data));
    // @step When I run "fspec query-metrics --work-unit-id AUTH-001"
    let (code, _, stderr) = fspec_in(dir.path(), &["query-metrics", "--work-unit-id", "AUTH-001"])?;
    // @step Then the command fails with exactly "error: unknown option '--work-unit-id'"
    assert_ne!(code, 0);
    assert_eq!(stderr.trim_end(), "error: unknown option '--work-unit-id'");
    Ok(())
}

#[test]
fn p_means_parent_and_prefix_has_no_short_flag() -> TestResult {
    // @step Given a board with the card AUTH-001 and its child AUTH-002, and the card BUG-001
    let dir = project(vec![
        card("AUTH-001", json!({"children": ["AUTH-002"]})),
        card("AUTH-002", json!({"parent": "AUTH-001"})),
        card("BUG-001", json!({"type": "bug"})),
    ])?;
    // @step When I run "fspec list-work-units -p AUTH-001 --format ids"
    let (code, stdout, stderr) = fspec_in(
        dir.path(),
        &["list-work-units", "-p", "AUTH-001", "--format", "ids"],
    )?;
    // @step Then the output is AUTH-002
    assert_eq!(code, 0, "stderr={stderr}");
    assert_eq!(stdout, "AUTH-002\n");
    // @step When I run "fspec list-work-units --prefix BUG --format ids"
    let (code, stdout, stderr) = fspec_in(
        dir.path(),
        &["list-work-units", "--prefix", "BUG", "--format", "ids"],
    )?;
    // @step Then the output is BUG-001
    assert_eq!(code, 0, "stderr={stderr}");
    assert_eq!(stdout, "BUG-001\n");
    Ok(())
}

#[test]
fn link_flags_show_the_work_unit_id_placeholder() -> TestResult {
    let dir = tempfile::tempdir()?;
    // @step When I run "fspec add-dependency --help"
    let (code, help, stderr) = fspec_in(dir.path(), &["add-dependency", "--help"])?;
    assert_eq!(code, 0, "stderr={stderr}");
    // @step Then the USAGE is "fspec add-dependency <workUnitId> [workUnitId] [options]"
    assert_eq!(
        section(&help, "USAGE"),
        vec!["  fspec add-dependency <workUnitId> [workUnitId] [options]"]
    );
    // @step And the options are "--blocks <workUnitId>", "--blocked-by <workUnitId>", "--depends-on <workUnitId>" and "--relates-to <workUnitId>"
    let flags: Vec<&str> = help
        .lines()
        .filter(|line| line.starts_with("  --"))
        .map(str::trim)
        .collect();
    assert_eq!(
        flags,
        vec![
            "--blocks <workUnitId>",
            "--blocked-by <workUnitId>",
            "--depends-on <workUnitId>",
            "--relates-to <workUnitId>",
        ]
    );
    Ok(())
}

#[test]
fn copy_virtual_hooks_and_create_story_show_work_unit_id_for_their_card_flags() -> TestResult {
    let dir = tempfile::tempdir()?;
    // @step When I run "fspec copy-virtual-hooks --help"
    let (code, help, stderr) = fspec_in(dir.path(), &["copy-virtual-hooks", "--help"])?;
    assert_eq!(code, 0, "stderr={stderr}");
    // @step Then the USAGE is "fspec copy-virtual-hooks --from <workUnitId> --to <workUnitId> [--hook-name <name>]"
    assert_eq!(
        section(&help, "USAGE"),
        vec![
            "  fspec copy-virtual-hooks --from <workUnitId> --to <workUnitId> [--hook-name <name>]"
        ]
    );
    // @step When I run "fspec create-story --help"
    let (code, help, stderr) = fspec_in(dir.path(), &["create-story", "--help"])?;
    assert_eq!(code, 0, "stderr={stderr}");
    // @step Then the options show "-p, --parent <workUnitId>"
    assert!(
        help.lines()
            .any(|line| line == "  -p, --parent <workUnitId>"),
        "{help}"
    );
    Ok(())
}

#[test]
fn audit_and_research_json_args_reference_the_card_as_work_unit_id() -> TestResult {
    // BUG-695: research reads the user's fspec config from the user
    // directory, which a test-support build resolves only when isolated.
    codelet_test_helpers::isolated_user_directory()?;
    // @step Given a board with the card AUTH-001
    let dir = project(vec![card("AUTH-001", json!({}))])?;
    std::fs::create_dir_all(dir.path().join("spec/features"))?;
    // @step When the audit and research commands run with the args {"workUnitId": "AUTH-001"}
    let audit = dispatch(dir.path(), "audit", json!({"workUnitId": "AUTH-001"}));
    let research = dispatch(dir.path(), "research", json!({"workUnitId": "AUTH-001"}));
    // @step Then both accept the card
    for result in [&audit, &research] {
        let error = result.error.clone().unwrap_or_default();
        assert!(!error.contains("unknown field"), "{result:?}");
    }
    assert!(research.success, "{research:?}");
    // @step When they run with the args {"workUnit": "AUTH-001"}
    for command in ["audit", "research"] {
        let old = dispatch(dir.path(), command, json!({"workUnit": "AUTH-001"}));
        // @step Then both fail with an unknown field `workUnit` error
        assert!(!old.success, "{command}: {old:?}");
        let error = old.error.unwrap_or_default();
        assert!(
            error.contains("unknown field `workUnit`"),
            "{command}: {error}"
        );
    }
    Ok(())
}

/// The MCP tools whose arguments name a card.
const CHECKPOINT_TOOLS: &[&str] = &[
    "checkpoint_diff_files",
    "checkpoint_file_diff",
    "restore_checkpoint_all",
    "delete_checkpoint",
    "checkpoint_diff_files_in_workspace",
];

#[test]
fn mcp_checkpoint_tools_take_work_unit_id() -> TestResult {
    // @step When the MCP tool checkpoint_diff_files is validated with {"workUnitId": "AUTH-001", "name": "baseline"}
    let new = codelet_rpc::mcp_dispatch::validate(
        "checkpoint_diff_files",
        json!({"workUnitId": "AUTH-001", "name": "baseline"}),
    );
    // @step Then the arguments are accepted
    assert!(new.is_ok(), "{new:?}");
    // @step And the catalog schema of every checkpoint tool lists the property workUnitId and no work_unit_id
    let methods = codelet_rpc::mcp_dispatch::methods()?;
    let mut with_card = Vec::new();
    for method in &methods {
        let schema = method.schema.to_string();
        assert!(
            !schema.contains("\"work_unit_id\""),
            "{}: {schema}",
            method.name
        );
        if schema.contains("\"workUnitId\"") {
            with_card.push(method.name);
        }
    }
    for tool in CHECKPOINT_TOOLS {
        assert!(
            with_card.contains(tool),
            "{tool} lacks workUnitId: {with_card:?}"
        );
    }
    assert_eq!(with_card.len(), 11, "{with_card:?}");
    // @step When it is validated with {"work_unit_id": "AUTH-001", "name": "baseline"}
    let old = codelet_rpc::mcp_dispatch::validate(
        "checkpoint_diff_files",
        json!({"work_unit_id": "AUTH-001", "name": "baseline"}),
    );
    // @step Then it fails with an unknown field `work_unit_id` error
    let error = old.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(error.contains("unknown field `work_unit_id`"), "{error}");
    Ok(())
}

#[test]
fn auto_advance_to_done_records_done_at() -> TestResult {
    // @step Given the card AUTH-001 in validating
    let dir = project(vec![card("AUTH-001", json!({"status": "validating"}))])?;
    let path = dir.path().join("spec/work-units.json");
    let mut board: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    board["states"]["backlog"] = json!([]);
    board["states"]["validating"] = json!(["AUTH-001"]);
    std::fs::write(&path, serde_json::to_string_pretty(&board)?)?;
    // @step When auto-advance moves AUTH-001 from validating on the validation-pass event
    let result = dispatch(
        dir.path(),
        "auto-advance",
        json!({"workUnitId": "AUTH-001", "from": "validating", "event": "validation-pass"}),
    );
    assert!(result.success, "{result:?}");
    // @step Then AUTH-001 is done and list-work-units lists its doneAt
    let stored: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    let unit = &stored["workUnits"]["AUTH-001"];
    assert_eq!(unit["status"], "done");
    let listed = dispatch(
        dir.path(),
        "list-work-units",
        json!({"workUnitId": "AUTH-001", "fields": "id,doneAt", "format": "json"}),
    );
    assert!(listed.success, "{listed:?}");
    let listed: Value = serde_json::from_str(&listed.data)?;
    assert!(
        listed["workUnits"][0]["doneAt"]
            .as_str()
            .is_some_and(|at| !at.is_empty()),
        "{listed}"
    );
    // @step And AUTH-001 has no completedAt or stored doneAt key
    assert!(unit.get("completedAt").is_none(), "{unit}");
    assert!(unit.get("doneAt").is_none(), "{unit}");
    Ok(())
}

/// The lines of the document section `heading` (up to the next heading).
fn document_section(
    doc: &codelet_fspec_tui::components::board_card::document::Document,
    heading: &str,
) -> Vec<String> {
    use codelet_fspec_tui::components::board_card::document::LineKind;
    doc.lines
        .iter()
        .skip_while(|line| !(matches!(line.kind, LineKind::Heading) && line.text == heading))
        .skip(1)
        .take_while(|line| !matches!(line.kind, LineKind::Heading))
        .map(|line| line.text.clone())
        .collect()
}

#[test]
fn the_board_holds_only_the_new_keys_after_the_one_time_migration() -> TestResult {
    // @step Given the repository board after the CLI-020 migration
    let raw = std::fs::read_to_string(project_root().join("spec/work-units.json"))?;
    let board: Value = serde_json::from_str(&raw)?;
    let units = board["workUnits"].as_object().ok_or("workUnits")?;
    assert!(units.len() > 1000, "expected the repository board");
    // @step Then no card has the key updated, statusHistory or completedAt
    let legacy: Vec<String> = units
        .iter()
        .flat_map(|(id, unit)| {
            ["updated", "statusHistory", "completedAt"]
                .into_iter()
                .filter(|key| unit.get(*key).is_some())
                .map(move |key| format!("{id}: {key}"))
        })
        .collect();
    assert!(
        legacy.is_empty(),
        "{} legacy keys: {legacy:?}",
        legacy.len()
    );
    // @step When the TUI renders the card document of a card that has updated and statusHistory keys
    let record = card(
        "AUTH-001",
        json!({"updated": "OLD-UPDATED",
               "stateHistory": [{"state": "backlog", "timestamp": T}],
               "statusHistory": [{"status": "OLD-HISTORY"}]}),
    );
    let record = record.as_object().ok_or("record")?;
    let doc = codelet_fspec_tui::components::board_card::document::project(record);
    // @step Then the document shows its updatedAt and stateHistory and no updated or statusHistory entry in the History or Timestamps sections
    let history = document_section(&doc, "History");
    let timestamps = document_section(&doc, "Timestamps");
    assert!(
        history.iter().any(|l| l.starts_with("State History")),
        "{history:?}"
    );
    assert!(
        timestamps.iter().any(|l| l == &format!("Updated At: {T}")),
        "{timestamps:?}"
    );
    for line in history.iter().chain(timestamps.iter()) {
        assert!(
            !line.contains("OLD-"),
            "{line}: history {history:?} timestamps {timestamps:?}"
        );
    }
    Ok(())
}

#[test]
fn error_hints_and_docs_name_the_card_work_unit_id() -> TestResult {
    let dir = tempfile::tempdir()?;
    // @step When I run "fspec add-hotspot --help"
    let (code, help, stderr) = fspec_in(dir.path(), &["add-hotspot", "--help"])?;
    assert_eq!(code, 0, "stderr={stderr}");
    // @step Then its COMMON ERRORS fix says "Ensure work unit exists: fspec show-work-unit <workUnitId>"
    assert!(
        section(&help, "COMMON ERRORS")
            .iter()
            .any(|line| line.trim()
                == "Fix: Ensure work unit exists: fspec show-work-unit <workUnitId>"),
        "{help}"
    );
    // @step And the init docs, their fixtures and the bootstrap doc say "fspec show-work-unit <workUnitId>" and hold no "<work-unit-id>" and no card "<id>"
    let core = project_root().join("rust/fspec-core");
    let mut docs: Vec<std::path::PathBuf> = ["plain_cli", "plain_ext", "plain_ide", "sysrem_cli"]
        .iter()
        .map(|kind| core.join(format!("src/commands/init_doc_{kind}.md")))
        .collect();
    docs.push(core.join("src/commands/bootstrap_doc.txt"));
    for entry in std::fs::read_dir(core.join("tests/fixtures/init_docs"))? {
        docs.push(entry?.path());
    }
    for path in docs {
        let text = std::fs::read_to_string(&path)?;
        let name = path.display();
        assert!(text.contains("fspec show-work-unit <workUnitId>"), "{name}");
        assert!(!text.contains("<work-unit-id>"), "{name}");
        assert!(!text.contains("show-work-unit <id>"), "{name}");
    }
    Ok(())
}
