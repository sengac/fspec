//! Feature: spec/features/transient-provider-waiting.feature

mod common;

use codelet_fspec_tui::store::{AgentViewStore, SessionContext};
use codelet_fspec_tui::views::agent::{pane_render::PaneSession, AgentView};
use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{
    SessionId, SessionState, SessionStatus, StreamChunk, ToolCallInfo, ToolProgressInfo,
    ToolResultInfo,
};
use ratatui::{buffer::Buffer, layout::Rect};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn session_id(name: &str) -> SessionId {
    SessionId::new(name)
}

fn harness() -> (AgentView, AgentViewStore) {
    let (sender, _receiver) = unbounded_channel();
    let mut store = AgentViewStore::default();
    for name in ["first", "second"] {
        let session = session_id(name);
        store.append_session(SessionContext::new(session.clone()));
        store.set_session_status(session, SessionStatus::Running);
    }
    store.focus_session_index(0);
    (AgentView::new(sender), store)
}

fn apply(store: &mut AgentViewStore, name: &str, chunk: &StreamChunk) -> TestResult {
    store
        .session_context_mut_for(&session_id(name))
        .ok_or("cannot apply chunk: harness session is missing")?
        .record_chunk(chunk);
    Ok(())
}

fn waiting(message: Option<&str>) -> Result<StreamChunk, serde_json::Error> {
    serde_json::from_value(json!({"ProviderWaiting": {"message": message}}))
}

fn render(view: &mut AgentView, store: &mut AgentViewStore, name: &str, focused: bool) -> String {
    let area = Rect::new(0, 0, 100, 18);
    let mut buffer = Buffer::empty(area);
    view.render_session_pane(
        area,
        &mut buffer,
        store,
        PaneSession {
            session: Some(session_id(name)),
            is_focused: focused,
            menu_row: false,
        },
    );
    (0..area.height)
        .map(|row| {
            (0..area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn transcript(store: &AgentViewStore, name: &str) -> TestResult<Vec<String>> {
    Ok(store
        .session_context_for(&session_id(name))
        .ok_or("cannot inspect transcript: harness session is missing")?
        .scrollback
        .visible_window(100)
        .into_iter()
        .filter_map(|row| row.source.as_ref().map(|source| source.text.clone()))
        .collect())
}

#[test]
fn waiting_changes_chrome_without_entering_the_transcript() -> TestResult {
    // @step Given a running session and a transient provider waiting update
    let (mut view, mut store) = harness();
    apply(
        &mut store,
        "first",
        &StreamChunk::text("Existing answer".into()),
    )?;
    let before = transcript(&store, "first")?;
    let update = waiting(Some("Codex buffering"))?;
    // @step When the waiting update is applied repeatedly and then cleared
    for _ in 0..3 {
        apply(&mut store, "first", &update)?;
    }
    apply(&mut store, "first", &StreamChunk::text(String::new()))?;
    apply(&mut store, "first", &StreamChunk::thinking(String::new()))?;
    assert!(store.selected_model_id_for(&session_id("first")).is_none());
    store.set_selected_model_id(session_id("first"), "codex/current".into());
    // @step Then the running chrome reflects only the current waiting message
    for focused in [true, false] {
        let first = render(&mut view, &mut store, "first", focused);
        assert!(first.contains("Codex buffering"), "{first}");
        assert!(first.contains("Esc to stop"));
        assert!(!render(&mut view, &mut store, "second", !focused).contains("Codex buffering"));
    }
    // @step And the transcript and other sessions remain unchanged
    assert_eq!(transcript(&store, "first")?, before);
    assert!(transcript(&store, "second")?.is_empty());
    assert_eq!(
        store.session_status_for(&session_id("first")),
        Some(&SessionStatus::Running)
    );
    store.set_selected_model_id(session_id("first"), "codex/current".into());
    assert!(render(&mut view, &mut store, "first", true).contains("Codex buffering"));
    store.set_selected_model_id(session_id("first"), "openai/other".into());
    assert!(!render(&mut view, &mut store, "first", true).contains("Codex buffering"));
    apply(&mut store, "first", &waiting(Some("Waiting for provider"))?)?;
    let replaced = render(&mut view, &mut store, "first", true);
    assert!(replaced.contains("Waiting for provider"));
    assert!(!replaced.contains("Codex buffering"));
    apply(&mut store, "first", &waiting(None)?)?;
    let cleared = render(&mut view, &mut store, "first", true);
    assert!(cleared.contains("Thinking"));
    assert!(!cleared.contains("Waiting for provider"));
    assert_eq!(transcript(&store, "first")?, before);
    apply(
        &mut store,
        "first",
        &waiting(Some("\u{1b}[31mSafe\nmessage\u{1b}[0m"))?,
    )?;
    let safe = render(&mut view, &mut store, "first", true);
    assert!(safe.contains("Safe message"));
    assert!(!safe.contains('\u{1b}'));
    apply(&mut store, "first", &waiting(Some(" \n\t "))?)?;
    assert!(render(&mut view, &mut store, "first", true).contains("Thinking"));
    Ok(())
}

#[test]
fn output_and_turn_boundaries_dismiss_waiting() -> TestResult {
    // @step Given a running session showing provider waiting
    let boundaries = [
        StreamChunk::text("Visible answer".into()),
        StreamChunk::thinking("Visible reasoning".into()),
        StreamChunk::tool_call(ToolCallInfo {
            id: "call".into(),
            name: "Bash".into(),
            input: "{}".into(),
        }),
        StreamChunk::tool_result(ToolResultInfo {
            tool_call_id: "call".into(),
            content: "result".into(),
            is_error: false,
        }),
        StreamChunk::tool_progress(ToolProgressInfo {
            tool_call_id: "call".into(),
            tool_name: "Bash".into(),
            output_chunk: "progress".into(),
            is_stderr: false,
        }),
        StreamChunk::Done,
        StreamChunk::error("Visible failure".into()),
        StreamChunk::interrupted(Vec::new()),
        StreamChunk::InterruptedWithReason {
            queued_inputs: Vec::new(),
            reason: codelet_rpc_types::InterruptionReason::User,
        },
        StreamChunk::user_input("Next question".into()),
        StreamChunk::session_state_change(SessionState::Idle),
        StreamChunk::session_state_change(SessionState::Interrupted),
        StreamChunk::session_state_change(SessionState::Cleared),
        StreamChunk::ResponseAttemptStarted,
        StreamChunk::ResponseAttemptAborted,
    ];
    for boundary in boundaries {
        let (mut view, mut store) = harness();
        apply(&mut store, "first", &waiting(Some("Codex buffering"))?)?;
        assert!(render(&mut view, &mut store, "first", true).contains("Codex buffering"));
        // @step When assistant output or a terminal or input boundary arrives
        apply(&mut store, "first", &boundary)?;
        // @step Then waiting is cleared without changing normal session behavior
        assert!(!render(&mut view, &mut store, "first", true).contains("Codex buffering"));
        store.set_session_status(session_id("first"), SessionStatus::Idle);
        // @step And no waiting message is captured in transcript or finish animations
        for _ in 0..4 {
            assert!(!render(&mut view, &mut store, "first", true).contains("Codex buffering"));
        }
        assert!(transcript(&store, "first")?
            .iter()
            .all(|text| !text.contains("Codex buffering")));
    }
    for status in [
        SessionStatus::Idle,
        SessionStatus::Interrupted,
        SessionStatus::Cleared,
        SessionStatus::Paused,
        SessionStatus::Compacting,
    ] {
        let (mut view, mut store) = harness();
        apply(&mut store, "first", &waiting(Some("Codex buffering"))?)?;
        render(&mut view, &mut store, "first", true);
        store.set_session_status(session_id("first"), status);
        let finished = render(&mut view, &mut store, "first", true);
        assert!(!finished.contains("Codex buffering"), "{finished}");
        store.set_session_status(session_id("first"), SessionStatus::Running);
        assert!(!render(&mut view, &mut store, "first", true).contains("Codex buffering"));
    }
    let (mut view, mut store) = harness();
    store.set_selected_model_id(session_id("first"), "codex/current".into());
    apply(&mut store, "first", &waiting(Some("Codex buffering"))?)?;
    store.set_selected_model_id(session_id("first"), "openai/other".into());
    assert!(!render(&mut view, &mut store, "first", true).contains("Codex buffering"));
    apply(&mut store, "first", &waiting(Some("Codex buffering"))?)?;
    store
        .session_context_mut_for(&session_id("first"))
        .ok_or("cannot reset scrollback: harness session is missing")?
        .reset_scrollback();
    assert!(!render(&mut view, &mut store, "first", true).contains("Codex buffering"));
    let mut app = App::new(Arc::new(common::MockBackend::new()));
    let session = session_id("live");
    app.dispatch(Action::SessionCreated(session.clone()));
    app.dispatch(Action::SessionStatusChanged(
        session.clone(),
        SessionStatus::Running,
    ));
    let (sender, _receiver) = unbounded_channel();
    let mut view = AgentView::new(sender);
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        waiting(Some("Codex buffering"))?,
    ));
    assert!(render(&mut view, app.agent_view_store_mut(), "live", true).contains("Codex buffering"));
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        StreamChunk::text("Live answer".into()),
    ));
    assert!(
        !render(&mut view, app.agent_view_store_mut(), "live", true).contains("Codex buffering")
    );
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        waiting(Some("Codex buffering"))?,
    ));
    app.dispatch(Action::SessionStatusChanged(
        session.clone(),
        SessionStatus::Idle,
    ));
    assert!(
        !render(&mut view, app.agent_view_store_mut(), "live", true).contains("Codex buffering")
    );
    app.dispatch(Action::SessionStatusChanged(
        session.clone(),
        SessionStatus::Running,
    ));
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        waiting(Some("Codex buffering"))?,
    ));
    app.dispatch(Action::ModelSelected(
        Some(session),
        "openai".into(),
        "other-model".into(),
    ));
    assert!(
        !render(&mut view, app.agent_view_store_mut(), "live", true).contains("Codex buffering")
    );
    let live = transcript(app.agent_view_store(), "live")?;
    assert!(live.iter().any(|text| text == "Live answer"));
    assert!(live.iter().all(|text| !text.contains("Codex buffering")));
    Ok(())
}
