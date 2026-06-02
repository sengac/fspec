//! App-level integration tests using MockBackend (RPC-008).
//!
//! Feature: spec/features/fspec-tui-app-shell.feature
//!
//! Scenarios covered:
//!   - "App::new pre-populates the compositor with a Background-priority HelloComponent"
//!   - "'?' at App level pushes the HelpDialog onto the compositor"
//!   - "ESC while the HelpDialog is on top removes the dialog via deferred callback"
//!   - "'q' at App level sets should_quit and the run loop exits"
//!   - "App-with-mock-backend snapshot captures HelpDialog visible after '?' and gone after ESC"

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{synth_key, App, FspecBackend, Priority};
use crossterm::event::KeyCode;

mod common;

use common::{buffer_to_rows, render_one_frame, test_app, MockBackend};

/// Helper: build an `App` against a fresh MockBackend.
fn fresh_app_with_mock_backend() -> (App, ratatui::Terminal<ratatui::backend::TestBackend>) {
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    test_app(backend)
}

/// Scenario: App::new pre-populates the compositor with a
/// Background-priority HelloComponent
///
/// RPC-009 update: HelloComponent is no longer pre-populated by
/// `App::new` (rule [15]). RootView replaced it as the always-on
/// background.
///
/// RPC-012 update: RootView was retired in favour of the Navigator
/// (which paints Board or Agent + footer). The Navigator lives directly
/// on the App struct, NOT in the Compositor. The Compositor is now
/// reserved for MODAL layers (HelpDialog, DisconnectDialog).
#[test]
fn app_new_pre_populates_the_compositor_with_background_priority_hello_component() {
    // @step Given a mock backend implementing `dyn FspecBackend`
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());

    // @step When `App::new(Arc::new(mock))` is constructed
    let app = App::new(backend);

    // @step Then the App's compositor contains exactly one layer
    // (RPC-012: compositor is empty at construction; Navigator is the
    // always-on background held directly on the App struct.)
    assert_eq!(app.compositor().len(), 0);

    // @step And that layer's id() returns "hello"
    // (RPC-012: Navigator's id is "navigator"; it lives on App.navigator()
    // not in the compositor.)
    assert_eq!(app.navigator().id(), "navigator");

    // @step And that layer's priority() returns Priority::Background
    assert_eq!(app.navigator().priority(), Priority::Background);
    // Silence unused-import warning for `Priority` when this test is
    // the only consumer (legacy RPC-009 invariant).
    let _ = Priority::Background;
}

/// Scenario: '?' at App level pushes the HelpDialog onto the compositor
#[test]
fn question_at_app_level_pushes_the_help_dialog_onto_the_compositor() {
    // @step Given an App constructed against a MockBackend
    let (mut app, _term) = fresh_app_with_mock_backend();

    // @step And the App's compositor contains exactly the HelloComponent
    // (RPC-009: compositor is empty until `?` pushes the HelpDialog.)
    assert_eq!(app.compositor().len(), 0);

    // @step When the App receives a synthetic Key('?') event
    let _ = app.handle_event(&synth_key(KeyCode::Char('?')));

    // @step Then the compositor contains exactly two layers
    // (RPC-009: HelpDialog is the only modal — compositor goes from 0
    // to 1.)
    assert_eq!(app.compositor().len(), 1);

    // @step And the topmost layer's priority() returns Priority::Critical
    assert_eq!(app.compositor().topmost_priority(), Some(Priority::Critical));

    // @step And the topmost layer's id() returns "help-dialog"
    assert_eq!(app.compositor().topmost_id(), Some("help-dialog".to_string()));
}

/// Scenario: ESC while the HelpDialog is on top removes the dialog via
/// deferred callback
#[test]
fn esc_while_help_dialog_on_top_removes_the_dialog_via_deferred_callback() {
    // @step Given an App with a HelpDialog pushed onto the compositor at Priority::Critical
    let (mut app, _term) = fresh_app_with_mock_backend();
    let _ = app.handle_event(&synth_key(KeyCode::Char('?')));
    assert_eq!(app.compositor().topmost_id(), Some("help-dialog".to_string()));

    // @step When the App receives a synthetic Key(Esc) event
    let result = app.handle_event(&synth_key(KeyCode::Esc));

    // @step Then the dialog's handle_event returned `Consumed(Some(callback))`
    assert!(result.is_consumed());

    // @step And the App ran the callback against the compositor after dispatch completed
    // @step And the compositor now contains only the HelloComponent
    // (RPC-009: after ESC the compositor is empty; RootView remains
    // the always-on background on App.root().)
    assert_eq!(app.compositor().len(), 0);
}

/// Scenario: Ctrl+D at App level sets should_quit and the run loop exits.
///
/// RPC-102: replaced the legacy `q` quit binding with a BoardView ESC
/// confirmation dialog (see `boardview_esc_exit_confirmation_rpc102.rs`).
/// `Ctrl+D` remains the lowest-level hard-exit shortcut.
#[test]
fn ctrl_d_at_app_level_sets_should_quit_and_run_loop_exits() {
    // @step Given an App with a MockBackend and the run loop driven by a TestBackend terminal
    let (mut app, _term) = fresh_app_with_mock_backend();

    // @step When the App receives a synthetic Key(Ctrl+D) event
    use crossterm::event::{Event, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    let ctrl_d = Event::Key(KeyEvent {
        code: KeyCode::Char('d'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let _ = app.handle_event(&ctrl_d);

    // @step Then the App's should_quit flag is true
    assert!(app.should_quit());

    // @step And the next iteration of the tokio::select! breaks the loop
    // @step And `App::run().await` returns Ok(())
    // App::run() drives `tokio::select!` over the crossterm EventStream,
    // the action_rx, and a 16ms render-tick interval (rule [11]). Its
    // loop predicate is `while !self.should_quit`, so once the App has
    // observed should_quit=true the next iteration's predicate fails
    // and the function returns `Ok(())`. We type-check that surface
    // here without standing up a real TTY (which `App::run` requires
    // for the `TerminalGuard::init()` call).
    let _typecheck: fn(App) -> _ = |app| async move { app.run().await };
}

/// Scenario: App-with-mock-backend snapshot captures HelpDialog visible
/// after '?' and gone after ESC
#[test]
fn app_with_mock_backend_snapshot_captures_help_dialog_visible_then_dismissed() {
    // @step Given an App constructed against a MockBackend on an 80x24 TestBackend terminal
    let (mut app, mut terminal) = fresh_app_with_mock_backend();

    // @step When the App processes a synthetic Key('?') event and renders one frame
    let _ = app.handle_event(&synth_key(KeyCode::Char('?')));
    let frame_visible = render_one_frame(&mut terminal, &mut app);
    let rows_visible = buffer_to_rows(&frame_visible);

    // @step Then the rendered buffer matches the insta snapshot "help_dialog_visible"
    insta::assert_yaml_snapshot!("help_dialog_visible", rows_visible);

    // @step When the App processes a synthetic Key(Esc) event and renders one frame
    let _ = app.handle_event(&synth_key(KeyCode::Esc));
    let frame_dismissed = render_one_frame(&mut terminal, &mut app);
    let rows_dismissed = buffer_to_rows(&frame_dismissed);

    // @step Then the rendered buffer matches the insta snapshot "help_dialog_dismissed"
    insta::assert_yaml_snapshot!("help_dialog_dismissed", rows_dismissed);
}
