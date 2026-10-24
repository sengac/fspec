//! Feature: spec/features/accepted-model-selection.feature

use std::sync::{atomic::Ordering, Arc};

use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{ModelEntry, ModelInfo, ProviderInfo, SessionId};

mod common;
#[path = "common/model_backend.rs"]
mod model_backend;
use model_backend::{responses, settle, ModelBackend};

fn origin() -> SessionId {
    SessionId::new("origin")
}

#[tokio::test]
async fn a_later_rejection_retains_an_earlier_accepted_pending_selection() {
    // @step Given a session displays accepted model A
    let (mut app, _) = fixture().await;
    let view = &mut app.navigator_mut().model_selector;
    view.set_session(Some(origin()));
    view.set_current_model(Some("A".into()));
    view.set_providers(vec![ProviderInfo {
        key: "openai".into(),
        display_name: "OpenAI".into(),
        models: ["A", "B"]
            .into_iter()
            .map(|model| ModelEntry {
                id: model.into(),
                display_name: format!("model {model}"),
                ..ModelEntry::default()
            })
            .collect(),
        ..ProviderInfo::default()
    }]);
    // @step When model B is accepted and a subsequent model C request is rejected
    select(&mut app, "B");
    select(&mut app, "reject");
    let mut actions = responses(&mut app).await;
    actions.reverse();
    for action in actions {
        app.dispatch(action);
    }
    // @step Then reversed response delivery retains accepted model B and shows the rejection
    assert_model(&app, "B");
    let area = ratatui::layout::Rect::new(0, 0, 100, 15);
    let mut buffer = ratatui::buffer::Buffer::empty(area);
    app.navigator_mut().model_selector.render(area, &mut buffer);
    let lines: Vec<String> = (0..area.height)
        .map(|row| {
            (0..area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect()
        })
        .collect();
    assert!(lines
        .iter()
        .any(|line| line.contains("model B") && line.contains("(current)")));
    assert!(!lines
        .iter()
        .any(|line| line.contains("model A") && line.contains("(current)")));
    assert!(notice(&app, &origin(), "session busy"));
}

fn seed(app: &mut App) {
    app.agent_view_store_mut()
        .set_selected_model_id(origin(), "A".into());
    app.agent_view_store_mut().set_model_info(
        origin(),
        ModelInfo {
            display_name: "A".into(),
            ..ModelInfo::default()
        },
    );
}

async fn fixture() -> (App, Arc<ModelBackend>) {
    let backend = Arc::new(ModelBackend::new());
    let mut app = App::new(backend.clone());
    app.dispatch(Action::SessionCreated(origin()));
    settle(&mut app).await;
    seed(&mut app);
    (app, backend)
}

fn select(app: &mut App, model: &str) {
    app.dispatch(Action::ModelSelected(
        Some(origin()),
        "openai".into(),
        model.into(),
    ));
}

fn assert_model(app: &App, model: &str) {
    assert_eq!(
        app.agent_view_store().selected_model_id_for(&origin()),
        Some(model)
    );
    assert_eq!(
        app.agent_view_store()
            .model_info_for(&origin())
            .map(|info| info.display_name.as_str()),
        Some(model)
    );
}

fn notice(app: &App, session: &SessionId, text: &str) -> bool {
    app.agent_view_store()
        .session_context_for(session)
        .is_some_and(|context| {
            context.scrollback.chunks().iter().any(|chunk| {
                chunk
                    .source
                    .as_ref()
                    .is_some_and(|source| source.text.contains(text))
            })
        })
}

#[tokio::test]
async fn pending_and_rejected_selection_preserves_accepted_identity() {
    // @step Given a session displays accepted model A
    let (mut app, _) = fixture().await;
    // @step When model B is requested and rejected as busy
    select(&mut app, "reject");
    // @step Then model A remains selected before and after completion
    assert_model(&app, "A");
    settle(&mut app).await;
    assert_model(&app, "A");
    // @step And the session shows the rejection reason
    assert!(notice(&app, &origin(), "session busy"));
}

#[tokio::test]
async fn successful_selection_publishes_backend_identity_rather_than_request_alias() {
    // @step Given a session displays accepted model A
    let (mut app, _) = fixture().await;
    // @step When a model alias is accepted as canonical model B
    select(&mut app, "alias");
    settle(&mut app).await;
    // @step Then the selector and header display canonical model B
    assert_model(&app, "B");
}

#[tokio::test]
async fn selection_feedback_stays_with_its_originating_session() {
    // @step Given a session displays accepted model A
    let (mut app, _) = fixture().await;
    // @step When its rejected selection completes after focus moves to another session
    select(&mut app, "reject");
    app.dispatch(Action::SessionCreated(SessionId::new("other")));
    app.agent_view_store_mut()
        .set_selected_model_id(SessionId::new("other"), "other-model".into());
    settle(&mut app).await;
    // @step Then only the originating session receives the rejection notice
    assert!(notice(&app, &origin(), "session busy"));
    assert!(!notice(&app, &SessionId::new("other"), "session busy"));
    // @step And the other session identity is unchanged
    assert_eq!(
        app.agent_view_store()
            .selected_model_id_for(&SessionId::new("other")),
        Some("other-model")
    );
}

#[tokio::test]
async fn delayed_earlier_selection_cannot_overwrite_a_newer_request() {
    // @step Given a session displays accepted model A
    let (mut app, backend) = fixture().await;
    // @step When model B is delayed and model C is requested next
    backend.hold_write.store(true, Ordering::SeqCst);
    select(&mut app, "B");
    backend.wait_started().await;
    select(&mut app, "C");
    for _attempt in 0..100 {
        tokio::task::yield_now().await;
    }
    let writes_before_release = backend
        .writes
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .len();
    backend.release.add_permits(1);
    let mut actions = responses(&mut app).await;
    // @step Then backend writes complete in request order
    assert_eq!(writes_before_release, 1);
    // @step And reversed response delivery still displays model C
    actions.reverse();
    for action in actions {
        app.dispatch(action);
    }
    assert_model(&app, "C");
}

#[tokio::test]
async fn earlier_chrome_hydration_cannot_overwrite_an_accepted_selection() {
    // @step Given an earlier model A chrome read is delayed
    let (mut app, backend) = fixture().await;
    backend.hold_read.store(true, Ordering::SeqCst);
    app.dispatch(Action::ReattachSession(origin()));
    backend.wait_started().await;
    // @step When model B is accepted before that read is delivered
    select(&mut app, "B");
    backend.release.add_permits(1);
    let mut actions = responses(&mut app).await;
    actions.reverse();
    for action in actions {
        app.dispatch(action);
    }
    // @step Then delayed hydration does not replace model B
    assert_model(&app, "B");
}

#[tokio::test]
async fn reconciliation_failure_retains_accepted_display_and_surfaces_feedback() {
    // @step Given a session displays accepted model A
    let (mut app, backend) = fixture().await;
    // @step When a model switch fails and backend metadata cannot be refreshed
    backend.fail_read.store(true, Ordering::SeqCst);
    select(&mut app, "reject");
    settle(&mut app).await;
    // @step Then model A remains selected and the session shows the failure
    assert_model(&app, "A");
    assert!(notice(&app, &origin(), "session busy"));
    assert!(notice(&app, &origin(), "metadata unavailable"));
}

#[test]
fn selection_without_a_runtime_does_not_publish_an_unaccepted_model() {
    // @step Given a session displays accepted model A without an async runtime
    let mut app = App::new(Arc::new(ModelBackend::new()));
    app.dispatch(Action::SessionCreated(origin()));
    seed(&mut app);
    // @step When model B is requested
    select(&mut app, "B");
    // @step Then model A remains selected and the session shows an unavailable notice
    assert_model(&app, "A");
    assert!(notice(&app, &origin(), "unavailable"));
}
