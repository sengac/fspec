//! Feature: spec/features/model-selector-view.feature
//!
//! BUG-508: the session header badge refresh after a model selection is
//! App-level behaviour (the selector only emits `Action::ModelSelected`;
//! the App writes the model through the backend and re-reads the model
//! info the SessionHeader paints). This drives the whole path through the
//! App: open the selector, press Enter on a model row, let the backend
//! accept it, then render the agent view and read its header row.

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{ModelEntry, ModelInfo, ProviderInfo, SessionId};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

mod common;
#[path = "common/model_backend.rs"]
#[allow(dead_code)]
mod model_backend;
use common::TaskOutcome;
use model_backend::ModelBackend;

fn origin() -> SessionId {
    SessionId::new("origin")
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Run spawned backend tasks and dispatch their follow-up actions until
/// nothing is pending.
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

/// Render the whole App at 120x24 and return its top row (the agent
/// view's SessionHeader row).
fn header_row(app: &mut App) -> Result<String, codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    let buf = term.backend().buffer();
    Ok((0..buf.area.width).map(|x| buf[(x, 0)].symbol()).collect())
}

fn anthropic_provider() -> ProviderInfo {
    ProviderInfo {
        key: "anthropic".into(),
        display_name: "Anthropic".into(),
        models: ["claude-haiku", "claude-sonnet"]
            .into_iter()
            .map(|model| ModelEntry {
                id: model.into(),
                display_name: model.into(),
                ..ModelEntry::default()
            })
            .collect(),
        ..ProviderInfo::default()
    }
}

/// Scenario: The session header badge shows the model chosen in the model selector
#[tokio::test]
async fn header_badge_shows_the_model_chosen_in_the_selector() -> TestResult {
    // @step Given an agent session whose header shows the model "claude-haiku"
    let backend = Arc::new(ModelBackend::with_providers(vec![anthropic_provider()]));
    let mut app = App::new(backend.clone());
    app.dispatch(Action::SessionCreated(origin()));
    app.dispatch(Action::OpenAgentView(Some(origin())));
    drain(&mut app).await?;
    app.agent_view_store_mut()
        .set_selected_model(origin(), ("anthropic".into(), "claude-haiku".into()));
    app.agent_view_store_mut().set_model_info(
        origin(),
        ModelInfo {
            display_name: "claude-haiku".into(),
            ..ModelInfo::default()
        },
    );
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    let before = header_row(&mut app)?;
    assert!(
        before.contains("claude-haiku") && !before.contains("claude-sonnet"),
        "precondition: the header shows claude-haiku; got: {before:?}"
    );

    // @step And the model selector is open with that session and the cursor on the model row "claude-sonnet"
    app.dispatch(Action::OpenModelSelectorView);
    drain(&mut app).await?;
    assert_eq!(app.navigator().active_view, ViewMode::ModelSelector);
    let selector = &app.navigator().model_selector;
    assert_eq!(selector.session(), Some(&origin()));
    // The list answered with anthropic_provider() when the selector opened.
    // The cursor seeds on the current model (claude-haiku); Down moves it
    // to the next model row.
    app.handle_event(&key(KeyCode::Down));

    // @step When I press Enter and the backend accepts the model
    app.handle_event(&key(KeyCode::Enter));
    drain(&mut app).await?;
    assert_eq!(
        backend.writes.lock().as_slice(),
        ["claude-sonnet".to_string()],
        "the backend received exactly the selected model"
    );

    // @step Then the model selector closes back to the agent view
    assert_eq!(app.navigator().active_view, ViewMode::Agent);

    // @step And the session header row shows "claude-sonnet"
    let after = header_row(&mut app)?;
    assert!(
        after.contains("claude-sonnet"),
        "the header must show the selected model; got: {after:?}"
    );

    // @step And the session header row no longer shows "claude-haiku"
    assert!(
        !after.contains("claude-haiku"),
        "the header must no longer show the previous model; got: {after:?}"
    );
    Ok(())
}
