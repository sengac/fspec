//! Shared fixtures and drivers for the BUG-576 scenarios.

use std::sync::Arc;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::{ModelSelectorEvent, ModelSelectorView};
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::{ModelEntry, ProviderInfo, SessionId, SessionInfo};
use codelet_test_helpers::{deadlock_guard, TestError};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::common::{MockBackend, TaskOutcome};

pub const LIST: &str = "list_providers";

pub fn model(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        ..ModelEntry::default()
    }
}

pub fn custom(id: &str) -> ModelEntry {
    ModelEntry {
        is_custom: true,
        ..model(id)
    }
}

/// A cloud section `key` listing `models`.
pub fn cloud(key: &str, display: &str, models: Vec<ModelEntry>) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: display.to_string(),
        models,
        provider_id: key.to_string(),
        ..ProviderInfo::default()
    }
}

/// The profile section `provider:profile` listing `models`.
pub fn profile(provider: &str, name: &str, models: Vec<ModelEntry>) -> ProviderInfo {
    ProviderInfo {
        key: format!("{provider}:{name}"),
        display_name: format!("{provider}: {name}"),
        models,
        profile_name: Some(name.to_string()),
        provider_id: provider.to_string(),
        ..ProviderInfo::default()
    }
}

pub fn mock(providers: Vec<ProviderInfo>) -> Arc<MockBackend> {
    let mock = Arc::new(MockBackend::new());
    mock.seed_providers(providers);
    mock
}

pub fn app(mock: &Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    App::new(backend)
}

pub fn session(id: &str) -> SessionId {
    SessionId::new(id)
}

/// The session `id`, listed with `provider` and `model`.
pub fn session_info(id: &str, provider: &str, model: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: id.to_string(),
        status: "idle".to_string(),
        project: String::new(),
        message_count: 0,
        provider_id: Some(provider.to_string()),
        model_id: Some(model.to_string()),
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: None,
        work_unit_focus: Default::default(),
    }
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

/// Press `code` and dispatch what it emits.
pub fn press(app: &mut App, code: KeyCode) {
    app.handle_event(&key(code));
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

pub fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
}

/// Dispatch arriving actions until `ready` holds.
pub async fn pump_until(
    app: &mut App,
    what: &str,
    mut ready: impl FnMut(&App) -> bool,
) -> Result<(), TestError> {
    deadlock_guard(what, async {
        loop {
            while let Some(action) = app.try_recv_action() {
                app.dispatch(action);
            }
            if ready(app) {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

/// Finish every task and dispatch everything (no gate may be holding).
pub async fn drain(app: &mut App) -> Result<(), TestError> {
    deadlock_guard("every task to finish", async {
        loop {
            while let Some(handle) = app.next_pending_task() {
                handle.await.completed_or_cancelled()?;
            }
            let Some(action) = app.try_recv_action() else {
                return Ok::<(), TestError>(());
            };
            app.dispatch(action);
        }
    })
    .await??;
    Ok(())
}

/// Open /model and let its list answer.
pub async fn open(app: &mut App) -> Result<(), TestError> {
    app.dispatch(Action::OpenModelSelectorView);
    drain(app).await
}

/// Make `id` the open session, its chrome loaded.
pub async fn open_session(app: &mut App, id: &str) -> Result<(), TestError> {
    app.dispatch(Action::SessionCreated(session(id)));
    app.dispatch(Action::OpenAgentView(Some(session(id))));
    drain(app).await
}

pub fn view(app: &App) -> &ModelSelectorView {
    &app.navigator().model_selector
}

/// The model under the cursor as `section/model`, read through the copy
/// key (`c`); `None` when the cursor is on a section header.
pub fn focused(app: &mut App) -> Option<String> {
    let event = app
        .navigator_mut()
        .model_selector
        .handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    match event {
        ModelSelectorEvent::Emit(action) => match *action {
            Action::CopyToClipboard(text) => Some(text),
            _ => None,
        },
        _ => None,
    }
}

/// The selector drawn alone at `width` x 30, as text lines and the buffer.
pub fn selector(app: &mut App, width: u16) -> (Vec<String>, Buffer) {
    let area = Rect::new(0, 0, width, 30);
    let mut buffer = Buffer::empty(area);
    app.navigator_mut().model_selector.render(area, &mut buffer);
    let lines = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    (lines, buffer)
}

/// The whole App screen as text (dialogs included).
pub fn screen(app: &mut App) -> String {
    let area = Rect::new(0, 0, 160, 40);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether any row showing `label` is drawn with the selected band.
pub fn drawn_selected(lines: &[String], buffer: &Buffer, label: &str) -> bool {
    lines.iter().enumerate().any(|(y, line)| {
        line.contains(label)
            && (0..buffer.area.width)
                .any(|x| u16::try_from(y).is_ok_and(|y| buffer[(x, y)].bg == Color::Cyan))
    })
}
