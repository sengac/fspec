//! CLI-017 — the fspec CLI passes the board-query flags of
//! `list-work-units` to the core command, and its help documents them.
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature
//!
//! The child runs with `TZ=Australia/Brisbane` and only absolute dates, so
//! the output never depends on the wall clock or the machine's zone.

use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const HELP_FIXTURE: &str = include_str!("fixtures/help/list-work-units.txt");

fn write_project(root: &Path) -> TestResult {
    let unit = |id: &str, kind: &str, created: &str| {
        json!({"id": id, "title": format!("{id} title"), "type": kind, "status": "backlog",
               "createdAt": created, "updatedAt": created})
    };
    let board = json!({
        "version": "0.7.1",
        "meta": {"version": "1.0.0", "lastUpdated": "2026-10-01T00:00:00.000Z"},
        "workUnits": {
            "BUG-001": unit("BUG-001", "bug", "2026-10-30T02:00:00.000Z"),
            "CLI-001": unit("CLI-001", "story", "2026-10-29T00:00:00.000Z"),
        },
        "states": {"backlog": ["BUG-001", "CLI-001"], "specifying": [], "testing": [],
                   "implementing": [], "validating": [], "done": [], "blocked": []},
    });
    std::fs::create_dir_all(root.join("spec"))?;
    std::fs::write(
        root.join("spec/work-units.json"),
        serde_json::to_string_pretty(&board)?,
    )?;
    Ok(())
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

#[test]
fn the_fspec_cli_passes_every_new_flag_to_the_core_command() -> TestResult {
    // @step Given a project whose board has bug BUG-001 created 2026-10-30T02:00:00Z and story CLI-001 created 2026-10-29T00:00:00Z
    let dir = tempfile::tempdir()?;
    write_project(dir.path())?;
    // @step When I run fspec with TZ=Australia/Brisbane and "list-work-units --type bug --created-since 2026-10-30 --fields id,createdAt --format csv"
    let (code, stdout, stderr) = fspec_in(
        dir.path(),
        &[
            "list-work-units",
            "--type",
            "bug",
            "--created-since",
            "2026-10-30",
            "--fields",
            "id,createdAt",
            "--format",
            "csv",
        ],
    )?;
    // @step Then the command exits 0 and prints the lines "id,createdAt" and "BUG-001,2026-10-30T12:00:00+10:00"
    assert_eq!(code, 0, "stderr={stderr}");
    assert_eq!(stdout, "id,createdAt\nBUG-001,2026-10-30T12:00:00+10:00\n");
    // @step When I run fspec with TZ=Australia/Brisbane and "list-work-units --status donee"
    let (code, _, stderr) = fspec_in(dir.path(), &["list-work-units", "--status", "donee"])?;
    // @step Then the command exits 1 and stderr contains "Unknown status 'donee'"
    assert_eq!(code, 1, "stderr={stderr}");
    assert!(stderr.contains("Unknown status 'donee'"), "{stderr}");
    Ok(())
}

#[test]
fn the_list_work_units_help_documents_the_board_queries() -> TestResult {
    // @step Given the fspec binary has been compiled
    let dir = tempfile::tempdir()?;
    // @step When I run "fspec list-work-units --help"
    let (code, stdout, stderr) = fspec_in(dir.path(), &["list-work-units", "--help"])?;
    assert_eq!(code, 0, "stderr={stderr}");
    // @step Then stdout is byte-for-byte identical to the fixture rust/fspec/tests/fixtures/help/list-work-units.txt
    assert_eq!(stdout, HELP_FIXTURE);
    // @step And the help lists --done-since, --entered, --has-open-questions, --blocked-by, --fields, --group-by and --sort
    for flag in [
        "--done-since",
        "--entered <status>",
        "--has-open-questions",
        "--blocked-by",
        "--fields",
        "--group-by",
        "--sort",
    ] {
        assert!(stdout.contains(flag), "help lacks {flag}");
    }
    Ok(())
}
