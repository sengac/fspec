//! PROV-101 FIX 1 / BUG-410: the TUI surfaces a declined session creation
//! instead of swallowing it.
//!
//! Feature: spec/features/session-creation-decline-surfaced.feature
//!
//! `create_session` returns an error when no default model is set. The TUI
//! routes every creation error through its single failure path (the session
//! failure dialog) and never makes a session active that was not created.
//! Fully offline against the MockBackend fixture — no network, no env
//! mutation.

use codelet_fspec_tui::Action;
use codelet_rpc_types::SessionId;
use codelet_test_helpers::TestResult;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::harness::AppTestHarness;

const FAILURE_DIALOG_ID: &str = "session-storage-dialog";
const NO_DEFAULT_MODEL: &str = "no default model is set";

/// The rendered screen, one line per row.
fn screen(harness: &mut AppTestHarness) -> String {
    let area = Rect::new(0, 0, 200, 50);
    let mut buffer = Buffer::empty(area);
    harness.app.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// =============================================================================
// Scenario: The TUI surfaces a declined session creation as an explicit error
// =============================================================================
#[tokio::test]
async fn declined_session_creation_surfaces_failure_dialog() -> TestResult {
    // @step Given an App whose backend rejects create_session because no default model is set
    let mut harness = AppTestHarness::empty();
    harness.mock.script_create_session_error(
        "no default model is set: select a model with /model before creating a session",
    );

    // @step When the user confirms creating a non-isolated session
    harness
        .app
        .dispatch(Action::CreateSessionSubmitted { isolated: false });
    harness.drain_pending().await?;

    // @step Then the session failure dialog names the missing default model
    if !harness.compositor_contains(FAILURE_DIALOG_ID) {
        return Err("a declined creation must open the session failure dialog".into());
    }
    let shown = screen(&mut harness);
    if !shown.contains(NO_DEFAULT_MODEL) {
        return Err(format!("{NO_DEFAULT_MODEL:?} is not shown in:\n{shown}").into());
    }

    // @step And no session becomes the active session
    if let Some(session) = harness.app.agent_view_store().current_session() {
        return Err(format!("no session may become active, found {session:?}").into());
    }
    Ok(())
}

// =============================================================================
// Scenario: A created session becomes the active session
// =============================================================================
#[tokio::test]
async fn created_session_becomes_active() -> TestResult {
    // @step Given an App whose backend creates the session "s-1"
    let mut harness = AppTestHarness::empty();
    harness.mock.script_create_session(SessionId::new("s-1"));

    // @step When the user confirms creating a non-isolated session
    harness
        .app
        .dispatch(Action::CreateSessionSubmitted { isolated: false });
    harness.drain_pending().await?;

    // @step Then the session "s-1" becomes the active session
    let active = harness.app.agent_view_store().current_session().cloned();
    if active != Some(SessionId::new("s-1")) {
        return Err(format!("expected s-1 to be active, found {active:?}").into());
    }
    Ok(())
}
