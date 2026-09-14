//! BUG-182 — GitStateWatcher → TUI: git-state stream fold, mux-pane lazy
//! loads, selection stability on refresh, mux-aware redraw gate.
//!
//! Feature: spec/features/git-state-watcher.feature
//!
//! Covers the feature's App/transport scenarios:
//!   - Chrome bar branch + counts fold end-to-end through the embedded
//!     transport (real repo, real watcher);
//!   - Bootstrap subscriber fold via MockBackend;
//!   - Mux entry loads un-loaded lazy panes in place (R7);
//!   - Selection stability on refresh (R6, both views);
//!   - Mux-aware redraw gate (R5);
//!   - Refresh drop while the initial load is in flight (R4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend, Navigator, ViewMode};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::{ChangedFile, CheckpointCounts, CheckpointInfo, GitState, WorkspaceInfo};
use tempfile::TempDir;

use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::{ChangedFilesView, CheckpointsView};

mod common;
use common::MockBackend;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Create a basic test git repository with an initial commit (mirrors
/// `rust/git/tests/common/mod.rs::setup_test_repo`).
fn setup_test_repo() -> TempDir {
    let tmp_dir = tempfile::TempDir::new().expect("tempdir");
    let repo_path = tmp_dir.path();
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test User"],
    ] {
        std::process::Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .output()
            .expect("git");
    }
    fs::write(repo_path.join("README.md"), "# Test Repository\n").expect("write README");
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo_path)
        .output()
        .expect("git commit");
    tmp_dir
}

/// Build a `SharedFspecService` with a real (empty) watcher + the repo
/// cwd attached, so the GitStateWatcher has a live cwd to read.
fn service_for(repo: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo).expect("watcher on temp repo"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo.to_path_buf()))
}

/// Dispatch every action currently on the bus (best-effort, no wait).
fn drain_bus(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Wait (bounded) for `pred` to hold, draining the action bus on every
/// iteration so push frames fold before the check.
async fn wait_for<F: Fn(&App) -> bool>(app: &mut App, pred: F, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        drain_bus(app);
        if pred(app) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timeout: condition did not become true within {timeout:?}");
}

fn changed_file(path: &str, staged: bool) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        change_type: "M".to_string(),
        staged,
    }
}

fn checkpoint(work_unit_id: &str, name: &str) -> CheckpointInfo {
    CheckpointInfo {
        work_unit_id: work_unit_id.to_string(),
        name: name.to_string(),
        timestamp: "2026-01-01T00:00:00.000Z".to_string(),
        is_automatic: false,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The chrome bar git branch indicator updates when the branch
// changes without a TUI restart
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn the_chrome_bar_git_branch_indicator_updates_when_the_branch_changes_without_a_tui_restart()
{
    // @step Given an App on the embedded transport whose AgentViewStore holds WorkspaceInfo with git_branch main from bootstrap
    let tmp = setup_test_repo();
    let repo = tmp.path();
    std::process::Command::new("git")
        .args(["checkout", "-b", "main"])
        .current_dir(repo)
        .output()
        .expect("git checkout -b main");
    let service = service_for(repo);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);
    let initial_branch = app
        .agent_view_store()
        .workspace()
        .and_then(|w| w.git_branch.clone());
    assert_eq!(initial_branch.as_deref(), Some("main"), "bootstrap branch");

    // @step When the branch is switched to feature-x from another terminal
    std::process::Command::new("git")
        .args(["checkout", "-b", "feature-x"])
        .current_dir(repo)
        .output()
        .expect("git checkout -b feature-x");

    // @step Then the AgentViewStore workspace git_branch is feature-x and the BoardStore checkpoint counts are folded from the same GitState frame without a TUI restart
    wait_for(
        &mut app,
        |a| {
            a.agent_view_store()
                .workspace()
                .is_some_and(|w| w.git_branch.as_deref() == Some("feature-x"))
        },
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 0, auto: 0 },
        "counts stay folded from the GitState stream (no checkpoints in repo)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The bootstrap git-state subscriber folds frames into the
// BoardStore and AgentViewStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn the_bootstrap_git_state_subscriber_folds_frames_into_the_board_store_and_agent_view_store()
{
    // @step Given an App constructed with a backend whose git-state push channel is live, bootstrapped so the BoardStore holds CheckpointCounts { manual: 0, auto: 0 } and the AgentViewStore workspace git_branch is None
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 0, auto: 0 },
        "bootstrap leaves the BoardStore at zero counts (MockBackend default)"
    );
    assert!(
        app.agent_view_store()
            .workspace()
            .and_then(|w| w.git_branch.clone())
            .is_none(),
        "MockBackend workspace starts with git_branch None"
    );

    // @step When a GitState frame with checkpoint_counts { manual: 1, auto: 0 } and git_branch Some(main) is pushed onto the channel
    mock.push_git_state_changed(GitState {
        checkpoint_counts: CheckpointCounts { manual: 1, auto: 0 },
        git_branch: Some("main".to_string()),
        changed_files: Vec::new(),
        checkpoints: Vec::new(),
    });

    // @step Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 1, auto: 0 } and app.agent_view_store().workspace().git_branch() is Some(main)
    wait_for(
        &mut app,
        |a| {
            a.board_store().checkpoint_counts() == CheckpointCounts { manual: 1, auto: 0 }
                && a.agent_view_store()
                    .workspace()
                    .is_some_and(|w| w.git_branch.as_deref() == Some("main"))
        },
        Duration::from_millis(500),
    )
    .await;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 1, auto: 0 }
    );
    assert_eq!(
        app.agent_view_store()
            .workspace()
            .and_then(|w| w.git_branch.clone()),
        Some("main".to_string())
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Entering mux mode loads un-loaded lazy panes in place
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn entering_mux_mode_loads_un_loaded_lazy_panes_in_place() {
    // @step Given an App whose changed-files and checkpoints views have not loaded yet
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);
    assert!(
        app.navigator_mut().changed_files.load.is_loading(),
        "fresh ChangedFilesView starts in its initial load"
    );
    assert!(
        app.navigator_mut().checkpoints.load.is_loading(),
        "fresh CheckpointsView starts in its initial load"
    );

    // @step When mux mode is entered with a config that renders Board, Agent, ChangedFiles and Checkpoints panes
    app.dispatch(Action::SessionCreated(codelet_rpc_types::SessionId::new(
        "s-1",
    )));
    app.navigator_mut().mux.set_pane_list(
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
        ],
        None,
    );
    app.dispatch(Action::InputSubmitted("/mux on".to_string()));

    // @step Then the ChangedFiles and Checkpoints panes run their initial open flows in place (reset view + spawn the list load) without flipping the whole view out of Mux
    assert_eq!(app.navigator().active_view, ViewMode::Mux);

    // Both panes' initial loads flush (MockBackend default: empty lists).
    wait_for(
        &mut app,
        |a| !a.navigator().changed_files.load.is_loading(),
        Duration::from_millis(500),
    )
    .await;
    wait_for(
        &mut app,
        |a| !a.navigator().checkpoints.load.is_loading(),
        Duration::from_millis(500),
    )
    .await;
    assert!(
        mock.changed_files_calls() >= 1,
        "the ChangedFiles pane must have spawned the changed_files RPC (saw {})",
        mock.changed_files_calls()
    );
    assert!(
        mock.list_checkpoints_calls() >= 1,
        "the Checkpoints pane must have spawned the list_checkpoints RPC (saw {})",
        mock.list_checkpoints_calls()
    );
    assert_eq!(app.navigator().active_view, ViewMode::Mux);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Refreshing a loaded changed-files view keeps the selection
// stable by path
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn refreshing_a_loaded_changed_files_view_keeps_the_selection_stable_by_path() {
    // @step Given a loaded changed-files view with file b.txt selected out of [a.txt, b.txt, c.txt]
    let mut view = ChangedFilesView::new();
    view.set_files(vec![
        changed_file("a.txt", false),
        changed_file("b.txt", false),
        changed_file("c.txt", false),
    ]);
    view.load.mark_list_flushed();
    view.set_selected_index(1); // b.txt
    assert_eq!(view.selected_path().as_deref(), Some("b.txt"));

    // @step When a git-state refresh lands the new file list [a.txt, b.txt, d.txt]
    view.refresh_files_preserving_selection(vec![
        changed_file("a.txt", false),
        changed_file("b.txt", false),
        changed_file("d.txt", false),
    ]);

    // @step Then b.txt remains selected (re-selected by path, not index) and its diff reloads
    assert_eq!(
        view.selected_path().as_deref(),
        Some("b.txt"),
        "selection must survive the refresh by PATH"
    );
    assert_eq!(
        view.selected_index(),
        1,
        "b.txt still sits at index 1 in the refreshed list"
    );
    assert!(
        !view.is_loading(),
        "refresh must not re-open the loading dialog"
    );
    // Deleted-selection fallback: a path that vanished falls back to row 0.
    view.set_selected_index(2); // d.txt
    view.refresh_files_preserving_selection(vec![
        changed_file("a.txt", false),
        changed_file("b.txt", false),
    ]);
    assert_eq!(
        view.selected_path().as_deref(),
        Some("a.txt"),
        "a deleted selection falls back to the first row"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Refreshing a loaded checkpoints view keeps the selection
// stable by work-unit and name
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn refreshing_a_loaded_checkpoints_view_keeps_the_selection_stable_by_work_unit_and_name() {
    // @step Given a loaded checkpoints view with checkpoint AUTH-001/alpha selected
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![
        checkpoint("AUTH-001", "alpha"),
        checkpoint("AUTH-002", "beta"),
    ]);
    view.load.mark_list_flushed();
    view.set_selected_checkpoint(0);
    assert_eq!(
        view.selected_checkpoint_info().map(|c| c.name.clone()),
        Some("alpha".to_string())
    );

    // @step When a git-state refresh lands a checkpoint list that still contains AUTH-001/alpha
    view.refresh_checkpoints_preserving_selection(vec![
        checkpoint("AUTH-002", "beta"),
        checkpoint("AUTH-001", "alpha"),
    ]);

    // @step Then AUTH-001/alpha remains selected (re-selected by work-unit + name) and its files and diff reload
    assert_eq!(
        view.selected_checkpoint_info()
            .map(|c| (c.work_unit_id.clone(), c.name.clone())),
        Some(("AUTH-001".to_string(), "alpha".to_string())),
        "selection must survive the refresh by (work_unit_id, name)"
    );
    assert_eq!(view.selected_checkpoint(), 1);
    assert!(
        !view.is_loading(),
        "refresh must not re-open the loading dialog"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The redraw gate stays open while a rendered mux lazy pane is
// loading
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_redraw_gate_stays_open_while_a_rendered_mux_lazy_pane_is_loading() {
    // @step Given a Navigator in ViewMode::Mux whose rendered ChangedFiles pane has a LoadTracker stage in flight and no other view is loading
    let theme = Arc::new(codelet_fspec_tui::Theme::default());
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut nav = Navigator::new(theme, tx);
    nav.mux.set_pane_list(
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
        ],
        None,
    );
    // `enable_default` would clobber the pane list back to the 2-pane
    // preset — set the enabled flag directly instead.
    nav.mux.config_mut().enabled = true;
    nav.active_view = ViewMode::Mux;
    // The ChangedFiles pane: list flushed, a diff stage in flight.
    nav.changed_files.load.mark_list_flushed();
    nav.changed_files.load.begin_stage(
        &codelet_fspec_tui::components::load_state::LoadTracker::diff_stage_key_path("a.txt"),
        "…",
    );
    // The Checkpoints pane: settled (no stage).
    nav.checkpoints.load.mark_list_flushed();
    assert!(
        nav.checkpoints.load.is_loaded() && !nav.checkpoints.load.is_loading(),
        "the Checkpoints pane must be settled"
    );

    // @step When Navigator::is_view_loading is queried
    let loading = nav.is_view_loading();

    // @step Then it returns true so the 16ms tick keeps drawing and the loading dialog braille spinner does not freeze in mux mode
    assert!(
        loading,
        "is_view_loading must be mux-aware: a rendered loading pane keeps the gate open"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A git-state refresh of a view still in its initial load is
// dropped
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_git_state_refresh_of_a_view_still_in_its_initial_load_is_dropped() {
    // @step Given an App whose changed-files view is still in its initial load (the list stage has not flushed)
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);
    // The view's initial load is in flight (MockBackend default: it has
    // already delivered the empty list → flush it so the App is at rest,
    // then re-arm the initial load by re-opening the view).
    app.dispatch(Action::OpenChangedFilesView);
    assert!(
        app.navigator().changed_files.load.is_loading(),
        "the re-opened view is in its initial load"
    );
    let changed_calls_before = mock.changed_files_calls();

    // @step When a git-state frame arrives while the initial load is in flight
    app.dispatch(Action::GitStateChanged(GitState {
        checkpoint_counts: CheckpointCounts { manual: 0, auto: 0 },
        git_branch: Some("main".to_string()),
        changed_files: vec![changed_file("new.txt", false)],
        checkpoints: Vec::new(),
    }));

    // @step Then the refresh is dropped (no double-start of the initial load) and the in-flight initial load completes un-impeded
    assert_eq!(
        mock.changed_files_calls(),
        changed_calls_before,
        "a refresh must not double-start the initial load (no extra changed_files RPC)"
    );
    // The original in-flight load flushes un-impeded.
    wait_for(
        &mut app,
        |a| !a.navigator().changed_files.load.is_loading(),
        Duration::from_millis(500),
    )
    .await;
    assert!(
        app.navigator().changed_files.load.is_loaded(),
        "the initial load completes un-impeded"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Regression guard: git-state frames re-enter the single-writer paths
// (R3) — counts via CheckpointCountsLoaded, branch via WorkspaceInfoLoaded.
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn git_state_frames_fold_counts_via_the_existing_checkpoint_counts_loaded_path() {
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoint_counts(CheckpointCounts { manual: 5, auto: 2 });
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);
    // Bootstrap's RPC-015 poll left the store at (5, 2); re-stale it.
    app.board_store_mut()
        .set_checkpoint_counts(CheckpointCounts { manual: 0, auto: 0 });

    // A git-state frame carrying fresh counts must land via the
    // CheckpointCountsLoaded fold (no second writer path).
    mock.push_git_state_changed(GitState {
        checkpoint_counts: CheckpointCounts { manual: 7, auto: 3 },
        git_branch: None,
        changed_files: Vec::new(),
        checkpoints: Vec::new(),
    });
    wait_for(
        &mut app,
        |a| a.board_store().checkpoint_counts() == CheckpointCounts { manual: 7, auto: 3 },
        Duration::from_millis(500),
    )
    .await;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 7, auto: 3 }
    );
}

#[tokio::test]
async fn unchanged_git_branch_frames_do_not_resend_workspace_info_loaded() {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);

    let mut branch_frames = 0;
    mock.push_git_state_changed(GitState {
        checkpoint_counts: CheckpointCounts { manual: 1, auto: 0 },
        git_branch: Some("main".to_string()),
        changed_files: Vec::new(),
        checkpoints: Vec::new(),
    });
    wait_for(
        &mut app,
        |a| {
            a.agent_view_store()
                .workspace()
                .is_some_and(|w| w.git_branch.as_deref() == Some("main"))
        },
        Duration::from_millis(500),
    )
    .await;
    branch_frames += 1;
    // Same branch again: the store must already hold it (dedup — no new
    // WorkspaceInfoLoaded dispatch can clobber the cwd string).
    mock.push_git_state_changed(GitState {
        checkpoint_counts: CheckpointCounts { manual: 2, auto: 0 },
        git_branch: Some("main".to_string()),
        changed_files: Vec::new(),
        checkpoints: Vec::new(),
    });
    wait_for(
        &mut app,
        |a| a.board_store().checkpoint_counts() == CheckpointCounts { manual: 2, auto: 0 },
        Duration::from_millis(500),
    )
    .await;
    let info = app.agent_view_store().workspace().expect("workspace set");
    assert_eq!(info.git_branch.as_deref(), Some("main"));
    assert_eq!(
        branch_frames, 1,
        "the branch arrived once; unchanged frames must not re-send"
    );
    assert_eq!(
        info.cwd,
        WorkspaceInfo::default().cwd,
        "cwd survives unchanged frames"
    );
}
