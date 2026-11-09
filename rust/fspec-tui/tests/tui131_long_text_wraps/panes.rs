//! TUI-131 — panes: the blocklist detail pane, the changed files view and
//! the exec-stdin notice wrap their text.
//!
//! Feature: spec/features/long-text-wraps-instead-of-being-cut-off.feature

use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::Action;
use codelet_rpc_types::{BlocklistRuleInfo, ChangedFile, ExecStdinRequest, StreamChunk};
use codelet_test_helpers::TestResult;

use super::fixture::*;
use super::screen::{Checks, Screen};
use crate::common::MockBackend;

/// Scenario: The blocklist detail pane wraps a long guidance
#[tokio::test(start_paused = true)]
async fn the_blocklist_detail_pane_wraps_a_long_guidance() -> TestResult {
    // @step Given the blocklist view shows a rule whose guidance is 200 characters long
    let guidance = prose("use", 200, "instead-of-cat");
    let mock = Arc::new(MockBackend::new());
    mock.seed_blocklist_rules(vec![BlocklistRuleInfo {
        id: "cat-block".into(),
        pattern: "^cat\\s+".into(),
        action: "block".into(),
        reason: "cat is blocked".into(),
        guidance: Some(guidance.clone()),
        source: "project".into(),
    }]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::SessionCreated(sid("s-1")));
    settle(&mut fx.app).await?;
    let session = fx.app.current_session();
    fx.app.dispatch(Action::SlashCommandSelected(
        session,
        SlashCommandAction::Blocklist,
    ));
    settle(&mut fx.app).await?;
    let mut checks = Checks::default();

    // @step When the view is drawn at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);

    // @step Then the guidance is on screen whole, wrapped to the detail pane
    checks.check(
        screen.shown_whole(&guidance),
        "80x24: the guidance is not shown whole",
        &screen,
    );
    // @step And it is on screen whole at 120x40
    let screen = Screen::of(&mut fx.app, 120, 40);
    checks.check(
        screen.shown_whole(&guidance),
        "120x40: the guidance is not shown whole",
        &screen,
    );
    checks.done()
}

/// Scenario: A changed file keeps its file name and a long diff line wraps
#[tokio::test(start_paused = true)]
async fn a_changed_file_keeps_its_file_name_and_a_long_diff_line_wraps() -> TestResult {
    // @step Given the changed files view lists a file whose path is "a/" repeated 30 times then "file_name.rs"
    let path = "a/".repeat(30) + "file_name.rs";
    let mock = Arc::new(MockBackend::new());
    mock.set_changed_files(vec![ChangedFile {
        path: path.clone(),
        change_type: "M".into(),
        staged: false,
    }]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::OpenChangedFilesView);
    settle(&mut fx.app).await?;
    // @step And its diff has a 120-character added line
    let added = format!("+{}", prose("added", 119, "line-tail"));
    assert_eq!(added.chars().count(), 120);
    fx.app.dispatch(Action::FileDiffLoaded {
        path: path.clone(),
        diff: Some(format!("@@ -1 +1 @@\n-old line\n{added}")),
    });
    settle(&mut fx.app).await?;
    let mut checks = Checks::default();

    // @step When the view is drawn at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);

    // @step Then the file's row ends with "file_name.rs"
    let row = screen.row_from("a/a/a/").unwrap_or_default();
    checks.check(
        row.ends_with("file_name.rs"),
        &format!("80x24: the file's row is {row:?}, which does not end with file_name.rs"),
        &screen,
    );
    // @step And the 120-character diff line is on screen whole, wrapped to the diff pane
    checks.check(
        screen.shown_whole(&added),
        "80x24: the 120-character diff line is not shown whole",
        &screen,
    );
    // @step And at 120x40 the diff line is on screen whole
    let screen = Screen::of(&mut fx.app, 120, 40);
    checks.check(
        screen.shown_whole(&added),
        "120x40: the 120-character diff line is not shown whole",
        &screen,
    );
    checks.done()
}

const STDIN_KEYS: &str = "Ctrl+O answer · Ctrl+X dismiss";

/// Scenario: The exec-stdin notice keeps its key hints for a long command or probe error
#[tokio::test(start_paused = true)]
async fn the_exec_stdin_notice_keeps_its_key_hints_for_a_long_command_or_probe_error() -> TestResult
{
    // @step Given the agent view is open
    let mut fx = agent_fixture(Arc::new(MockBackend::new())).await?;
    fx.app.dispatch(Action::SessionCreated(sid("other")));
    fx.app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    settle(&mut fx.app).await?;
    let mut checks = Checks::default();

    // @step When a tool asks for stdin for a 70-character command
    let command = token("git commit --amend --message-file=/tmp/fspec-", 70);
    let request = ExecStdinRequest {
        exec_session_id: "exec-1".into(),
        command: command.clone(),
        quiet_seconds: 5,
        ts_ms: 1000,
    };
    fx.mock
        .script_exec_stdin_request(sid("s-1"), Some(request.clone()));
    fx.app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ExecStdinRequest { request },
    ));
    settle(&mut fx.app).await?;

    // @step Then "Ctrl+O answer · Ctrl+X dismiss" is on screen at 80x24 and at 120x40
    for (width, height) in [(80, 24), (120, 40)] {
        let screen = Screen::of(&mut fx.app, width, height);
        checks.check(
            screen.shown_whole(STDIN_KEYS),
            &format!("{width}x{height}: the notice's key hints are not on screen"),
            &screen,
        );
    }

    // @step When the probe for that request fails with a 150-character error
    let error = prose("probe", 150, "transport-closed");
    fx.mock.set_get_exec_stdin_request_error(error.clone());
    // Focusing the session again probes its request.
    fx.app.dispatch(Action::SessionNext);
    fx.app.dispatch(Action::SessionPrev);
    settle(&mut fx.app).await?;
    let marked = fx
        .app
        .agent_view_store()
        .exec_stdin_probe_error(&sid("s-1"));
    assert_eq!(
        marked,
        Some(error.as_str()),
        "the probe failure is recorded"
    );

    // @step Then the probe error is on screen whole at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);
    checks.check(
        screen.shown_whole(&error),
        "80x24: the probe error is not shown whole",
        &screen,
    );
    // @step And "Ctrl+O answer · Ctrl+X dismiss" is still on screen
    checks.check(
        screen.shown_whole(STDIN_KEYS),
        "80x24 after the probe error: the key hints are not on screen",
        &screen,
    );
    checks.done()
}
