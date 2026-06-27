//! RPC-354 — umbrella end-to-end acceptance test for the Rust TUI
//! "Changed Files" view.
//!
//! Feature: spec/features/rust-file-changes-view.feature
//!
//! Drives the REAL integrated stack headlessly: a real temp git repo +
//! real gitoxide via `EmbeddedFspecBackend`, the real `BoardView` F-key
//! emission, the real `Navigator` view flip, and the real
//! `ChangedFilesView` render. Reproduces by hand the App loop that
//! normally drains the action bus and spawns the backend loads
//! (mirrors `app/dispatch_changed_files.rs`). Integration-first: no
//! mocking — children RPC-355 (backend) and RPC-356 (UI) are already
//! implemented, so these assertions pass GREEN, as expected for an
//! umbrella acceptance test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::store::{AgentViewStore, BoardStore};
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::navigator::{Navigator, ViewMode};
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend};
use codelet_rpc::SharedFspecService;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;
use tokio::sync::mpsc::unbounded_channel;

/// Initialize a repo with a single committed-then-modified file (one
/// modified entry in the working tree). Copied from
/// `tests/changed_files_rpc355.rs` to reuse the proven seeding flow.
fn seed_repo_one_modified() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);
    fs::write(repo.join("tracked.txt"), "alpha\n").expect("write tracked");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo)
        .output()
        .expect("git commit");
    fs::write(repo.join("tracked.txt"), "alpha\nbeta\n").expect("modify tracked");
    tmp
}

/// Initialize a repo that is committed and CLEAN (no working-tree
/// changes) — used for the empty-state scenario.
fn seed_repo_clean() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);
    fs::write(repo.join("tracked.txt"), "alpha\n").expect("write tracked");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo)
        .output()
        .expect("git commit");
    tmp
}

fn git_init(repo: &Path) {
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test"],
    ] {
        Command::new("git")
            .args(&args)
            .current_dir(repo)
            .output()
            .expect("git setup");
    }
}

fn service_for(repo_path: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).expect("WorkUnitsWatcher::new"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()))
}

fn backend_for(repo: &Path) -> Arc<dyn FspecBackend> {
    let service = service_for(repo);
    Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ))
}

/// Build a key-press event (no modifiers).
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

/// Render the navigator into a `TestBackend` buffer and return the
/// joined cell text. Mirrors the App's frame render path.
fn render_navigator_text(nav: &mut Navigator, board: &BoardStore) -> String {
    let mut agent = AgentViewStore::default();
    let mut term = Terminal::new(TestBackend::new(120, 30)).expect("term");
    term.draw(|frame| {
        nav.render_with_stores(frame.area(), frame.buffer_mut(), board, &mut agent);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    joined
}

/// Mirror the App loop's load flow (app/dispatch_changed_files.rs):
/// fetch the changed files, fold them into the view, then fetch + fold
/// the diff for the now-selected file.
async fn drive_load(nav: &mut Navigator, backend: &Arc<dyn FspecBackend>) {
    let files = backend.changed_files().await.expect("changed_files");
    nav.changed_files.set_files(files);
    if let Some(path) = nav.changed_files.selected_path() {
        let diff = backend.file_diff(path.clone()).await.expect("file_diff");
        nav.changed_files.set_diff(&path, diff);
    }
}

/// Scenario: Pressing F on the board opens the Changed Files view with
/// the changed file and its diff.
#[tokio::test]
async fn pressing_f_opens_changed_files_view_with_file_and_diff() {
    // @step Given a workspace whose git working tree has one modified file
    let tmp = seed_repo_one_modified();
    let backend = backend_for(tmp.path());

    // @step And the navigator is showing the board view
    let (tx, mut rx) = unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    let board_store = BoardStore::default();
    assert_eq!(nav.active_view, ViewMode::Board);

    // @step When the user presses the "F" key
    nav.handle_event(&key(KeyCode::Char('F')), &board_store);
    while let Ok(action) = rx.try_recv() {
        nav.apply_action(&action);
    }

    // @step Then the active view becomes the Changed Files view
    assert_eq!(nav.active_view, ViewMode::ChangedFiles);
    drive_load(&mut nav, &backend).await;

    // @step And the file list contains the modified file
    assert_eq!(
        nav.changed_files.selected_path().as_deref(),
        Some("tracked.txt")
    );
    let listed = render_navigator_text(&mut nav, &board_store);
    assert!(
        listed.contains("tracked.txt"),
        "rendered Changed Files view must list the modified file"
    );

    // @step And the diff pane shows the unified diff for the modified file
    let rendered = render_navigator_text(&mut nav, &board_store);
    assert!(
        rendered.contains("beta"),
        "rendered diff pane must show the added line from the unified diff"
    );
}

/// Scenario: Pressing Esc in the Changed Files view returns to the
/// board.
#[tokio::test]
async fn pressing_esc_in_changed_files_view_returns_to_board() {
    let tmp = seed_repo_one_modified();
    let backend = backend_for(tmp.path());
    let (tx, mut rx) = unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    let board_store = BoardStore::default();

    // Reach the Changed Files view via the real F-key path first.
    nav.handle_event(&key(KeyCode::Char('F')), &board_store);
    while let Ok(action) = rx.try_recv() {
        nav.apply_action(&action);
    }
    drive_load(&mut nav, &backend).await;

    // @step Given the navigator is showing the Changed Files view
    assert_eq!(nav.active_view, ViewMode::ChangedFiles);

    // @step When the user presses the "Esc" key
    nav.handle_event(&key(KeyCode::Esc), &board_store);
    while let Ok(action) = rx.try_recv() {
        nav.apply_action(&action);
    }

    // @step Then the active view becomes the board view
    assert_eq!(nav.active_view, ViewMode::Board);
}

/// Scenario: Opening the Changed Files view in a clean repo shows an
/// empty state.
#[tokio::test]
async fn opening_changed_files_view_in_clean_repo_shows_empty_state() {
    // @step Given a workspace whose git working tree has no changes
    let tmp = seed_repo_clean();
    let backend = backend_for(tmp.path());

    // @step And the navigator is showing the board view
    let (tx, mut rx) = unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    let board_store = BoardStore::default();
    assert_eq!(nav.active_view, ViewMode::Board);

    // @step When the user presses the "F" key
    nav.handle_event(&key(KeyCode::Char('F')), &board_store);
    while let Ok(action) = rx.try_recv() {
        nav.apply_action(&action);
    }

    // @step Then the active view becomes the Changed Files view
    assert_eq!(nav.active_view, ViewMode::ChangedFiles);
    let files = backend.changed_files().await.expect("changed_files");
    assert!(files.is_empty(), "clean repo must report no changed files");
    nav.changed_files.set_files(files);

    // @step And the view shows an empty-state message that there are no changed files
    assert!(nav.changed_files.is_empty());
    let rendered = render_navigator_text(&mut nav, &board_store);
    assert!(
        rendered.contains("No changed files"),
        "rendered clean-repo view must show the empty-state message"
    );

    // @step When the user presses the "Esc" key
    nav.handle_event(&key(KeyCode::Esc), &board_store);
    while let Ok(action) = rx.try_recv() {
        nav.apply_action(&action);
    }
    // @step Then the active view becomes the board view
    assert_eq!(nav.active_view, ViewMode::Board);
}
