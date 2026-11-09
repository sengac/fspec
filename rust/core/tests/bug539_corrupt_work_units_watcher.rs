//! BUG-539: a temporarily corrupt work-units.json must never break the board.
//!
//! Feature: spec/features/graceful-initialization-without-spec-directory.feature
//!
//! Every wait here is on a watcher event (a published snapshot or a
//! reported reload failure); the timeouts are only hang guards.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc_types::WorkUnitInfo;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::sync::broadcast::{error::TryRecvError, Receiver};

const CORRUPT: &str = r#"{"workUnits": {"AUTH-001": {"id": "AUTH-0"#;

fn workspace() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec_dir = dir.path().join("spec");
    fs::create_dir_all(&spec_dir).expect("mkdir spec");
    let path = spec_dir.join("work-units.json");
    (dir, path)
}

fn write_units(path: &Path, ids: &[&str]) {
    let entries: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(r#""{id}":{{"id":"{id}","title":"{id}","type":"story","status":"backlog"}}"#)
        })
        .collect();
    fs::write(
        path,
        format!(r#"{{"workUnits":{{{}}}}}"#, entries.join(",")),
    )
    .expect("write work-units.json");
}

fn ids(units: &[WorkUnitInfo]) -> Vec<String> {
    units.iter().map(|unit| unit.id.clone()).collect()
}

/// Wait for the next published snapshot whose ids are `expected`. Every
/// snapshot seen on the way must be non-empty (the board is never blanked).
async fn wait_for_board(rx: &mut Receiver<Vec<WorkUnitInfo>>, expected: &[&str]) {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        loop {
            let snapshot = rx.recv().await.expect("snapshot channel open");
            assert!(!snapshot.is_empty(), "an empty board was published");
            if ids(&snapshot) == expected {
                return;
            }
        }
    })
    .await
    .expect("the watcher never published the expected board");
}

async fn wait_for_failure(rx: &mut Receiver<String>) -> String {
    codelet_test_helpers::deadlock_guard("rx.recv()", rx.recv())
        .await
        .expect("the watcher never reported the reload failure")
        .expect("failure channel open")
}

/// Drain the snapshots published so far: none may differ from `last_good`.
fn assert_no_other_board_published(rx: &mut Receiver<Vec<WorkUnitInfo>>, last_good: &[&str]) {
    loop {
        match rx.try_recv() {
            Ok(snapshot) => assert_eq!(
                ids(&snapshot),
                last_good,
                "a corrupt re-read must not publish a different board"
            ),
            Err(TryRecvError::Empty) => return,
            Err(error) => panic!("snapshot channel failed: {error}"),
        }
    }
}

/// Scenario: Start with an empty board when work-units.json is corrupt, then reload once it is fixed
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_corrupt_then_fix() {
    // @step Given the spec directory exists
    let (dir, path) = workspace();
    // @step But the work-units.json file contains invalid JSON
    fs::write(&path, CORRUPT).expect("write corrupt file");

    // @step When fspec starts
    let watcher = WorkUnitsWatcher::new(dir.path());

    // @step Then it starts instead of aborting and shows an empty Kanban board
    let watcher = watcher.expect("a corrupt board must not abort startup");
    assert!(watcher.snapshot().is_empty(), "the board starts empty");

    // @step And it keeps watching the file
    let mut snapshots = watcher.subscribe();

    // @step When the work-units.json file is fixed with valid JSON
    write_units(&path, &["AUTH-001", "AUTH-002"]);

    // @step Then the board reloads on its own and shows the work units
    wait_for_board(&mut snapshots, &["AUTH-001", "AUTH-002"]).await;
    let mut board = ids(&watcher.snapshot());
    board.sort();
    assert_eq!(board, ["AUTH-001", "AUTH-002"]);
}

/// Scenario: A corrupt re-read while running keeps the last good board until the file is fixed
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn corrupt_mid_session_then_fix() {
    // @step Given fspec is running and the board shows the work unit "AUTH-001"
    let (dir, path) = workspace();
    write_units(&path, &["AUTH-001"]);
    let watcher = WorkUnitsWatcher::new(dir.path()).expect("watcher");
    assert_eq!(ids(&watcher.snapshot()), ["AUTH-001"]);
    let mut snapshots = watcher.subscribe();
    let mut failures = watcher.subscribe_reload_failures();

    // @step When the work-units.json file becomes invalid JSON
    fs::write(&path, CORRUPT).expect("write corrupt file");

    // @step Then the reload failure is reported
    let failure = wait_for_failure(&mut failures).await;
    assert!(
        failure.contains("work-units.json"),
        "the failure names the file: {failure}"
    );

    // @step And the board still shows "AUTH-001" and no empty board is published
    assert_eq!(ids(&watcher.snapshot()), ["AUTH-001"]);
    assert_no_other_board_published(&mut snapshots, &["AUTH-001"]);

    // @step When the work-units.json file is fixed with the work unit "AUTH-002"
    write_units(&path, &["AUTH-002"]);

    // @step Then the board reloads on its own and shows "AUTH-002"
    wait_for_board(&mut snapshots, &["AUTH-002"]).await;
    assert_eq!(ids(&watcher.snapshot()), ["AUTH-002"]);
}

/// Scenario: Repeated break and fix cycles never blank the board
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repeated_break_and_fix_cycles() {
    // @step Given fspec is running and the board shows a valid work-units.json
    let (dir, path) = workspace();
    write_units(&path, &["CYCLE-000"]);
    let watcher = WorkUnitsWatcher::new(dir.path()).expect("watcher");
    let mut snapshots = watcher.subscribe();
    let mut failures = watcher.subscribe_reload_failures();
    let mut last_good = "CYCLE-000".to_string();

    // @step When the work-units.json file is broken and fixed five times in a row
    for cycle in 1..=5 {
        fs::write(&path, CORRUPT).expect("write corrupt file");

        // @step Then each break is reported as a reload failure while the board keeps the last good content
        wait_for_failure(&mut failures).await;
        assert_eq!(
            ids(&watcher.snapshot()),
            [last_good.as_str()],
            "cycle {cycle}"
        );
        assert_no_other_board_published(&mut snapshots, &[last_good.as_str()]);

        // @step And after each fix the board shows the work units of that fix
        let fixed = format!("CYCLE-{cycle:03}");
        write_units(&path, &[fixed.as_str()]);
        wait_for_board(&mut snapshots, &[fixed.as_str()]).await;
        assert_eq!(ids(&watcher.snapshot()), [fixed.as_str()], "cycle {cycle}");
        last_good = fixed;
    }
}
