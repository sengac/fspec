//! BUG-539: the real `fspec daemon` (the same build_service the embedded
//! TUI uses) starts with an empty board when work-units.json is corrupt and
//! reloads the board on its own once the file is valid again.
//!
//! Feature: spec/features/graceful-initialization-without-spec-directory.feature
//!
//! The reload is awaited on the backend's work-unit broadcast; the timeout
//! is only a hang guard.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(unix)]

mod common;

use std::time::Duration;

use codelet_fspec_tui::{FspecBackend, WebSocketFspecBackend};
use common::spawn_fspec_daemon;
use url::Url;

/// Scenario: Start with an empty board when work-units.json is corrupt, then reload once it is fixed
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn daemon_starts_with_an_empty_board_when_work_units_json_is_corrupt() {
    // @step Given the spec directory exists
    let workspace = tempfile::tempdir().expect("tempdir");
    let spec_dir = workspace.path().join("spec");
    std::fs::create_dir_all(&spec_dir).expect("mkdir spec");
    let path = spec_dir.join("work-units.json");
    // @step But the work-units.json file contains invalid JSON
    std::fs::write(&path, r#"{"workUnits": {"AUTH-001": {"id": "AUTH-0"#).expect("write");

    // @step When fspec starts
    // (spawn_fspec_daemon panics unless the daemon prints its port, i.e.
    // unless build_service, which also backs the embedded TUI, succeeds.)
    let daemon_home = common::TestHome::new();
    let (_guard, port) = spawn_fspec_daemon(workspace.path(), &daemon_home);

    // @step Then it starts instead of aborting and shows an empty Kanban board
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).expect("ws url");
    let backend = WebSocketFspecBackend::connect(url).await.expect("connect");
    let mut updates = backend.work_units_rx();
    let board = backend.list_work_units().await.expect("list_work_units");
    assert!(board.is_empty(), "the board starts empty: {board:?}");

    // @step And it keeps watching the file
    // @step When the work-units.json file is fixed with valid JSON
    std::fs::write(
        &path,
        r#"{"workUnits":{"AUTH-001":{"id":"AUTH-001","title":"Login","type":"story","status":"backlog"}}}"#,
    )
    .expect("fix work-units.json");

    // @step Then the board reloads on its own and shows the work units
    let reloaded = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let units = updates.recv().await.expect("work-unit updates open");
            if units.iter().any(|unit| unit.id == "AUTH-001") {
                return units;
            }
        }
    })
    .await
    .expect("the daemon never published the fixed board");
    assert_eq!(reloaded.len(), 1);
    let listed = backend.list_work_units().await.expect("list_work_units");
    assert_eq!(
        listed
            .iter()
            .map(|unit| unit.id.as_str())
            .collect::<Vec<_>>(),
        ["AUTH-001"]
    );
}
