use crate::common::TaskOutcome;
use std::sync::Arc;

use codelet_fspec_tui::views::multiplex::{MuxLayout, MuxPaneKind};
use codelet_fspec_tui::{Action, App, ViewMode};
use codelet_rpc_types::{ModelEntry, ProviderInfo, SessionId, WorkUnitInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect};
use tempfile::TempDir;

use crate::common::MockBackend;

pub struct Fixture {
    pub app: App,
    pub mock: Arc<MockBackend>,
    _directory: TempDir,
}

impl Fixture {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_test_helpers::isolated_user_directory()?;
        codelet_common::set_data_directory(directory.path().to_path_buf())?;
        let mock = Arc::new(MockBackend::new());
        mock.seed_providers(vec![ProviderInfo {
            key: "openai".into(),
            display_name: "OpenAI".into(),
            models: vec![ModelEntry {
                id: "chosen-model".into(),
                display_name: "Chosen Model".into(),
                ..ModelEntry::default()
            }],
            ..ProviderInfo::default()
        }]);
        let mut app = App::new(mock.clone());
        app.dispatch(Action::WorkUnitsLoaded(vec![
            unit("CARD-1"),
            unit("CARD-2"),
        ]));
        app.board_store_mut().set_focused_column("implementing");
        app.board_store_mut()
            .set_selected_index_for("implementing", 1);
        Ok(Self {
            app,
            mock,
            _directory: directory,
        })
    }

    pub async fn session(&mut self) -> codelet_test_helpers::TestResult {
        self.app.dispatch(Action::SessionCreated(sid()));
        drain(&mut self.app).await?;
        Ok(())
    }

    pub async fn caller(&mut self, mode: ViewMode) -> codelet_test_helpers::TestResult {
        self.session().await?;
        match mode {
            ViewMode::Agent => self.app.dispatch(Action::OpenAgentView(Some(sid()))),
            ViewMode::Mux => {
                if let Some(mux_layout) = MuxLayout::columns(&[
                    MuxPaneKind::Board,
                    MuxPaneKind::Agent,
                    MuxPaneKind::ChangedFiles,
                    MuxPaneKind::Checkpoints,
                ]) {
                    self.app.navigator_mut().mux.set_layout(mux_layout);
                }
                if let Some(session) = self.app.current_session() {
                    self.app
                        .dispatch(Action::InputSubmitted(session, "/mux on".into()));
                }
            }
            _ => self.app.dispatch(Action::BackToBoard),
        }
        drain(&mut self.app).await?;
        assert_eq!(self.app.active_view(), mode);
        screen(&mut self.app);
        Ok(())
    }

    pub fn effects(&self) -> [usize; 5] {
        [
            self.mock.create_session_calls(),
            self.mock.resume_session_calls(),
            self.mock.set_work_unit_context_calls(),
            self.mock.set_session_model_calls(),
            self.mock.set_default_model_calls(),
        ]
    }
}

pub fn sid() -> SessionId {
    SessionId::new("agent-a")
}

fn unit(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.into(),
        title: id.into(),
        work_type: "bug".into(),
        status: "implementing".into(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

pub async fn drain(app: &mut App) -> codelet_test_helpers::TestResult {
    loop {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        let Some(action) = app.try_recv_action() else {
            break;
        };
        app.dispatch(action);
    }
    Ok(())
}

pub async fn key(app: &mut App, code: KeyCode) -> codelet_test_helpers::TestResult {
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    drain(app).await?;
    Ok(())
}

pub async fn open(
    app: &mut App,
    action: Action,
    mode: ViewMode,
) -> codelet_test_helpers::TestResult {
    app.dispatch(action);
    drain(app).await?;
    assert_eq!(app.active_view(), mode);
    Ok(())
}

pub fn screen(app: &mut App) -> String {
    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    buffer
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

pub fn auxiliary_actions() -> Vec<(Action, Action, ViewMode)> {
    vec![
        (
            Action::OpenProviderSettingsView,
            Action::CloseProviderSettingsView,
            ViewMode::ProviderSettings,
        ),
        (
            Action::OpenModelSelectorView,
            Action::CloseModelSelectorView,
            ViewMode::ModelSelector,
        ),
        (
            Action::OpenBlocklistView,
            Action::CloseBlocklistView,
            ViewMode::Blocklist,
        ),
        (
            Action::OpenChangedFilesView,
            Action::CloseChangedFilesView,
            ViewMode::ChangedFiles,
        ),
        (
            Action::OpenCheckpointsView,
            Action::CloseCheckpointsView,
            ViewMode::Checkpoints,
        ),
    ]
}
