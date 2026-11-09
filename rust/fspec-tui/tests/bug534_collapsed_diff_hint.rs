// Feature: spec/features/a-collapsed-diff-hint-names-a-key-not-the-removed-expand-command.feature
//! BUG-534 — a collapsed diff hint names a key, not the removed /expand
//! command.
//!
//! Drives the real store path through the `App`: a ToolCall chunk (Edit or
//! apply_patch) then its ToolResult, and reads the wrapped lines of the tool
//! card and the full text the turn modal shows.

use codelet_test_helpers::{OrFail, TestError, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolResultInfo};

mod common;
use common::MockBackend;

const HINT_SUFFIX: &str = " lines (Tab to select, Enter to view full)";

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

fn app_with_session() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app
}

fn tool_call(name: &str, input: String) -> Action {
    Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::tool_call(ToolCallInfo {
            id: "tc-1".to_string(),
            name: name.to_string(),
            input,
        }),
    )
}

fn tool_result() -> Action {
    Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::tool_result(ToolResultInfo {
            tool_call_id: "tc-1".to_string(),
            content: "ok".to_string(),
            is_error: false,
        }),
    )
}

/// The text of every wrapped line of the tool card (the first chunk), and
/// the full text the turn modal shows for it.
fn tool_card(app: &App) -> Result<(Vec<String>, String), TestError> {
    let context = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .or_fail("session s-1 is open")?;
    let window = context.scrollback.visible_window(4096);
    let card = window
        .first()
        .or_fail("the tool card is in the scrollback")?;
    let lines = card
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect();
    let full = context
        .scrollback
        .full_text_for_seq(card.seq)
        .or_fail("the turn modal's full text")?;
    Ok((lines, full))
}

fn json(value: &str) -> Result<String, TestError> {
    Ok(serde_json::to_string(value).or_fail("encode a JSON string")?)
}

/// The last non-empty line of the card.
fn last_line(lines: &[String]) -> Result<&String, TestError> {
    Ok(lines
        .iter()
        .rev()
        .find(|line| !line.is_empty())
        .or_fail("the card has lines")?)
}

/// The diff rows of the card: the lines after the header line, up to the
/// hint line.
fn rows_before_hint(lines: &[String]) -> Result<usize, TestError> {
    let hint = lines
        .iter()
        .position(|line| line.ends_with(HINT_SUFFIX))
        .or_fail("the card ends with the hint")?;
    Ok(hint - 1)
}

// Scenario: A long Edit diff ends with the Enter to view full hint
#[test]
fn a_long_edit_diff_ends_with_the_enter_to_view_full_hint() -> TestResult {
    // @step Given a session whose agent edits a file, replacing 60 lines with 60 others
    let mut app = app_with_session();
    let old: String = (1..=60).map(|i| format!("old{i}\n")).collect();
    let new: String = (1..=60).map(|i| format!("new{i}\n")).collect();
    let input = format!(
        r#"{{"old_string":{},"new_string":{}}}"#,
        json(&old)?,
        json(&new)?
    );
    app.dispatch(tool_call("Edit", input));
    // @step When the edit's tool result arrives in the agent view
    app.dispatch(tool_result());
    let (lines, full) = tool_card(&app)?;
    // @step Then the tool card shows 25 diff lines followed by a "... +N lines (Tab to select, Enter to view full)" hint
    let last = last_line(&lines)?;
    assert!(
        last.trim_start().starts_with("... +") && last.ends_with(HINT_SUFFIX),
        "the card must end with the hint: {lines:?}"
    );
    assert_eq!(rows_before_hint(&lines)?, 25, "{lines:?}");
    // @step And no line of the tool card mentions "/expand"
    assert!(
        !lines.iter().any(|line| line.contains("/expand")),
        "{lines:?}"
    );
    // @step And the full diff the turn modal shows holds every line and no hint
    assert!(full.contains("old1") && full.contains("new60"), "{full}");
    assert!(
        !full.contains(HINT_SUFFIX) && !full.contains("/expand"),
        "{full}"
    );
    Ok(())
}

// Scenario: A long apply_patch diff ends with the Enter to view full hint
#[test]
fn a_long_apply_patch_diff_ends_with_the_enter_to_view_full_hint() -> TestResult {
    // @step Given a session whose agent applies a patch adding a 40-line file
    let mut app = app_with_session();
    let body: String = (1..=40).map(|i| format!("+added{i}\n")).collect();
    let patch = format!("*** Begin Patch\n*** Add File: notes.txt\n{body}*** End Patch\n");
    app.dispatch(tool_call(
        "apply_patch",
        format!(r#"{{"patch":{}}}"#, json(&patch)?),
    ));
    // @step When the patch's tool result arrives in the agent view
    app.dispatch(tool_result());
    let (lines, _full) = tool_card(&app)?;
    // @step Then the tool card shows 25 diff rows, the file header and 24 added lines, followed by "... +16 lines (Tab to select, Enter to view full)"
    let last = last_line(&lines)?;
    assert!(
        last.ends_with("... +16 lines (Tab to select, Enter to view full)"),
        "{lines:?}"
    );
    assert_eq!(rows_before_hint(&lines)?, 25, "{lines:?}");
    // @step And no line of the tool card mentions "/expand"
    assert!(
        !lines.iter().any(|line| line.contains("/expand")),
        "{lines:?}"
    );
    Ok(())
}
