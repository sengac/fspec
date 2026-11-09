// Feature: spec/features/codelet-core-spec-writers-share-the-spec-lock.feature
//
// BUG-477: the codelet-core writers of spec JSON (the TUI work-unit reorder
// and the scheduler's last-run update) must contend on the same spec lock as
// fspec commands. A child process (this test binary re-executed with
// BUG477_HOLD_ROOT set) takes `spec/.fspec-write.lock` with the raw fs2
// protocol and keeps it until the parent closes its stdin (TEST-042: the
// handshake is a line on the child's stdout, not a polled file).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use codelet_core::work_units_write::{move_work_unit, Direction};
use fs2::FileExt;
use serde_json::json;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LOCK_FILE: &str = ".fspec-write.lock";

fn seed(root: &Path) -> TestResult {
    let spec = root.join("spec");
    fs::create_dir_all(&spec)?;
    let unit = |id: &str| {
        json!({"id": id, "title": id, "type": "story", "status": "backlog",
               "createdAt": "2026-11-04T00:00:00.000Z", "updatedAt": "2026-11-04T00:00:00.000Z"})
    };
    let board = json!({
        "meta": {"version": "0.7.1", "lastUpdated": "2026-11-04T00:00:00.000Z"},
        "workUnits": {"A-001": unit("A-001"), "B-001": unit("B-001")},
        "states": {"backlog": ["A-001", "B-001"], "specifying": [], "testing": [],
                   "implementing": [], "validating": [], "done": [], "blocked": []}
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&board)?,
    )?;
    let schedules = json!({
        "version": "1.0",
        "schedules": {"nightly": {"cron": "0 0 * * *", "timezone": "UTC", "status": "active",
                                   "job_type": "shell", "created_at": null, "last_run_at": null,
                                   "last_run_status": null, "agent": null,
                                   "shell": {"command": "true"}, "overlap_policy": null}}
    });
    fs::write(
        spec.join("schedules.json"),
        serde_json::to_string_pretty(&schedules)?,
    )?;
    Ok(())
}

const HOLDER_READY: &str = "bug477-core-holder-holds-the-spec-lock";

fn spawn_holder(root: &Path) -> Result<Child, Box<dyn std::error::Error>> {
    let mut child = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "bug477_lock_holder_child",
            "--nocapture",
            "--quiet",
        ])
        .env("BUG477_HOLD_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("the holder's stdout is piped")?;
    let mut lines = BufReader::new(stdout).lines();
    loop {
        match lines.next() {
            Some(line) => {
                if line?.trim() == HOLDER_READY {
                    break;
                }
            }
            None => return Err("lock holder exited before it held the lock".into()),
        }
    }
    // Keep reading, so the holder's later output never hits a closed pipe.
    std::thread::spawn(move || lines.for_each(drop));
    Ok(child)
}

/// Child-process entry point. A no-op unless BUG477_HOLD_ROOT is set.
#[test]
fn bug477_lock_holder_child() -> TestResult {
    let Ok(root) = std::env::var("BUG477_HOLD_ROOT") else {
        return Ok(());
    };
    let root = Path::new(&root);
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("spec").join(LOCK_FILE))?;
    file.lock_exclusive()?;
    let mut stdout = std::io::stdout();
    writeln!(stdout, "{HOLDER_READY}")?;
    stdout.flush()?;
    // Hold the lock until the parent closes stdin.
    std::io::stdin().read_to_end(&mut Vec::new())?;
    FileExt::unlock(&file)?;
    Ok(())
}

#[test]
fn the_tui_reorder_and_the_scheduler_use_the_same_spec_lock() -> TestResult {
    let project = TempDir::new()?;
    let root = project.path();
    seed(root)?;
    let board_path = root.join("spec").join("work-units.json");
    let schedules_path = root.join("spec").join("schedules.json");
    let board_before = fs::read(&board_path)?;
    let schedules_before = fs::read(&schedules_path)?;

    // @step Given another process holds the spec lock and does not release it
    let mut holder = spawn_holder(root)?;

    // @step When the TUI moves a work unit up and the scheduler records a run
    // (with the lock budget shortened through its documented override; this
    // binary's only other test is the no-op child entry point).
    std::env::set_var("FSPEC_SPEC_LOCK_TIMEOUT_MS", "3000");
    let reorder = move_work_unit(root, "B-001", Direction::Up);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let scheduler = runtime.block_on(codelet_core::scheduler::trigger::update_last_run(
        &schedules_path,
        "nightly",
        "success",
    ));
    drop(holder.stdin.take());
    let holder_status = holder.wait()?;
    assert!(holder_status.success());

    // @step Then both fail with "Failed to acquire file lock"
    let reorder_error = match reorder {
        Ok(()) => String::from("reorder succeeded while the spec lock was held"),
        Err(e) => format!("{e:#}"),
    };
    assert!(
        reorder_error.contains("Failed to acquire file lock"),
        "move_work_unit: {reorder_error}"
    );
    let scheduler_error = match scheduler {
        Ok(()) => String::from("scheduler update succeeded while the spec lock was held"),
        Err(e) => format!("{e:#}"),
    };
    assert!(
        scheduler_error.contains("Failed to acquire file lock"),
        "update_last_run: {scheduler_error}"
    );

    // @step And spec/work-units.json and spec/schedules.json are byte-for-byte unchanged
    assert_eq!(fs::read(&board_path)?, board_before);
    assert_eq!(fs::read(&schedules_path)?, schedules_before);
    Ok(())
}
