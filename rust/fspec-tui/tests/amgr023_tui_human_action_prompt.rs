//! Feature: spec/features/a-human-action-has-its-own-done-or-cancel-reply.feature
//!
//! AMGR-023: the TUI shows a human action (the wire PauseKind::HumanAction)
//! as its own prompt, [Enter] Done and [Esc] Cancel, and answers it with
//! pause_human_action naming the request id. y and n are not choices there.

use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{
    HumanActionReply, PauseKind, PauseState, RequestHolder, RequestKind, RequestRoute, SessionId,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::{MockBackend, TaskOutcome};

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

/// The screen as text, one line per row.
fn render(app: &mut App) -> String {
    let area = Rect::new(0, 0, 120, 24);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn drain(app: &mut App) -> TestResult {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

/// The TUI has read the session's pending human action `request_id`.
fn human_action(app: &mut App, request_id: &str) {
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        route: RequestRoute {
            request_id: request_id.into(),
            origin: sid("s-1"),
            kind: RequestKind::HumanAction,
            chain: vec![sid("s-1")],
            holder: RequestHolder::User,
        },
        state: PauseState {
            kind: PauseKind::HumanAction,
            prompt: "WebSearch: Page loaded at https://example.com. Interact with the page, then press Enter to capture screenshot.".into(),
            details: None,
            tool_call_id: None,
        },
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_the_tui_prompt_answers_a_human_action_with_done_or_cancel() -> TestResult {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;

    // @step Given the TUI shows a session's pending WebSearch human action
    human_action(&mut app, "req-human-1");
    let screen = render(&mut app);
    assert!(screen.contains("Interact with the page"), "{screen}");

    // @step Then the prompt shows "[Enter] Done" and "[Esc] Cancel" and no Y/N choice
    assert!(screen.contains("[Enter] Done"), "{screen}");
    assert!(screen.contains("[Esc] Cancel"), "{screen}");
    assert!(!screen.contains("[Y]"), "{screen}");
    assert!(!screen.contains("[N]"), "{screen}");

    // @step When the user presses y, then Enter, and then Esc on the session's next human action
    app.handle_event(&key(KeyCode::Char('y')));
    drain(&mut app).await?;
    let after_y = mock.reply_request_ids();
    app.handle_event(&key(KeyCode::Enter));
    drain(&mut app).await?;
    human_action(&mut app, "req-human-2");
    render(&mut app);
    app.handle_event(&key(KeyCode::Esc));
    drain(&mut app).await?;

    // @step Then y sends nothing, and the backend receives pause_human_action with Done for the first request id and Cancel for the second
    assert!(after_y.is_empty(), "y sent {after_y:?}");
    assert_eq!(
        mock.reply_request_ids(),
        vec![
            ("pause_human_action", sid("s-1"), "req-human-1".to_string()),
            ("pause_human_action", sid("s-1"), "req-human-2".to_string()),
        ]
    );
    assert_eq!(
        mock.pause_human_action_log(),
        vec![
            (sid("s-1"), HumanActionReply::Done),
            (sid("s-1"), HumanActionReply::Cancel),
        ]
    );
    Ok(())
}
