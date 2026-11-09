//! Feature: spec/features/workspace-aware-work-unit-details-and-reordering.feature
//!
//! RPC-450: get_work_unit_details, move_work_unit_up and move_work_unit_down
//! act on the workspace the caller names (resolved through the daemon's
//! workspace registry, like list_workspace_work_units), and the TUI names its
//! selected board workspace. The daemon scenarios drive a real
//! `SharedFspecService` through the embedded transport.

mod common;

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, OrFailErr, TestResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::{WorkUnitInfo, WorkspaceEntry, WorkspaceWorkUnits};
use common::MockBackend;
use serde_json::{json, Value};
use serial_test::serial;

fn project(
    root: &Path,
    name: &str,
    units: &[&str],
) -> Result<PathBuf, codelet_test_helpers::TestError> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join("spec")).or_fail("expected a value")?;
    let work_units: serde_json::Map<String, Value> = units
        .iter()
        .map(|id| {
            (
                (*id).to_string(),
                json!({"id": id, "title": format!("{name} {id}"), "type": "story", "status": "backlog"}),
            )
        })
        .collect();
    let store = json!({"workUnits": work_units, "states": {"backlog": units}});
    std::fs::write(
        path.join("spec/work-units.json"),
        serde_json::to_string_pretty(&store).or_fail("expected a value")?,
    )
    .or_fail("expected a value")?;
    Ok(std::fs::canonicalize(path).or_fail("expected a value")?)
}

fn backlog(workspace: &Path) -> Result<Vec<String>, codelet_test_helpers::TestError> {
    let store: Value = serde_json::from_str(
        &std::fs::read_to_string(workspace.join("spec/work-units.json"))
            .or_fail("expected a value")?,
    )
    .or_fail("expected a value")?;
    Ok(store["states"]["backlog"]
        .as_array()
        .or_fail("expected a value")?
        .iter()
        .map(|id| {
            id.as_str()
                .map(str::to_owned)
                .or_fail("a work-unit id string")
        })
        .collect::<Result<_, _>>()?)
}

/// The embedded transport over a daemon service started in `start`.
fn daemon(start: &Path) -> Result<Arc<dyn FspecBackend>, codelet_test_helpers::TestError> {
    let service = Arc::new(
        SharedFspecService::new(Arc::new(
            WorkUnitsWatcher::new(start).or_fail("expected a value")?,
        ))
        .with_cwd(start.to_path_buf()),
    );
    Ok(Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    )))
}

fn path(workspace: &Path) -> String {
    workspace.to_string_lossy().into_owned()
}

fn data_directory() -> Result<tempfile::TempDir, codelet_test_helpers::TestError> {
    let data = tempfile::tempdir().or_fail("expected a value")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf()).or_fail("expected a value")?;
    Ok(data)
}

// Scenario: Details and reordering act on the named workspace
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_details_and_reordering_act_on_the_named_workspace() -> TestResult {
    let _data = data_directory()?;
    let root = tempfile::tempdir().or_fail("expected a value")?;
    // @step Given a daemon started in workspace "alpha" and a second workspace "beta" with cards "BETA-1" and "BETA-2" in backlog
    let alpha = project(root.path(), "alpha", &["ALPHA-1", "ALPHA-2"])?;
    let beta = project(root.path(), "beta", &["BETA-1", "BETA-2"])?;
    let alpha_before =
        std::fs::read(alpha.join("spec/work-units.json")).or_fail("expected a value")?;
    let backend = daemon(&alpha)?;

    // @step When a client reads the details of "BETA-1" in "beta" and moves "BETA-2" up in "beta" and then down in "beta"
    let details = backend
        .get_work_unit_details(path(&beta), "BETA-1".into())
        .await
        .or_fail("expected a value")?;
    backend
        .move_work_unit_up(path(&beta), "BETA-2".into())
        .await
        .or_fail("expected a value")?;
    let after_up = backlog(&beta)?;
    backend
        .move_work_unit_down(path(&beta), "BETA-2".into())
        .await
        .or_fail("expected a value")?;

    // @step Then the details are beta's "BETA-1" record
    let details: Value = serde_json::from_str(&details).or_fail("expected a value")?;
    assert_eq!(details["workUnit"]["id"], "BETA-1");
    assert_eq!(details["workUnit"]["title"], "beta BETA-1");
    // @step And beta's backlog order changes and alpha's work-units.json is unchanged
    assert_eq!(after_up, ["BETA-2", "BETA-1"]);
    assert_eq!(backlog(&beta)?, ["BETA-1", "BETA-2"]);
    assert_eq!(
        std::fs::read(alpha.join("spec/work-units.json")).or_fail("expected a value")?,
        alpha_before
    );
    Ok(())
}

// Scenario: The start workspace is addressed by its path
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_start_workspace_is_addressed_by_its_path() -> TestResult {
    let _data = data_directory()?;
    let root = tempfile::tempdir().or_fail("expected a value")?;
    // @step Given a daemon started in workspace "alpha" with cards "ALPHA-1" and "ALPHA-2" in backlog
    let alpha = project(root.path(), "alpha", &["ALPHA-1", "ALPHA-2"])?;
    let backend = daemon(&alpha)?;

    // @step When a client moves "ALPHA-2" up in "alpha"
    backend
        .move_work_unit_up(path(&alpha), "ALPHA-2".into())
        .await
        .or_fail("expected a value")?;

    // @step Then alpha's backlog order is "ALPHA-2", "ALPHA-1"
    assert_eq!(backlog(&alpha)?, ["ALPHA-2", "ALPHA-1"]);
    Ok(())
}

// Scenario: An unknown workspace is refused
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_unknown_workspace_is_refused() -> TestResult {
    let _data = data_directory()?;
    let root = tempfile::tempdir().or_fail("expected a value")?;
    // @step Given a daemon started in workspace "alpha"
    let alpha = project(root.path(), "alpha", &["ALPHA-1", "ALPHA-2"])?;
    let backend = daemon(&alpha)?;
    let missing = path(&root.path().join("no-such-workspace"));

    // @step When a client reads card details or moves a card in a workspace path that does not exist
    let errors = [
        backend
            .get_work_unit_details(missing.clone(), "ALPHA-1".into())
            .await
            .map(|_| ())
            .map_err(|error| error.to_string()),
        backend
            .move_work_unit_up(missing.clone(), "ALPHA-2".into())
            .await
            .map_err(|error| error.to_string()),
        backend
            .move_work_unit_down(missing.clone(), "ALPHA-1".into())
            .await
            .map_err(|error| error.to_string()),
    ];

    // @step Then each call returns an error naming that path
    for result in errors {
        let error = result.or_fail_err("an unknown workspace must be refused")?;
        assert!(error.contains("no-such-workspace"), "{error}");
    }
    assert_eq!(backlog(&alpha)?, ["ALPHA-1", "ALPHA-2"]);
    Ok(())
}

async fn drain(app: &mut App) -> codelet_test_helpers::TestResult {
    loop {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        let Some(action) = app.try_recv_action() else {
            break;
        };
        app.dispatch(action);
    }
    Ok(())
}

fn unit(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.into(),
        title: id.into(),
        work_type: "story".into(),
        status: "backlog".into(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

// Scenario: The TUI acts on its selected board workspace
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_tui_acts_on_its_selected_board_workspace() -> TestResult {
    let mock = Arc::new(MockBackend::new());
    let mut app = App::new(mock.clone());
    // @step Given the TUI board has workspace "beta" selected
    app.board_store_mut().set_workspaces(vec![
        WorkspaceEntry {
            path: "/work/alpha".into(),
            name: "alpha".into(),
            start: true,
        },
        WorkspaceEntry {
            path: "/work/beta".into(),
            name: "beta".into(),
            start: false,
        },
    ]);
    assert!(app.board_store_mut().select_workspace("/work/beta"));
    app.dispatch(Action::WorkspaceWorkUnitsLoaded(WorkspaceWorkUnits {
        workspace: "/work/beta".into(),
        units: vec![unit("BETA-1"), unit("BETA-2")],
    }));
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 1);

    // @step When I open the details of a card and move a card up and down
    app.dispatch(Action::RequestBoardCardDetails {
        request_generation: 1,
        work_unit_id: "BETA-1".into(),
    });
    app.dispatch(Action::ReorderUp);
    app.dispatch(Action::ReorderDown);
    drain(&mut app).await?;

    // @step Then each request names workspace "beta"
    let mut requests = mock.card_requests();
    requests.sort();
    assert_eq!(
        requests,
        vec![
            (
                "details".to_string(),
                "/work/beta".to_string(),
                "BETA-1".to_string()
            ),
            (
                "down".to_string(),
                "/work/beta".to_string(),
                "BETA-2".to_string()
            ),
            (
                "up".to_string(),
                "/work/beta".to_string(),
                "BETA-2".to_string()
            ),
        ]
    );
    Ok(())
}
