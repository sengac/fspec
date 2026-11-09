//! BUG-184 — mux Files/Checkpoints 10s refresh: no-op ticks dropped
//! before any re-fetch, scroll/selection preserved on change-bearing
//! refreshes, cascade same-key reloads keep the dependent panes' scroll.
//!
//! Feature: spec/features/git-state-refresh-stability.feature
//!
//! The 1:1 test file for the feature (7 scenarios):
//!   - Steady-state tick (files / checkpoints): an identical GitState
//!     frame is dropped before the re-fetch — RPC counter unchanged, no
//!     dialog flash, scroll + selection untouched;
//!   - Change-bearing refresh: the list scroll offset is preserved
//!     (clamped to the fresh list), the selection survives by
//!     path / (work-unit, name), and a fallback snaps the selection
//!     onto the preserved window;
//!   - Cascade same-key reload: a re-fetch of the (re-)selected
//!     checkpoint's files + diff by the same key keeps the dependent
//!     panes' scroll/selection; a key change resets them.
//!
//! Uses the MockBackend's scripted lazy-view payloads (BUG-184) so a
//! LOADED pane can be driven end-to-end at the App level.

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use codelet_fspec_tui::views::{ChangedFilesView, CheckpointsView};
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{ChangedFile, CheckpointCounts, CheckpointInfo, GitState};

mod common;
use common::MockBackend;
use common::TaskOutcome;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn cf(path: &str, staged: bool) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        change_type: "M".to_string(),
        staged,
    }
}

fn ci(work_unit_id: &str, name: &str, is_automatic: bool) -> CheckpointInfo {
    CheckpointInfo {
        work_unit_id: work_unit_id.to_string(),
        name: name.to_string(),
        timestamp: "2026-01-01T00:00:00.000Z".to_string(),
        is_automatic,
    }
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    })
}

/// A `n`-line diff body.
fn diff_body(n: usize) -> String {
    (0..n)
        .map(|i| format!("diff line {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// TUI-131: widths at which the Changed Files / Checkpoints footer hint
/// fits on one row, so the list windows these tests reason about keep
/// their size (a narrower view wraps the footer onto a second row).
const CF_WIDTH: u16 = 110;
const CP_WIDTH: u16 = 150;

/// Paint the ChangedFiles view into a `(w x h)` test grid.
fn render_cf(
    view: &mut ChangedFilesView,
    w: u16,
    h: u16,
) -> Result<(), codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(w, h)).or_fail("term")?;
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .or_fail("draw")?;
    Ok(())
}

/// Paint the Checkpoints view into a `(w x h)` test grid.
fn render_cp(
    view: &mut CheckpointsView,
    w: u16,
    h: u16,
) -> Result<(), codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(w, h)).or_fail("term")?;
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .or_fail("draw")?;
    Ok(())
}

/// BUG-657: bootstrap loads the checkpoint counts and the workspace info in
/// a tracked background task; await the bootstrap's tasks, then fold what
/// they sent.
async fn settle_bootstrap(app: &mut App) -> TestResult {
    while let Some(task) = app.next_pending_task() {
        codelet_test_helpers::deadlock_guard("a bootstrap task", task)
            .await?
            .completed_or_cancelled()?;
    }
    drain_bus(app);
    Ok(())
}

fn drain_bus(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

async fn wait_for<F: Fn(&App) -> bool>(
    app: &mut App,
    pred: F,
) -> Result<(), codelet_test_helpers::TestError> {
    // TEST-042: wait under the deadlock guard, not a deadline.
    codelet_test_helpers::deadlock_guard("the awaited App state", async {
        loop {
            drain_bus(app);
            if pred(app) {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

fn thirty_files() -> Vec<ChangedFile> {
    (0..30)
        .map(|i| cf(&format!("file{i:02}.txt"), false))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Steady-state tick: a GitState frame matching the displayed
// files list is dropped before any re-fetch
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn steady_state_frame_matching_the_displayed_files_list_is_dropped_before_any_refetch(
) -> TestResult {
    // @step Given a mux layout with a loaded, displayed Changed Files pane displaying 30 files with file04.txt selected and the file list scrolled past the top
    let mock = Arc::new(MockBackend::new());
    mock.set_changed_files(thirty_files());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    settle_bootstrap(&mut app).await?;
    app.dispatch(Action::OpenChangedFilesView);
    wait_for(&mut app, |a| !a.navigator().changed_files.load.is_loading()).await?;
    assert_eq!(
        app.navigator().changed_files.files_len(),
        30,
        "the pane displays the scripted list"
    );
    app.navigator_mut().changed_files.set_selected_index(4);
    assert_eq!(
        app.navigator().changed_files.selected_path().as_deref(),
        Some("file04.txt"),
    );
    let calls_before = mock.changed_files_calls();
    let scroll_before = app.navigator().changed_files.file_scroll();

    // @step When a GitState frame arrives whose changed_files leg is identical to the displayed list
    app.dispatch(Action::GitStateChanged(GitState {
        checkpoint_counts: CheckpointCounts::default(),
        git_branch: Some("main".to_string()),
        changed_files: thirty_files(),
        checkpoints: Vec::new(),
    }));
    drain_bus(&mut app);

    // @step Then no changed_files re-fetch is spawned
    assert_eq!(
        mock.changed_files_calls(),
        calls_before,
        "an identical frame must be dropped BEFORE the re-fetch (no extra changed_files RPC)"
    );
    // @step And file04.txt remains selected
    assert_eq!(
        app.navigator().changed_files.selected_path().as_deref(),
        Some("file04.txt"),
        "the selection survives the no-op tick"
    );
    // @step And the pane does not flash the loading dialog
    assert!(
        !app.navigator().changed_files.load.is_loading(),
        "a dropped no-op tick must not re-open the loading stage"
    );
    assert!(
        !app.is_view_loading(),
        "the redraw gate stays idle after a dropped tick"
    );
    // @step And the file list scroll offset is unchanged
    assert_eq!(
        app.navigator().changed_files.file_scroll(),
        scroll_before,
        "a dropped no-op tick must not touch the scroll offset"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Steady-state tick: a GitState frame matching the displayed
// checkpoints list is dropped before any re-fetch
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn steady_state_frame_matching_the_displayed_checkpoints_list_is_dropped_before_any_refetch(
) -> TestResult {
    // @step Given a mux layout with a loaded, displayed Checkpoints pane displaying 5 checkpoints with the list scrolled past the top
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoints(vec![
        ci("BUG-184", "checkpoint-a", true),
        ci("BUG-184", "checkpoint-b", false),
        ci("BUG-184", "checkpoint-c", true),
        ci("BUG-184", "checkpoint-d", false),
        ci("BUG-184", "checkpoint-e", true),
    ]);
    mock.set_checkpoint_files("BUG-184", "checkpoint-a", vec![cf("a.txt", false)]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    settle_bootstrap(&mut app).await?;
    app.dispatch(Action::OpenCheckpointsView);
    wait_for(&mut app, |a| !a.navigator().checkpoints.load.is_loading()).await?;
    assert_eq!(
        app.navigator().checkpoints.checkpoints_len(),
        5,
        "the pane displays the scripted list"
    );
    let list_calls_before = mock.list_checkpoints_calls();
    let scroll_before = app.navigator().checkpoints.checkpoint_scroll();

    // @step When a GitState frame arrives whose checkpoints leg renders identically to the displayed list
    app.dispatch(Action::GitStateChanged(GitState {
        checkpoint_counts: CheckpointCounts::default(),
        git_branch: Some("main".to_string()),
        changed_files: Vec::new(),
        checkpoints: vec![
            ci("BUG-184", "checkpoint-a", true),
            ci("BUG-184", "checkpoint-b", false),
            ci("BUG-184", "checkpoint-c", true),
            ci("BUG-184", "checkpoint-d", false),
            ci("BUG-184", "checkpoint-e", true),
        ],
    }));
    drain_bus(&mut app);

    // @step Then no list_checkpoints re-fetch is spawned
    assert_eq!(
        mock.list_checkpoints_calls(),
        list_calls_before,
        "an identical frame must be dropped BEFORE the re-fetch (no extra list_checkpoints RPC)"
    );
    // @step And the checkpoint list scroll offset is unchanged
    assert_eq!(
        app.navigator().checkpoints.checkpoint_scroll(),
        scroll_before,
        "a dropped no-op tick must not touch the scroll offset"
    );
    // @step And the checkpoint selection remains stable
    assert_eq!(
        app.navigator()
            .checkpoints
            .selected_checkpoint_info()
            .map(|c| c.name.clone()),
        Some("checkpoint-a".to_string()),
    );
    // @step And the pane does not flash the loading dialog
    assert!(
        !app.navigator().checkpoints.load.is_loading(),
        "a dropped no-op tick must not re-open the loading stage"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Change-bearing refresh: files committed above the window
// keep the scroll point
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn refresh_keeps_the_file_list_scroll_offset_when_files_above_the_window_disappear() -> TestResult {
    // @step Given a loaded Changed Files pane displaying 30 files with "file04.txt" selected and the list scrolled past the top
    let mut view = ChangedFilesView::new();
    view.set_files(thirty_files());
    render_cf(&mut view, CF_WIDTH, 8)?;
    for _ in 0..4 {
        let _ = view.handle_event(&key(KeyCode::Down));
    }
    render_cf(&mut view, CF_WIDTH, 8)?;
    assert_eq!(view.selected_path().as_deref(), Some("file04.txt"));
    let scroll = view.file_scroll();
    assert!(scroll > 0, "the list is scrolled");

    // @step When a change-bearing git-state refresh lands a list in which file00.txt and file01.txt were committed
    view.refresh_files_preserving_selection(
        (2..30)
            .map(|i| cf(&format!("file{i:02}.txt"), false))
            .collect(),
    );

    // @step Then the file04.txt selection survives by path
    assert_eq!(
        view.selected_path().as_deref(),
        Some("file04.txt"),
        "the selection survives by path"
    );
    // @step And the file list scroll offset keeps its point (it does not jump to the top)
    assert_eq!(
        view.file_scroll(),
        scroll,
        "the scroll offset is the list point the user keeps"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Change-bearing refresh: a list shorter than the window
// falls back to the top
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn refresh_falls_back_to_the_top_when_the_list_shrinks_beneath_the_window() -> TestResult {
    // @step Given a loaded Changed Files pane displaying 30 files scrolled to the bottom with "file29.txt" selected
    let mut view = ChangedFilesView::new();
    view.set_files(thirty_files());
    render_cf(&mut view, CF_WIDTH, 8)?;
    for _ in 0..40 {
        let _ = view.handle_event(&key(KeyCode::Down));
    }
    render_cf(&mut view, CF_WIDTH, 8)?;
    assert_eq!(view.selected_path().as_deref(), Some("file29.txt"));
    assert!(view.file_scroll() > 0, "the bottom window is scrolled");

    // @step When a change-bearing git-state refresh lands a list of 3 files (shorter than the visible window) that does not contain the selection
    view.refresh_files_preserving_selection(vec![
        cf("file25.txt", false),
        cf("file26.txt", false),
        cf("file27.txt", false),
    ]);

    // @step Then the file list window falls back to the top
    assert_eq!(
        view.file_scroll(),
        0,
        "a list shorter than the window starts at the top"
    );
    // @step And the selection lands on the first row (inside the visible window)
    assert_eq!(view.selected_path().as_deref(), Some("file25.txt"));
    assert!(
        view.selected_index() < 3,
        "selection must lie inside the window"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Change-bearing refresh: a committed selection keeps the
// window and lands on the first visible row
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn refresh_clamps_the_file_list_scroll_when_the_selected_file_is_committed() -> TestResult {
    // @step Given a loaded Changed Files pane displaying 30 files scrolled to the bottom with "file29.txt" selected
    let mut view = ChangedFilesView::new();
    view.set_files(thirty_files());
    render_cf(&mut view, CF_WIDTH, 8)?;
    for _ in 0..40 {
        let _ = view.handle_event(&key(KeyCode::Down));
    }
    render_cf(&mut view, CF_WIDTH, 8)?;
    assert_eq!(view.selected_path().as_deref(), Some("file29.txt"));

    // @step When a change-bearing git-state refresh lands a list in which file29.txt was committed (it no longer exists)
    view.refresh_files_preserving_selection(
        (0..29)
            .map(|i| cf(&format!("file{i:02}.txt"), false))
            .collect(),
    );

    // @step Then the selection moves to the first visible row of the preserved window
    // (the deleted selection falls back to row 0, which sits above the
    // bottom window — the anchor rule snaps it to the window's first
    // visible row)
    assert_eq!(
        view.selected_path().as_deref(),
        Some("file26.txt"),
        "a deleted selection lands on the first visible row (highlighted row and window stay together)"
    );
    // @step And the window keeps its point (clamped to the last full page of the fresh list)
    assert_eq!(
        view.file_scroll(),
        26,
        "the window keeps its point (clamped to the last full page)"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Change-bearing refresh: a committed checkpoint keeps the
// window and lands on the first visible row
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn refresh_clamps_the_checkpoint_list_scroll_when_the_selected_checkpoint_is_committed(
) -> TestResult {
    // @step Given a loaded Checkpoints pane displaying 30 checkpoints scrolled to the bottom with checkpoint-29 selected
    let mut view = CheckpointsView::new();
    view.set_checkpoints(
        (0..30)
            .map(|i| ci("BUG-184", &format!("checkpoint-{i:02}"), i % 2 == 0))
            .collect(),
    );
    render_cp(&mut view, CP_WIDTH, 30)?;
    for _ in 0..40 {
        let _ = view.handle_event(&key(KeyCode::Down));
    }
    render_cp(&mut view, CP_WIDTH, 30)?;
    assert_eq!(
        view.selected_checkpoint_info().map(|c| c.name.clone()),
        Some("checkpoint-29".to_string())
    );

    // @step When a change-bearing git-state refresh lands a list in which checkpoint-29 was removed (it no longer exists)
    view.refresh_checkpoints_preserving_selection(
        (0..29)
            .map(|i| ci("BUG-184", &format!("checkpoint-{i:02}"), i % 2 == 0))
            .collect(),
    );

    // @step Then the selection moves to the first visible row of the preserved window
    // (a removed selection falls back to row 0, which sits above the
    // bottom window — the anchor rule snaps it to the window's first
    // row)
    let scroll = view.checkpoint_scroll();
    assert_eq!(
        view.selected_checkpoint(),
        scroll,
        "the fallback lands on the first visible row (highlighted row and window stay together)"
    );
    // @step And the window keeps its point (clamped to the last full page of the fresh list)
    assert_eq!(
        scroll, 20,
        "the window clamps to the last full page of the fresh list"
    );
    assert_eq!(
        view.selected_checkpoint_info().map(|c| c.name.clone()),
        Some("checkpoint-20".to_string())
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cascade same-key reload: the checkpoint Files and Diff panes
// keep their scroll
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn cascade_same_key_reload_keeps_the_files_and_diff_panes_scrolled() -> TestResult {
    // @step Given a loaded Checkpoints pane with a checkpoint selected whose files list and diff are displayed and scrolled
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoints(vec![ci("BUG-184", "checkpoint-a", true)]);
    mock.set_checkpoint_files(
        "BUG-184",
        "checkpoint-a",
        vec![cf("a.txt", false), cf("b.txt", false)],
    );
    mock.set_checkpoint_file_diff("BUG-184", "checkpoint-a", "a.txt", Some(diff_body(30)));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    settle_bootstrap(&mut app).await?;
    app.dispatch(Action::OpenCheckpointsView);
    wait_for(&mut app, |a| !a.navigator().checkpoints.load.is_loading()).await?;
    assert!(
        app.navigator().checkpoints.load.is_loaded(),
        "the cascade (list → files → diff) has flushed"
    );
    assert_eq!(
        app.navigator()
            .checkpoints
            .selected_checkpoint_info()
            .map(|c| (c.work_unit_id.clone(), c.name.clone())),
        Some(("BUG-184".to_string(), "checkpoint-a".to_string())),
    );

    // @step When a change-bearing git-state refresh re-fetches the same checkpoint's files and diff (same work-unit + name + path)
    // The frame's checkpoints leg DIFFERS from the displayed list
    // (two checkpoints vs one) → change-bearing refresh → the re-fetch
    // runs, the (re-)selection stays on BUG-184/checkpoint-a by key, and
    // the cascade re-loads the SAME key's files + diff.
    app.dispatch(Action::GitStateChanged(GitState {
        checkpoint_counts: CheckpointCounts::default(),
        git_branch: Some("main".to_string()),
        changed_files: Vec::new(),
        checkpoints: vec![
            ci("BUG-184", "checkpoint-a", true),
            ci("BUG-184", "checkpoint-b", false),
        ],
    }));
    wait_for(&mut app, |a| !a.navigator().checkpoints.load.is_loading()).await?;

    // @step Then the Files pane keeps its scroll offset and selection
    assert_eq!(
        app.navigator().checkpoints.selected_file(),
        0,
        "the (re-)selected checkpoint's first file stays selected (same key)"
    );
    assert_eq!(
        app.navigator()
            .checkpoints
            .selected_checkpoint_info()
            .map(|c| c.name.clone()),
        Some("checkpoint-a".to_string()),
        "the selection survives the refresh by work-unit + name"
    );
    // @step And the Diff pane keeps its scroll offset (clamped to the fresh diff length)
    assert_eq!(
        app.navigator().checkpoints.diff_scroll(),
        0,
        "the diff pane stays at the top (same-key re-load — no jump)"
    );
    assert!(
        app.navigator().checkpoints.load.is_loaded(),
        "the refreshed cascade settles without re-opening the dialog"
    );
    Ok(())
}

#[test]
fn cascade_same_key_reload_keeps_the_scrolled_files_and_diff_pane_points() -> TestResult {
    // @step Given a loaded Checkpoints pane with a checkpoint selected whose files list and diff are displayed and scrolled
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("BUG-184", "checkpoint-a", true)]);
    view.set_files(
        "BUG-184",
        "checkpoint-a",
        (0..20)
            .map(|i| cf(&format!("file{i:02}.txt"), false))
            .collect(),
    );
    view.set_diff("BUG-184", "checkpoint-a", "file00.txt", Some(diff_body(40)));
    render_cp(&mut view, 120, 30)?;
    // Files pane: move the selection down 12 rows (focus: Files pane;
    // 9 visible rows, so the scroll kicks in past row 8).
    let _ = view.handle_event(&key(KeyCode::Tab)); // Checkpoints → Files
    for _ in 0..12 {
        let _ = view.handle_event(&key(KeyCode::Down));
    }
    render_cp(&mut view, 120, 30)?;
    assert_eq!(view.selected_file(), 12, "the files pane selection moved");
    assert!(view.file_scroll() > 0, "the files pane is scrolled");
    let file_scroll_before = view.file_scroll();

    // @step When a change-bearing git-state refresh re-fetches the same checkpoint's files and diff (same work-unit + name + path)
    view.set_files(
        "BUG-184",
        "checkpoint-a",
        (0..20)
            .map(|i| cf(&format!("file{i:02}.txt"), false))
            .collect(),
    );
    view.set_diff("BUG-184", "checkpoint-a", "file00.txt", Some(diff_body(40)));

    // @step Then the Files pane keeps its scroll offset and selection
    assert_eq!(
        view.file_scroll(),
        file_scroll_before,
        "the Files pane scroll survives the same-key reload"
    );
    assert_eq!(
        view.selected_file(),
        12,
        "the Files pane selection survives the same-key reload"
    );

    // @step And the Diff pane keeps its scroll offset (clamped to the fresh diff length)
    // Move the file selection back to file00 (the diff document on
    // display; focus is still the Files pane) and scroll that diff.
    for _ in 0..12 {
        let _ = view.handle_event(&key(KeyCode::Up));
    }
    render_cp(&mut view, 120, 30)?;
    assert_eq!(view.selected_file(), 0, "back on file00.txt");
    view.set_diff("BUG-184", "checkpoint-a", "file00.txt", Some(diff_body(40)));
    let _ = view.handle_event(&key(KeyCode::Tab)); // Files → Diff
    for _ in 0..20 {
        let _ = view.handle_event(&key(KeyCode::Down));
    }
    render_cp(&mut view, 120, 30)?;
    let diff_scroll_before = view.diff_scroll();
    assert!(diff_scroll_before > 0, "the diff pane is scrolled");
    // A same-key re-load of a LONGER diff keeps the offset verbatim…
    view.set_diff("BUG-184", "checkpoint-a", "file00.txt", Some(diff_body(50)));
    assert_eq!(
        view.diff_scroll(),
        diff_scroll_before,
        "the Diff pane scroll survives the same-key reload"
    );
    // …and a same-key re-load of a SHORTER diff clamps to the fresh
    // length (10 lines fit the 14-row viewport → no overflow → top).
    view.set_diff("BUG-184", "checkpoint-a", "file00.txt", Some(diff_body(10)));
    assert_eq!(
        view.diff_scroll(),
        0,
        "the Diff pane scroll clamps to the fresh diff length"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cascade key change: selecting a different checkpoint
// resets the dependent panes
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cascade_key_change_resets_the_files_and_diff_panes_to_the_top() -> TestResult {
    // @step Given a loaded Checkpoints pane with a checkpoint's files and diff displayed and scrolled
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![
        ci("BUG-184", "checkpoint-a", true),
        ci("BUG-184", "checkpoint-b", false),
    ]);
    view.set_files(
        "BUG-184",
        "checkpoint-a",
        (0..20)
            .map(|i| cf(&format!("file{i:02}.txt"), false))
            .collect(),
    );
    view.set_diff("BUG-184", "checkpoint-a", "file00.txt", Some(diff_body(40)));
    render_cp(&mut view, 120, 30)?;
    let _ = view.handle_event(&key(KeyCode::Tab)); // Checkpoints → Files
    for _ in 0..12 {
        let _ = view.handle_event(&key(KeyCode::Down));
    }
    let _ = view.handle_event(&key(KeyCode::Tab)); // Files → Diff
    for _ in 0..20 {
        let _ = view.handle_event(&key(KeyCode::Down));
    }
    render_cp(&mut view, 120, 30)?;
    assert!(view.file_scroll() > 0, "the Files pane is scrolled");
    assert!(view.diff_scroll() > 0, "the Diff pane is scrolled");

    // @step When a refresh re-selects a DIFFERENT checkpoint (different work-unit + name)
    view.set_selected_checkpoint(1); // the refresh re-lookup lands on checkpoint-b
    view.set_files(
        "BUG-184",
        "checkpoint-b",
        (0..10)
            .map(|i| cf(&format!("file{i:02}.txt"), false))
            .collect(),
    );
    view.set_diff("BUG-184", "checkpoint-b", "file00.txt", Some(diff_body(30)));

    // @step Then the Files pane resets to the first file at the top
    assert_eq!(
        view.selected_file(),
        0,
        "a key change resets the Files pane selection"
    );
    assert_eq!(
        view.file_scroll(),
        0,
        "a key change resets the Files pane scroll"
    );
    // @step And the Diff pane resets to the top
    assert_eq!(
        view.diff_scroll(),
        0,
        "a key change resets the Diff pane scroll"
    );
    Ok(())
}
