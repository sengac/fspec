//! TUI-131 — long text wraps instead of being cut off.
//!
//! Feature: spec/features/long-text-wraps-instead-of-being-cut-off.feature
//!
//! Every test drives the real `App` over a `MockBackend` (stream chunks,
//! actions the backend answers with, the user's keys and pastes) and reads
//! the frames `App::render` paints at 80x24 and 120x40. A text is "on
//! screen whole" when it can be read from consecutive rows (see
//! `tui131_long_text_wraps/screen.rs`). The dialog scenarios are here; the
//! pane, status-row and footer scenarios are in `tui131_long_text_wraps/`.
//! Time is tokio's paused clock.

use std::sync::{Arc, Mutex};

use codelet_fspec_tui::daemons::{DaemonChoice, DaemonForm, TokenChange};
use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::Action;
use codelet_rpc_types::{ChangedFile, CheckpointInfo, SessionWorktreeInfo, StreamChunk};
use codelet_test_helpers::TestResult;
use crossterm::event::KeyCode;

mod common;
mod tui131_long_text_wraps;

use common::MockBackend;
use tui131_long_text_wraps::fixture::*;
use tui131_long_text_wraps::screen::{dialog_shows, Checks, Screen};

/// The ErrorDialog footer.
const ERROR_FOOTER: &str = "c copy · Press ESC to dismiss";

async fn report_api_error(fx: &mut Fixture, error: &str) -> TestResult {
    fx.app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::Error {
            error: error.to_string(),
        },
    ));
    settle(&mut fx.app).await
}

/// Scenario: A long API error with line breaks is shown whole at 80 and at 120 columns
#[tokio::test(start_paused = true)]
async fn a_long_api_error_with_line_breaks_is_shown_whole_at_80_and_at_120_columns() -> TestResult {
    // @step Given the agent view is open
    let mut fx = agent_fixture(Arc::new(MockBackend::new())).await?;
    let mut checks = Checks::default();

    // @step When the stream reports a 300-character API error that contains two line breaks
    // The first line is short, so its line break falls inside the dialog.
    let error = format!(
        "{}\n{}\n{}",
        prose("upstream", 40, "first-end"),
        prose("detail", 150, "second-end"),
        prose("hint", 108, "third-end")
    );
    assert_eq!(error.chars().count(), 300);
    report_api_error(&mut fx, &error).await?;
    let shown = format!("API Error: {error}");

    // @step Then at 80x24 every line of the error is on screen whole, wrapped to the dialog width
    let screen = Screen::of(&mut fx.app, 80, 24);
    for line in shown.lines() {
        checks.check(
            dialog_shows(&screen, ERROR_FOOTER, line),
            &format!("80x24: the error dialog does not show {line:?} whole"),
            &screen,
        );
    }
    // @step And no cell of the screen holds a line break
    checks.check(
        !screen.holds_line_break(),
        "80x24: a cell holds a line break (shown as ⏎)",
        &screen,
    );
    // @step And at 120x40 every line of the error is on screen whole
    let screen = Screen::of(&mut fx.app, 120, 40);
    for line in shown.lines() {
        checks.check(
            dialog_shows(&screen, ERROR_FOOTER, line),
            &format!("120x40: the error dialog does not show {line:?} whole"),
            &screen,
        );
    }
    checks.done()
}

/// Scenario: An API error taller than the screen scrolls and keeps its footer
#[tokio::test(start_paused = true)]
async fn an_api_error_taller_than_the_screen_scrolls_and_keeps_its_footer() -> TestResult {
    // @step Given the agent view is open
    let mut fx = agent_fixture(Arc::new(MockBackend::new())).await?;
    let mut checks = Checks::default();

    // @step When the stream reports a 4000-character API error
    let last = "zebra-final-word";
    let error = prose("body", 4000, last);
    report_api_error(&mut fx, &error).await?;

    // @step Then at 80x24 the error dialog fills the screen and "c copy · Press ESC to dismiss" is on screen
    let screen = Screen::of(&mut fx.app, 80, 24);
    let dialog = screen.dialog(ERROR_FOOTER);
    let height = dialog.as_ref().map_or(0, |dialog| dialog.height);
    checks.check(
        height >= 20,
        &format!("80x24: the error dialog is {height} rows tall, not the screen's 20 or more"),
        &screen,
    );
    checks.check(
        dialog.is_some(),
        "80x24: the error dialog footer is not on screen",
        &screen,
    );
    // @step And the error's last word is not on screen
    checks.check(
        !dialog_shows(&screen, ERROR_FOOTER, last),
        "80x24: the error's last word is already on screen before End",
        &screen,
    );

    // @step When the user presses End
    key(&mut fx, KeyCode::End).await?;

    // @step Then the error's last word is on screen and "c copy · Press ESC to dismiss" is still on screen
    let screen = Screen::of(&mut fx.app, 80, 24);
    checks.check(
        dialog_shows(&screen, ERROR_FOOTER, last),
        "80x24 after End: the error's last word is not in the error dialog",
        &screen,
    );
    checks.check(
        screen.find(ERROR_FOOTER).is_some(),
        "80x24 after End: the error dialog footer is not on screen",
        &screen,
    );
    checks.done()
}

/// Scenario: The daemon switch notification shows its whole sentence
#[tokio::test(start_paused = true)]
async fn the_daemon_switch_notification_shows_its_whole_sentence() -> TestResult {
    // @step Given the board is open
    let name = "build-server-eu-west";
    let mut fx = fixture();
    let daemons = MemoryDaemons(Mutex::new(vec![DaemonChoice {
        name: name.into(),
        url: "wss://build-server-eu-west.example.com".into(),
        saved: true,
    }]));
    fx.app.set_daemon_directory(Arc::new(daemons), name.into());
    let mut checks = Checks::default();

    // @step When the user saves new settings for the connected daemon named "build-server-eu-west"
    // The "…the next time it connects to <name>." notification is raised
    // when the connected daemon's saved settings change.
    fx.app.dispatch(Action::SaveDaemon {
        previous: Some(name.into()),
        form: DaemonForm {
            name: name.into(),
            url: "wss://build-server-eu-west-2.example.com".into(),
            token: TokenChange::Keep,
        },
    });
    settle(&mut fx.app).await?;
    let sentence = format!("the next time it connects to {name}.");

    // @step Then the notification text "the next time it connects to build-server-eu-west." is on screen whole at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);
    checks.check(
        dialog_shows(&screen, "Saved daemon", &sentence),
        "80x24: the notification does not show its last sentence whole",
        &screen,
    );
    // @step And it is on screen whole at 120x40
    let screen = Screen::of(&mut fx.app, 120, 40);
    checks.check(
        dialog_shows(&screen, "Saved daemon", &sentence),
        "120x40: the notification does not show its last sentence whole",
        &screen,
    );
    checks.done()
}

/// Scenario: The checkpoint restore confirmation shows its warning for a long path
#[tokio::test(start_paused = true)]
async fn the_checkpoint_restore_confirmation_shows_its_warning_for_a_long_path() -> TestResult {
    // @step Given the checkpoints view lists a checkpoint with a file whose path is 100 characters long
    let path = token("rust/fspec-tui/src/views/checkpoints/", 97) + ".rs";
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoints(vec![CheckpointInfo {
        work_unit_id: "TUI-131".into(),
        name: "before-wrap".into(),
        timestamp: "2026-11-07T00:00:00.000Z".into(),
        is_automatic: false,
    }]);
    mock.set_checkpoint_files(
        "TUI-131",
        "before-wrap",
        vec![ChangedFile {
            path: path.clone(),
            change_type: "M".into(),
            staged: false,
        }],
    );
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::OpenCheckpointsView);
    settle(&mut fx.app).await?;
    let mut checks = Checks::default();

    // @step When the user presses r on that file
    key(&mut fx, KeyCode::Tab).await?;
    press(&mut fx, 'r').await?;
    let confirmation = format!("Restore {path}? This overwrites the working copy.");

    // @step Then the confirmation "Restore <path>? This overwrites the working copy." is on screen whole at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);
    checks.check(
        dialog_shows(&screen, "Restore Checkpoint", &confirmation),
        "80x24: the restore confirmation is not shown whole",
        &screen,
    );
    // @step And it is on screen whole at 120x40
    let screen = Screen::of(&mut fx.app, 120, 40);
    checks.check(
        dialog_shows(&screen, "Restore Checkpoint", &confirmation),
        "120x40: the restore confirmation is not shown whole",
        &screen,
    );
    checks.done()
}

/// A worktree of a session with a UUID id and a 90-character path.
fn worktree(i: u32) -> SessionWorktreeInfo {
    let id = format!("{i:08x}-0000-4000-8000-{i:012x}");
    SessionWorktreeInfo {
        session_id: sid(&id),
        worktree_path: token(
            &format!("/home/user/projects/fspec/.fspec/worktrees/{id}-"),
            90,
        ),
        base_commit: "abc1234".into(),
        head_commit: "def5678".into(),
        dirty: i.is_multiple_of(2),
    }
}

async fn open_worktrees(
    rows: Vec<SessionWorktreeInfo>,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mock = Arc::new(MockBackend::new());
    mock.seed_session_worktrees(rows);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::SessionCreated(sid("s-1")));
    settle(&mut fx.app).await?;
    let session = fx.app.current_session();
    fx.app.dispatch(Action::SlashCommandSelected(
        session,
        SlashCommandAction::Worktrees,
    ));
    settle(&mut fx.app).await?;
    Ok(fx)
}

const WORKTREES: &str = "Session Worktrees";

/// Scenario: The worktrees dialog shows each full path and keeps its buttons
#[tokio::test(start_paused = true)]
async fn the_worktrees_dialog_shows_each_full_path_and_keeps_its_buttons() -> TestResult {
    let mut checks = Checks::default();
    // @step Given the session has three worktrees whose paths are 90 characters long
    let three: Vec<SessionWorktreeInfo> = (1..=3).map(worktree).collect();

    // @step When the user opens /worktrees at 80x24
    let mut fx = open_worktrees(three.clone()).await?;
    let screen = Screen::of(&mut fx.app, 80, 24);

    // @step Then each worktree's full path is on screen whole
    for row in &three {
        checks.check(
            dialog_shows(&screen, WORKTREES, &row.worktree_path),
            &format!("80x24: {} is not shown whole", row.worktree_path),
            &screen,
        );
    }

    // @step Given the session has 40 worktrees
    let forty: Vec<SessionWorktreeInfo> = (1..=40).map(worktree).collect();
    let last = forty[39].worktree_path.clone();

    // @step When the user opens /worktrees at 80x24
    let mut fx = open_worktrees(forty).await?;
    let screen = Screen::of(&mut fx.app, 80, 24);

    // @step Then "Prune" and "Cancel" are on screen
    for button in ["Prune", "Cancel"] {
        checks.check(
            dialog_shows(&screen, WORKTREES, button),
            &format!("80x24 with 40 worktrees: {button} is not on screen"),
            &screen,
        );
    }

    // @step When the user presses End
    key(&mut fx, KeyCode::End).await?;

    // @step Then the last worktree's full path is on screen whole and "Prune" is still on screen
    let screen = Screen::of(&mut fx.app, 80, 24);
    checks.check(
        dialog_shows(&screen, WORKTREES, &last),
        "80x24 after End: the last worktree's path is not shown whole",
        &screen,
    );
    checks.check(
        dialog_shows(&screen, WORKTREES, "Prune"),
        "80x24 after End: Prune is not on screen",
        &screen,
    );
    checks.done()
}
