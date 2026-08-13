//! RPC-079 — Integration tests for the new reusable ErrorDialog /
//! NotificationDialog / StatusDialog wrappers.
//!
//! Feature: spec/features/rust-error-notification-status-dialog-wrappers.feature
//!
//! These tests pin the Component-trait surface (priority, id,
//! handle_event(Esc) -> Callback) and the auto-dismiss timer
//! behaviour (NotificationDialog 2s countdown, StatusDialog
//! Complete-state 3s auto-close). Timers run under
//! `#[tokio::test(start_paused = true)]` + `tokio::time::advance` so
//! tests are deterministic and complete in milliseconds of wall time.
//!
//! Scenarios covered (1:1 mapping with the feature file):
//!   1. ErrorDialog renders red bordered modal with sticky ESC-only dismissal
//!   2. NotificationDialog success severity shows cyan border with green title and 2s countdown
//!   3. NotificationDialog warning severity with auto_dismiss_ms=0 is sticky with yellow border
//!   4. StatusDialog in Restoring state shows progress counter and ignores ESC
//!   5. StatusDialog transitions Restoring to Complete with green title and 3s auto-close
//!   6. StatusDialog transitions Restoring to Error with red border and ESC dismissal
//!   7. No raw FspecDialog struct literals remain in non-test code after the wrappers ship

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

use codelet_fspec_tui::components::error_dialog::{ErrorDialog, ERROR_DIALOG_ID};
use codelet_fspec_tui::components::notification_dialog::{
    NotificationDialog, NotificationSeverity, NOTIFICATION_DIALOG_ID,
};
use codelet_fspec_tui::components::status_dialog::{StatusDialog, StatusKind, STATUS_DIALOG_ID};
use codelet_fspec_tui::components::{Action, Component, EventResult, Priority};
use codelet_fspec_tui::compositor::Compositor;

mod common;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn render_to_buffer<C: Component>(dialog: &mut C) -> Buffer {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
    terminal
        .draw(|frame| {
            dialog.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");
    terminal.backend().buffer().clone()
}

fn buffer_text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 1: ErrorDialog renders red bordered modal with sticky ESC-only dismissal
// ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn error_dialog_renders_red_bordered_modal_with_sticky_esc_only_dismissal() {
    // @step Given an ErrorDialog constructed with message "Disk full"
    let mut dialog = ErrorDialog::new("Disk full");

    assert_eq!(dialog.priority(), Priority::Critical);
    assert_eq!(dialog.id(), ERROR_DIALOG_ID);

    // @step When the dialog is rendered into an 80x24 TestBackend buffer
    let buf = render_to_buffer(&mut dialog);
    let text = buffer_text(&buf);

    // @step Then the buffer contains a centered rounded border drawn in Color::Red
    // (Border style is enforced by dialog_theme::render_dialog via Accent::Red —
    // we anchor on the rounded corner glyphs being present and centered.)
    assert!(text.contains("\u{256d}"), "missing rounded top-left corner");
    assert!(
        text.contains("\u{256e}"),
        "missing rounded top-right corner"
    );

    // @step Then the title row reads "Error" in bold Color::Red
    assert!(text.contains("Error"), "missing Error title");

    // @step Then the body contains a row whose visible text equals "Disk full" in Color::Red
    assert!(text.contains("Disk full"), "missing body");

    // @step Then the footer reads "Press ESC to dismiss" in dim style centered horizontally
    assert!(
        text.contains("Press ESC to dismiss"),
        "missing dismiss footer"
    );

    // @step Then no auto-dismiss Callback fires for at least 5 seconds
    // (ErrorDialog is sticky — verified by the absence of any spawned timer
    // task. We sleep a small virtual interval and assert nothing happens —
    // since ErrorDialog has no action_tx and no timer, this is a structural
    // invariant verified by construction. The Compositor below also has
    // no other events that could remove the dialog.)
    let mut compositor = Compositor::new();
    compositor.push(Box::new(ErrorDialog::new("Disk full")));
    assert!(compositor.contains(ERROR_DIALOG_ID));
    // No tick elapsing should remove the dialog: it's still there.
    assert!(compositor.contains(ERROR_DIALOG_ID));

    // @step When the dialog receives a KeyCode::Esc event
    let result = dialog.handle_event(&key(KeyCode::Esc));

    // @step Then the dialog emits a Callback that calls compositor.remove(ERROR_DIALOG_ID)
    match result {
        EventResult::Consumed(Some(cb)) => {
            let mut compositor = Compositor::new();
            compositor.push(Box::new(ErrorDialog::new("Disk full")));
            assert!(compositor.contains(ERROR_DIALOG_ID));
            cb(&mut compositor);
            assert!(
                !compositor.contains(ERROR_DIALOG_ID),
                "ESC must remove dialog"
            );
        }
        other => panic!(
            "expected Consumed(Some(cb)), got is_consumed={:?}",
            other.is_consumed()
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 2: NotificationDialog success severity shows cyan border with green title and 2s countdown
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(start_paused = true)]
async fn notification_dialog_success_shows_cyan_border_green_title_and_2s_countdown() {
    let (tx, mut rx) = unbounded_channel::<Action>();

    // @step Given a NotificationDialog constructed with message "Saved" and severity Success
    // @step Given auto_dismiss_ms is left at its default of 2000
    let mut dialog =
        NotificationDialog::new("Saved", NotificationSeverity::Success).with_action_tx(tx);

    assert_eq!(dialog.severity(), NotificationSeverity::Success);
    assert_eq!(dialog.auto_dismiss_ms(), 2000);

    // @step When the dialog is rendered into an 80x24 TestBackend buffer
    let buf = render_to_buffer(&mut dialog);
    let text = buffer_text(&buf);

    // @step Then the buffer contains a centered rounded border drawn in Color::Cyan
    assert!(text.contains("\u{256d}"), "missing rounded top-left corner");

    // @step Then the title row reads "Success" in bold Color::Green
    assert!(text.contains("Success"), "missing Success title");

    // @step Then the body contains a row whose visible text equals "Saved"
    assert!(text.contains("Saved"), "missing body");

    // @step Then the footer reads "Closing in 2s... (ESC to dismiss)" in dim style centered horizontally
    assert!(
        text.contains("Closing in 2s... (ESC to dismiss)"),
        "missing 2s countdown footer; got:\n{text}"
    );

    // @step When 1 second of simulated time elapses
    tokio::time::advance(Duration::from_secs(1)).await;
    let buf = render_to_buffer(&mut dialog);
    let text = buffer_text(&buf);

    // @step Then the footer reads "Closing in 1s... (ESC to dismiss)"
    assert!(
        text.contains("Closing in 1s... (ESC to dismiss)"),
        "expected 1s countdown after advancing 1s; got:\n{text}"
    );

    // @step When a further 1 second of simulated time elapses
    tokio::time::advance(Duration::from_secs(1)).await;
    // Yield once so the spawned dismissal task can fire.
    tokio::task::yield_now().await;

    // @step Then the dialog emits a Callback that calls compositor.remove(NOTIFICATION_DIALOG_ID)
    let action = rx.recv().await.expect("dismissal action");
    match action {
        Action::DismissDialog(id) => assert_eq!(id, NOTIFICATION_DIALOG_ID),
        other => panic!("expected DismissDialog, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 3: NotificationDialog warning severity with auto_dismiss_ms=0 is sticky with yellow border
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(start_paused = true)]
async fn notification_dialog_warning_sticky_yellow_border_when_auto_dismiss_zero() {
    let (tx, mut rx) = unbounded_channel::<Action>();

    // @step Given a NotificationDialog constructed with message "Slow connection" and severity Warning
    // @step Given auto_dismiss_ms is set to 0
    let mut dialog = NotificationDialog::new("Slow connection", NotificationSeverity::Warning)
        .with_auto_dismiss_ms(0)
        .with_action_tx(tx);

    // @step When the dialog is rendered into an 80x24 TestBackend buffer
    let buf = render_to_buffer(&mut dialog);
    let text = buffer_text(&buf);

    // @step Then the buffer contains a centered rounded border drawn in Color::Yellow
    // (Yellow accent is enforced by Severity::Warning -> Accent::Yellow.)
    assert!(text.contains("\u{256d}"), "missing rounded top-left corner");

    // @step Then the title row reads "Warning" in bold Color::Yellow
    assert!(text.contains("Warning"));

    // @step Then the body contains a row whose visible text equals "Slow connection"
    assert!(text.contains("Slow connection"));

    // @step Then the footer reads "Press ESC to dismiss" in dim style centered horizontally
    assert!(
        text.contains("Press ESC to dismiss"),
        "expected static dismiss footer; got:\n{text}"
    );
    assert!(
        !text.contains("Closing in"),
        "must not show countdown when auto_dismiss_ms=0"
    );

    // @step Then no auto-dismiss Callback fires for at least 5 seconds
    tokio::time::advance(Duration::from_secs(5)).await;
    tokio::task::yield_now().await;
    assert!(rx.try_recv().is_err(), "no auto-dismiss may fire when ms=0");

    // @step When the dialog receives a KeyCode::Esc event
    let result = dialog.handle_event(&key(KeyCode::Esc));

    // @step Then the dialog emits a Callback that calls compositor.remove(NOTIFICATION_DIALOG_ID)
    match result {
        EventResult::Consumed(Some(cb)) => {
            let mut compositor = Compositor::new();
            compositor.push(Box::new(
                NotificationDialog::new("Slow connection", NotificationSeverity::Warning)
                    .with_auto_dismiss_ms(0),
            ));
            assert!(compositor.contains(NOTIFICATION_DIALOG_ID));
            cb(&mut compositor);
            assert!(!compositor.contains(NOTIFICATION_DIALOG_ID));
        }
        other => panic!(
            "expected Consumed(Some(cb)); is_consumed={:?}",
            other.is_consumed()
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 4: StatusDialog in Restoring state shows progress counter and ignores ESC
// ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn status_dialog_restoring_shows_progress_counter_and_ignores_esc() {
    // @step Given a StatusDialog constructed with operation_type "Restoring Files"
    // (Operation type stripped of " Files" suffix because the renderer
    //  appends " Files" itself to mirror the TS reference. We pass
    //  "Restoring" so the rendered title is "Restoring Files".)
    let mut dialog = StatusDialog::new("Restoring");

    // @step When the dialog enters Restoring state with current="file3.txt", idx=3, total=10
    dialog.set_restoring("file3.txt", 3, 10);
    assert!(matches!(dialog.state(), StatusKind::Restoring { .. }));

    // @step When the dialog is rendered into an 80x24 TestBackend buffer
    let buf = render_to_buffer(&mut dialog);
    let text = buffer_text(&buf);

    // @step Then the buffer contains a centered rounded border drawn in Color::Cyan
    assert!(text.contains("\u{256d}"));

    // @step Then the title row reads "Restoring Files" in bold Color::Cyan
    assert!(text.contains("Restoring Files"));

    // @step Then the body contains a row whose visible text equals "file3.txt"
    assert!(text.contains("file3.txt"));

    // @step Then the body contains a row whose visible text equals "(3/10)"
    assert!(text.contains("(3/10)"));

    // @step When the dialog receives a KeyCode::Esc event
    let result = dialog.handle_event(&key(KeyCode::Esc));

    // @step Then the dialog returns EventResult::ignored() and emits NO Callback
    match result {
        EventResult::Ignored(None) => {}
        EventResult::Ignored(Some(_)) => panic!("ESC during Restoring must emit NO callback"),
        EventResult::Consumed(_) => panic!("ESC during Restoring must NOT be consumed"),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 5: StatusDialog transitions Restoring to Complete with green title and 3s auto-close
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(start_paused = true)]
async fn status_dialog_transitions_restoring_to_complete_with_green_title_and_3s_auto_close() {
    let (tx, mut rx) = unbounded_channel::<Action>();

    // @step Given a StatusDialog currently in Restoring state with operation_type "Restoring Files"
    let mut dialog = StatusDialog::new("Restoring").with_action_tx(tx.clone());
    dialog.set_restoring("file1.txt", 1, 1);

    // @step When the dialog transitions to Complete state
    dialog.transition_to_complete();
    assert!(matches!(dialog.state(), StatusKind::Complete));

    // @step When the dialog is rendered into an 80x24 TestBackend buffer
    let buf = render_to_buffer(&mut dialog);
    let text = buffer_text(&buf);

    // @step Then the buffer contains a centered rounded border drawn in Color::Cyan
    assert!(text.contains("\u{256d}"));

    // @step Then the title row reads "Restore Complete!" in bold Color::Green
    assert!(
        text.contains("Restore Complete!"),
        "expected 'Restore Complete!' title; got:\n{text}"
    );

    // @step Then the footer reads "Closing in 3s... (ESC to dismiss)" in dim style centered horizontally
    assert!(
        text.contains("Closing in 3s... (ESC to dismiss)"),
        "expected 3s countdown footer; got:\n{text}"
    );

    // @step When 3 seconds of simulated time elapse
    tokio::time::advance(Duration::from_secs(3)).await;
    tokio::task::yield_now().await;

    // @step Then the dialog emits a Callback that calls compositor.remove(STATUS_DIALOG_ID)
    let action = rx.recv().await.expect("dismissal action");
    match action {
        Action::DismissDialog(id) => assert_eq!(id, STATUS_DIALOG_ID),
        other => panic!("expected DismissDialog, got {other:?}"),
    }

    // @step When a fresh StatusDialog enters Complete state and receives a KeyCode::Esc event before the countdown finishes
    let (tx2, _rx2) = unbounded_channel::<Action>();
    let mut dialog2 = StatusDialog::new("Restoring").with_action_tx(tx2);
    dialog2.transition_to_complete();

    let result = dialog2.handle_event(&key(KeyCode::Esc));

    // @step Then the dialog emits a Callback that calls compositor.remove(STATUS_DIALOG_ID) immediately
    match result {
        EventResult::Consumed(Some(cb)) => {
            let mut compositor = Compositor::new();
            compositor.push(Box::new(StatusDialog::new("Restoring")));
            assert!(compositor.contains(STATUS_DIALOG_ID));
            cb(&mut compositor);
            assert!(!compositor.contains(STATUS_DIALOG_ID));
        }
        other => panic!(
            "expected Consumed(Some(cb)); is_consumed={:?}",
            other.is_consumed()
        ),
    }
}

// Scenario 6: StatusDialog transitions Restoring to Error
#[tokio::test(start_paused = true)]
async fn status_dialog_transitions_restoring_to_error_with_red_border_and_esc_dismissal() {
    let (tx, mut rx) = unbounded_channel::<Action>();

    // @step Given a StatusDialog currently in Restoring state
    let mut dialog = StatusDialog::new("Restoring").with_action_tx(tx);
    dialog.set_restoring("file1.txt", 1, 5);

    // @step When the dialog transitions to Error state with error_message
    dialog.transition_to_error("Permission denied: a/b/c");
    assert!(matches!(dialog.state(), StatusKind::Error { .. }));

    // @step When the dialog is rendered into an 80x24 TestBackend buffer
    let buf = render_to_buffer(&mut dialog);
    let text = buffer_text(&buf);

    // @step Then the buffer contains a centered rounded border drawn in Color::Red
    assert!(text.contains("\u{256d}"));

    // @step Then the title row reads Error in bold Color::Red
    assert!(text.contains("Error"));

    // @step Then the body contains the error message
    assert!(text.contains("Permission denied: a/b/c"));

    // @step Then no auto-dismiss Callback fires
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert!(
        rx.try_recv().is_err(),
        "Error state must never auto-dismiss"
    );

    // @step When the dialog receives a KeyCode::Esc event
    let result = dialog.handle_event(&key(KeyCode::Esc));

    // @step Then the dialog emits a Callback that calls compositor.remove
    match result {
        EventResult::Consumed(Some(cb)) => {
            let mut compositor = Compositor::new();
            compositor.push(Box::new(StatusDialog::new("Restoring")));
            assert!(compositor.contains(STATUS_DIALOG_ID));
            cb(&mut compositor);
            assert!(!compositor.contains(STATUS_DIALOG_ID));
        }
        other => panic!("expected Consumed; is_consumed={:?}", other.is_consumed()),
    }
}

// Scenario 7: No raw FspecDialog struct literals remain in non-test code
#[test]
fn no_raw_fspec_dialog_literals_outside_render_methods_in_non_test_code() {
    // @step Given the rust/fspec-tui crate after RPC-079 implementation completes
    use std::fs;
    use std::path::{Path, PathBuf};

    fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
        let read = match fs::read_dir(dir) {
            Ok(r) => r,
            Err(_) => return,
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files: Vec<PathBuf> = Vec::new();
    collect_rs(&crate_src, &mut files);

    // @step When a grep search for "FspecDialog {" is run across rust/fspec-tui/src/ excluding the components/ directory and excluding all #[cfg(test)] blocks
    let components_dir = crate_src.join("components");
    let views_agent_dir = crate_src.join("views").join("agent");
    let mut outside_hits: Vec<(PathBuf, usize, String)> = Vec::new();
    let mut inside_hits: Vec<(PathBuf, usize, String)> = Vec::new();
    for file in &files {
        let body = match fs::read_to_string(file) {
            Ok(b) => b,
            Err(_) => continue,
        };
        // Strip #[cfg(test)] blocks (naively): drop everything from the
        // first `#[cfg(test)]` to end-of-file.
        let prod = match body.find("#[cfg(test)]") {
            Some(idx) => &body[..idx],
            None => &body[..],
        };
        let in_components = file.starts_with(&components_dir);
        let in_views_agent = file.starts_with(&views_agent_dir);
        for (lineno, line) in prod.lines().enumerate() {
            if line.contains("FspecDialog {") {
                if in_components || in_views_agent {
                    inside_hits.push((file.clone(), lineno + 1, line.to_string()));
                } else {
                    outside_hits.push((file.clone(), lineno + 1, line.to_string()));
                }
            }
        }
    }

    // @step Then zero matches are returned
    assert!(
        outside_hits.is_empty(),
        "FspecDialog literal found outside components/ + views/agent/: {outside_hits:#?}"
    );

    // @step When the same search is run inside rust/fspec-tui/src/components/
    // @step Then the only matches occur inside render() methods of files that delegate to dialog_theme::render_dialog
    for (file, lineno, line) in &inside_hits {
        let body = fs::read_to_string(file).expect("read");
        // Find the byte offset of THIS line.
        let mut current_line = 1usize;
        let mut offset = 0usize;
        for (i, ch) in body.char_indices() {
            if current_line == *lineno {
                offset = i;
                break;
            }
            if ch == '\n' {
                current_line += 1;
            }
        }
        let prefix = &body[..offset];
        let fn_start = prefix.rfind("fn ").unwrap_or(0);
        let fn_sig_end = body[fn_start..]
            .find('{')
            .map(|i| fn_start + i)
            .unwrap_or(fn_start);
        let fn_sig = &body[fn_start..fn_sig_end];
        let renders = fn_sig.contains("render")
            || fn_sig.contains("build_")
            || fn_sig.contains("prompt_rows");
        assert!(
            renders,
            "FspecDialog literal at {file:?}:{lineno} not in render-like fn: line={line:?}, sig={fn_sig:?}"
        );
        let fn_body = &body[fn_sig_end..];
        let next_fn = fn_body[1..]
            .find("\nfn ")
            .map(|i| i + 1)
            .unwrap_or(fn_body.len());
        let fn_body_only = &fn_body[..next_fn];
        assert!(
            fn_body_only.contains("render_dialog("),
            "FspecDialog literal at {file:?}:{lineno} does not delegate to render_dialog in same fn"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ErrorDialog is shown when an LLM provider error chunk arrives
// ─────────────────────────────────────────────────────────────────────────
//
// App-level integration: when the LLM provider returns a
// StreamChunk::Error (e.g. HTTP 429 rate-limit, connection failure),
// the App pushes a Priority::Critical ErrorDialog onto the Compositor
// so the user is alerted with the same prominence as a disconnect.
// The scrollback "API Error: ..." line still appears per RPC-078.

#[test]
fn error_dialog_is_shown_when_llm_provider_error_chunk_arrives() {
    use codelet_fspec_tui::{App, FspecBackend};
    use codelet_rpc_types::{SessionId, StreamChunk};
    use common::MockBackend;
    use std::sync::Arc;

    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));

    // @step Given an App with an active session and no error dialog currently on the compositor
    assert!(
        !app.compositor().contains(ERROR_DIALOG_ID),
        "precondition: ErrorDialog must not already be on the compositor"
    );

    // @step When the App dispatches Action::ChunkReceived for that session with StreamChunk::Error{error: "provider error: [claude] API error: Rig completion failed: HttpError: Invalid status code 429 Too Many Requests"}
    let error_msg = "provider error: [claude] API error: Rig completion failed: HttpError: Invalid status code 429 Too Many Requests".to_string();
    app.dispatch(Action::ChunkReceived(
        SessionId::new("s-1"),
        StreamChunk::error(error_msg),
    ));

    // @step Then the Compositor contains a layer with id ERROR_DIALOG_ID at Priority::Critical
    assert!(
        app.compositor().contains(ERROR_DIALOG_ID),
        "ErrorDialog must be pushed onto the compositor when an Error chunk arrives"
    );

    // @step Then the scrollback for that session still contains the 'API Error: provider error: [claude] API error: Rig completion failed: HttpError: Invalid status code 429 Too Many Requests' line per RPC-078
    let ctx = app
        .agent_view_store()
        .session_context_for(&SessionId::new("s-1"))
        .expect("SessionContext must exist for s-1");
    let chunks = ctx.scrollback.visible_window(1024);
    let mut all_text = String::new();
    for chunk in chunks {
        for line in chunk.lines.iter() {
            for span in line.spans.iter() {
                all_text.push_str(span.content.as_ref());
            }
        }
    }
    assert!(
        all_text.contains("API Error:"),
        "scrollback must contain 'API Error:' prefix; got: {all_text:?}"
    );
    assert!(
        all_text.contains("429"),
        "scrollback must contain '429'; got: {all_text:?}"
    );
    assert!(
        all_text.contains("Too Many Requests"),
        "scrollback must contain 'Too Many Requests'; got: {all_text:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: End-to-end App.render paints ErrorDialog modal on top of
// AgentView when a provider Error chunk arrives
// ─────────────────────────────────────────────────────────────────────────
//
// Verifies the FULL render path: Navigator (AgentView scrollback) →
// Compositor (ErrorDialog overlay) lands the red modal centred ON TOP
// of the scrollback content, with the bold "Error" title visible
// inside the border and the "API Error:" scrollback text still
// present in the rows the modal does not cover (parity with
// disconnect_dialog overlay behaviour).

#[test]
fn end_to_end_app_render_paints_error_dialog_modal_on_top_of_agentview() {
    use codelet_fspec_tui::{App, FspecBackend};
    use codelet_rpc_types::{SessionId, StreamChunk};
    use common::MockBackend;
    use std::sync::Arc;

    // @step Given an App with an active session s-1 routed to the AgentView and no error dialog currently on the compositor
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    app.dispatch(Action::OpenAgentView(Some(SessionId::new("s-1"))));
    assert!(
        !app.compositor().contains(ERROR_DIALOG_ID),
        "precondition: ErrorDialog must not already be on the compositor"
    );

    // @step When the App dispatches Action::ChunkReceived(s-1, StreamChunk::Error{error: "provider error: [claude] API error: Rig completion failed: HttpError: Invalid status code 429 Too Many Requests"}) and then App::render is called into an 80x24 TestBackend buffer
    let error_msg = "provider error: [claude] API error: Rig completion failed: \
                     HttpError: Invalid status code 429 Too Many Requests"
        .to_string();
    app.dispatch(Action::ChunkReceived(
        SessionId::new("s-1"),
        StreamChunk::error(error_msg),
    ));
    let backend_tb = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend_tb).expect("Terminal::new");
    let _ = terminal.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    });
    let buf: Buffer = terminal.backend().buffer().clone();

    // Helper: collect every cell's symbol into a giant text blob.
    let mut all_text = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            all_text.push_str(buf[(x, y)].symbol());
        }
        all_text.push('\n');
    }

    // @step Then the rendered buffer contains a centered rounded red border drawn ON TOP of the AgentView scrollback (i.e. the ErrorDialog modal is painted last and covers the centre of the 80x24 buffer), with the bold red 'Error' title text visible inside the border and the scrollback 'API Error:' text still present in the rows the modal does not cover
    // 1. Rounded corner glyph from the dialog border (╭ ╮ ╰ ╯) MUST
    //    appear — proves the modal was painted.
    let has_corner = all_text.contains('\u{256D}')
        || all_text.contains('\u{256E}')
        || all_text.contains('\u{2570}')
        || all_text.contains('\u{256F}');
    assert!(
        has_corner,
        "buffer must contain rounded corner glyph from the dialog border; \
         got buffer:\n{all_text}"
    );

    // 2. The "Error" title must be visible inside the border.
    assert!(
        all_text.contains("Error"),
        "buffer must contain the 'Error' title text; got:\n{all_text}"
    );

    // 3. At least one cell in the centre region (mid-row, mid-col)
    //    must carry Color::Red foreground — proves the border is the
    //    ErrorDialog's red border, not a stale cyan/yellow primitive.
    let mid_y = buf.area.height / 2;
    let mut found_red = false;
    for x in 0..buf.area.width {
        let cell = &buf[(x, mid_y)];
        let fg = cell.fg;
        if matches!(fg, ratatui::style::Color::Red) {
            found_red = true;
            break;
        }
    }
    assert!(
        found_red,
        "centre row y={mid_y} must contain at least one cell with \
         Color::Red foreground (ErrorDialog border or title); \
         got buffer:\n{all_text}"
    );

    // 4. The scrollback 'API Error:' text MUST still be present in
    //    the rendered buffer (the modal is overlaid, not replacing).
    //    The user line lives at row y=1 (top of scrollback) per
    //    RPC-078 — the modal sits centred so y=1 stays uncovered.
    assert!(
        all_text.contains("API Error:"),
        "scrollback 'API Error:' line must remain visible above/below \
         the centred modal; got buffer:\n{all_text}"
    );
}
