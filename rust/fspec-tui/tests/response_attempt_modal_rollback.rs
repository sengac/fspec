#![doc = "Feature: spec/features/background-response-attempt-rollback.feature"]

use std::sync::Arc;

use codelet_fspec_tui::{components::dialog_theme_rows::fixed_dialog_rect, Action, App, ViewMode};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolResultInfo};
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{buffer::Buffer, layout::Rect};

mod common;

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

fn key(code: KeyCode) -> Event {
    Event::Key(common::harness::AppTestHarness::key(code))
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn render_app(app: &mut App) -> String {
    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    buffer
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

fn drain_app(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// @step Given a completed tool result followed by a live response in its assistant placeholder
/// @step And a turn modal with selected text from that live response
/// @step When the response attempt is aborted
/// @step Then the empty assistant placeholder is restored
/// @step And the modal and its text selection are dismissed
/// @step And scrollback keyboard navigation resumes without pressing Escape
#[test]
fn aborting_inspected_output_in_a_reused_streaming_block_closes_its_modal() {
    let mut app = app();
    let session = sid("focused");
    emit(
        &mut app,
        &session,
        StreamChunk::tool_call(ToolCallInfo {
            id: "tool".into(),
            name: "Read".into(),
            input: "{}".into(),
        }),
    );
    emit(
        &mut app,
        &session,
        StreamChunk::tool_result(ToolResultInfo {
            tool_call_id: "tool".into(),
            content: "completed tool result".into(),
            is_error: false,
        }),
    );
    let placeholder_seq = app
        .agent_view_store()
        .current_session_context()
        .and_then(|ctx| ctx.scrollback.chunks().last().map(|chunk| chunk.seq));
    assert!(placeholder_seq.is_some());
    emit(&mut app, &session, StreamChunk::ResponseAttemptStarted);
    emit(
        &mut app,
        &session,
        StreamChunk::text("abandoned output".into()),
    );
    open_modal(&mut app);
    select_modal_text(&mut app);
    assert_eq!(app.navigator().agent.turn_modal_seq, placeholder_seq);

    emit(&mut app, &session, StreamChunk::ResponseAttemptAborted);

    let placeholder_text = app
        .agent_view_store()
        .current_session_context()
        .and_then(|ctx| placeholder_seq.and_then(|seq| ctx.scrollback.full_text_for_seq(seq)));
    assert_eq!(placeholder_text.as_deref(), Some(""));
    assert_eq!(app.navigator().agent.turn_modal_seq, None);
    assert_eq!(app.navigator().agent.turn_modal_selection, None);
    assert_eq!(app.navigator().agent.turn_modal_offset, 0);
    assert!(app.navigator().agent.turn_select_mode);
    let _ = render_app(&mut app);
    let _ = app.handle_event(&key(KeyCode::Up));
    let actions: Vec<_> = std::iter::from_fn(|| app.try_recv_action()).collect();
    assert!(actions
        .iter()
        .any(|action| matches!(action, Action::TurnNavUp)));
    for action in actions {
        app.dispatch(action);
    }
    let selected_text = app
        .agent_view_store()
        .current_session_context()
        .and_then(|ctx| {
            ctx.scrollback
                .selected_seq()
                .and_then(|seq| ctx.scrollback.full_text_for_seq(seq))
        });
    assert!(selected_text.is_some_and(|text| text.contains("completed tool result")));
}

fn app() -> App {
    let mut app = App::new(Arc::new(common::MockBackend::new()));
    app.dispatch(Action::SessionCreated(sid("focused")));
    app.navigator_mut().active_view = ViewMode::Agent;
    app
}

fn emit(app: &mut App, session: &SessionId, chunk: StreamChunk) {
    app.dispatch(Action::ChunkReceived(session.clone(), chunk));
    drain_app(app);
}

fn open_modal(app: &mut App) {
    let _ = app.handle_event(&key(KeyCode::Tab));
    drain_app(app);
    let _ = app.handle_event(&key(KeyCode::Enter));
    drain_app(app);
    let _ = render_app(app);
}

fn select_modal_text(app: &mut App) {
    let rect = fixed_dialog_rect(Rect::new(0, 0, 80, 24));
    for event in [
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            rect.x + 2,
            rect.y + 4,
        ),
        mouse(
            MouseEventKind::Drag(MouseButton::Left),
            rect.x + 6,
            rect.y + 4,
        ),
    ] {
        let _ = app.handle_event(&event);
        drain_app(app);
    }
    assert!(app.navigator().agent.turn_modal_selection.is_some());
}

/// @step Given earlier completed output and an open turn modal showing a live response attempt
/// @step When that response attempt is aborted
/// @step Then the modal and its text selection are dismissed
/// @step And scrollback keyboard and mouse navigation work without pressing Escape
/// @step And replacement output can be inspected normally
#[test]
fn aborting_the_inspected_attempt_closes_its_turn_modal() {
    let mut app = app();
    let session = sid("focused");
    emit(
        &mut app,
        &session,
        StreamChunk::text("earlier output\n".repeat(40)),
    );
    emit(&mut app, &session, StreamChunk::Done);
    emit(&mut app, &session, StreamChunk::ResponseAttemptStarted);
    let abandoned = "abandoned output\n".repeat(40);
    emit(&mut app, &session, StreamChunk::text(abandoned));
    open_modal(&mut app);
    let abandoned_seq = app.navigator().agent.turn_modal_seq;
    assert!(abandoned_seq.is_some());
    app.dispatch(Action::TurnModalScrollDown);
    assert!(app.navigator().agent.turn_modal_offset > 0);
    let _ = render_app(&mut app);
    select_modal_text(&mut app);

    emit(&mut app, &session, StreamChunk::ResponseAttemptAborted);

    assert_eq!(app.navigator().agent.turn_modal_seq, None);
    assert_eq!(app.navigator().agent.turn_modal_selection, None);
    assert_eq!(app.navigator().agent.turn_modal_offset, 0);
    assert!(app.navigator().agent.turn_select_mode);
    let rendered = render_app(&mut app);
    assert!(!rendered.contains("abandoned"));
    let _ = app.handle_event(&key(KeyCode::Up));
    let actions: Vec<_> = std::iter::from_fn(|| app.try_recv_action()).collect();
    assert!(actions
        .iter()
        .any(|action| matches!(action, Action::TurnNavUp)));
    for action in actions {
        app.dispatch(action);
    }
    assert!(app
        .agent_view_store()
        .current_session_context()
        .is_some_and(|ctx| ctx.scrollback.selected_seq().is_some()));
    let _ = app.handle_event(&mouse(MouseEventKind::ScrollDown, 4, 5));
    drain_app(&mut app);
    let before = app
        .agent_view_store()
        .current_session_context()
        .map(|ctx| ctx.scrollback.scroll_state().offset);
    assert!(before.is_some_and(|offset| offset > 0));
    let _ = app.handle_event(&mouse(MouseEventKind::ScrollUp, 4, 5));
    drain_app(&mut app);
    let after = app
        .agent_view_store()
        .current_session_context()
        .map(|ctx| ctx.scrollback.scroll_state().offset);
    assert!(
        after
            .zip(before)
            .is_some_and(|(after, before)| after < before),
        "mouse wheel must scroll the surviving output"
    );

    emit(&mut app, &session, StreamChunk::ResponseAttemptStarted);
    emit(
        &mut app,
        &session,
        StreamChunk::text("replacement output".into()),
    );
    emit(&mut app, &session, StreamChunk::Done);
    let _ = app.handle_event(&key(KeyCode::Tab));
    drain_app(&mut app);
    open_modal(&mut app);
    assert!(app.navigator().agent.turn_modal_seq.is_some());
    assert_ne!(app.navigator().agent.turn_modal_seq, abandoned_seq);
    assert!(render_app(&mut app).contains("replacement output"));
    let _ = app.handle_event(&key(KeyCode::Esc));
    drain_app(&mut app);
    assert_eq!(app.navigator().agent.turn_modal_seq, None);
}

/// @step Given a turn modal showing completed output
/// @step When another response attempt is aborted in either the focused or a background session
/// @step Then the inspected output remains open and unchanged
#[test]
fn rollback_preserves_a_modal_whose_inspected_output_remains() {
    for target in ["focused", "background"] {
        let mut app = app();
        let focused = sid("focused");
        emit(
            &mut app,
            &focused,
            StreamChunk::text("completed output".into()),
        );
        emit(&mut app, &focused, StreamChunk::Done);
        if target == "background" {
            app.dispatch(Action::SessionCreated(sid(target)));
            app.agent_view_store_mut().focus_session_index(0);
        }
        open_modal(&mut app);
        select_modal_text(&mut app);
        let modal_seq = app.navigator().agent.turn_modal_seq;
        let selection = app.navigator().agent.turn_modal_selection;
        let target = sid(target);
        emit(&mut app, &target, StreamChunk::ResponseAttemptStarted);
        emit(
            &mut app,
            &target,
            StreamChunk::text("abandoned output".into()),
        );

        emit(&mut app, &target, StreamChunk::ResponseAttemptAborted);

        assert_eq!(app.navigator().agent.turn_modal_seq, modal_seq);
        assert_eq!(app.navigator().agent.turn_modal_selection, selection);
        assert!(render_app(&mut app).contains("completed output"));
    }
}
