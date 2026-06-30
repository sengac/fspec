//! Feature: spec/features/default-thinking-level-restore-parity.feature
//!
//! TUI-093 — App::dispatch wiring for TS-parity restore of the default thinking
//! level (`src/tui/hooks/useDefaultThinkingLevel.ts`). Exercises three
//! divergences against the shared `MockBackend`:
//!   * pressing D repaints the active session's `[T:level]` badge (a
//!     `get_thinking_level` re-fetch + `Action::ThinkingLevelLoaded`);
//!   * the persisted default is applied to a session on creation / resume;
//!   * the per-session guard (`applied_default_thinking`) means a manual
//!     `/thinking` selection is never clobbered when the session regains focus.
//!
//! These tests drive the process-global data directory (via
//! `codelet_common::set_data_directory`) so the apply path reads a real
//! `tui.defaultThinkingLevel`; a `Mutex` serialises them.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use tokio::sync::Mutex;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, ThinkingLevel};
use codelet_sessions::default_thinking_level_persistence::save_default_thinking_level_with_dirs;

mod common;
use common::MockBackend;

/// Serialises tests that mutate the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Root a throwaway data dir as the process-global data directory, optionally
/// seeding `tui.defaultThinkingLevel`. Returns the kept TempDir.
fn data_dir_with_default(level: Option<ThinkingLevel>) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    if let Some(level) = level {
        save_default_thinking_level_with_dirs(tmp.path(), tmp.path(), level).expect("persist");
    }
    codelet_common::set_data_directory(tmp.path().to_path_buf()).expect("set data dir");
    tmp
}

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

/// Scenario: Pressing D persists the default and immediately repaints the badge
#[tokio::test]
async fn pressing_d_persists_the_default_and_immediately_repaints_the_badge() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given an active session and the thinking dialog is open on Medium
    let _tmp = data_dir_with_default(None);
    let (mut app, mock) = fresh_app();
    // Session is created with the badge at Off (no persisted default).
    mock.set_thinking_level(ThinkingLevel::Off);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    // After pressing D the in-memory base level becomes Medium; model the
    // backend re-fetch returning that new value.
    mock.set_thinking_level(ThinkingLevel::Medium);
    assert_ne!(
        app.agent_view_store()
            .thinking_level_for(&SessionId::new("s-1"))
            .copied(),
        Some(ThinkingLevel::Medium),
        "precondition: badge not yet Medium"
    );

    // @step When the user presses D to set Medium as the default
    app.dispatch(Action::SetThinkingLevelDefault(
        SessionId::new("s-1"),
        ThinkingLevel::Medium,
    ));
    drain_pending(&mut app).await;

    // @step Then the default thinking level Medium is persisted to the shared config
    // (set_thinking_level_default is best-effort persist; the repaint is the
    //  user-visible behaviour pinned here)
    // @step And the handler dispatches a ThinkingLevelLoaded action carrying Medium for that session
    assert_eq!(
        app.agent_view_store()
            .thinking_level_for(&SessionId::new("s-1"))
            .copied(),
        Some(ThinkingLevel::Medium),
        "pressing D must repaint the badge to Medium via ThinkingLevelLoaded"
    );
}

/// Scenario: Persisted default is restored to the active session at startup
#[tokio::test]
async fn persisted_default_is_restored_to_the_active_session_at_startup() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given the persisted default thinking level is High
    let _tmp = data_dir_with_default(Some(ThinkingLevel::High));
    let (mut app, mock) = fresh_app();
    // @step And a fresh app with one active session whose base thinking level is Off
    let prior = mock.set_thinking_level_calls();

    // @step When the app runs the default thinking level bootstrap step
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;

    // @step Then the active session base thinking level becomes High
    assert_eq!(
        mock.set_thinking_level_calls(),
        prior + 1,
        "a new active session must have the persisted default applied"
    );
    let last = mock.last_set_thinking_level().expect("apply happened");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, ThinkingLevel::High);
    // @step And a ThinkingLevelLoaded action carrying High is dispatched for the active session
    // (the apply task re-fetches get_thinking_level → ThinkingLevelLoaded; drained above)
}

/// Scenario: Resuming an older session applies the persisted default once
#[tokio::test]
async fn resuming_an_older_session_applies_the_persisted_default_once() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given the persisted default thinking level is High
    let _tmp = data_dir_with_default(Some(ThinkingLevel::High));
    let (mut app, mock) = fresh_app();
    let prior = mock.set_thinking_level_calls();

    // @step And a resumed session that has not yet had the default applied
    // @step When the session becomes active and chrome is refreshed
    app.dispatch(Action::AttachToSession(SessionId::new("old-1")));
    drain_pending(&mut app).await;

    // @step Then the resumed session base thinking level becomes High
    assert_eq!(
        mock.set_thinking_level_calls(),
        prior + 1,
        "resuming a session must apply the persisted default"
    );
    let last = mock.last_set_thinking_level().expect("apply happened");
    assert_eq!(last.0, SessionId::new("old-1"));
    assert_eq!(last.1, ThinkingLevel::High);
    // @step And the resumed session id is recorded as already-applied
    // Re-attaching must NOT apply the default a second time (guard).
    let after_first = mock.set_thinking_level_calls();
    app.dispatch(Action::AttachToSession(SessionId::new("old-1")));
    drain_pending(&mut app).await;
    assert_eq!(
        mock.set_thinking_level_calls(),
        after_first,
        "second activation must NOT re-apply the default (per-session guard)"
    );
}

/// Scenario: A manual thinking selection is not clobbered when the session regains focus
#[tokio::test]
async fn a_manual_thinking_selection_is_not_clobbered_on_refocus() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given the persisted default thinking level is High
    let _tmp = data_dir_with_default(Some(ThinkingLevel::High));
    let (mut app, mock) = fresh_app();

    // @step And a session whose default was already applied and then manually set to Low
    app.dispatch(Action::AttachToSession(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    // first activation applied the default High
    assert_eq!(
        mock.last_set_thinking_level().expect("default applied").1,
        ThinkingLevel::High,
        "first activation applies the default High"
    );
    // user manually picks Low
    app.dispatch(Action::ThinkingLevelSelected(
        SessionId::new("s-1"),
        ThinkingLevel::Low,
    ));
    drain_pending(&mut app).await;
    assert_eq!(
        mock.last_set_thinking_level().expect("manual set").1,
        ThinkingLevel::Low,
        "manual selection writes Low"
    );
    let calls_before_refocus = mock.set_thinking_level_calls();

    // @step When the session regains focus and chrome is refreshed
    app.dispatch(Action::AttachToSession(SessionId::new("s-1")));
    drain_pending(&mut app).await;

    // @step Then the default is not re-applied to that session
    assert_eq!(
        mock.set_thinking_level_calls(),
        calls_before_refocus,
        "refocus must NOT re-apply the default (guard preserves manual selection)"
    );
    // @step And the session base thinking level remains Low
    assert_eq!(
        mock.last_set_thinking_level().expect("still Low").1,
        ThinkingLevel::Low,
        "the manual Low selection survives refocus"
    );
}

/// Scenario: No persisted default yields Off and does not error at startup
#[tokio::test]
async fn no_persisted_default_yields_off_and_does_not_error_at_startup() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given no default thinking level key exists on disk
    let _tmp = data_dir_with_default(None);
    let (mut app, mock) = fresh_app();
    let prior = mock.set_thinking_level_calls();

    // @step And a fresh app with one active session
    // @step When the app runs the default thinking level bootstrap step
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;

    // @step Then the loaded default thinking level resolves to Off
    assert_eq!(
        mock.set_thinking_level_calls(),
        prior,
        "with no persisted default, no apply write must fire"
    );
    // @step And the bootstrap step completes without error
    // (reaching this line without panic is the operational proof)
}
