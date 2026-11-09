use crate::common::TaskOutcome;
use std::path::PathBuf;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_sessions::SessionManager;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tempfile::TempDir;
use tokio::task::JoinHandle;

pub struct StorageFixture {
    pub directory: TempDir,
    pub manager: Arc<SessionManager>,
    pub backends: Vec<Arc<dyn FspecBackend>>,
    /// Transport name of each entry in `backends`, for assertion messages.
    pub labels: Vec<&'static str>,
    previous_directory: Option<PathBuf>,
    previous_key: Option<std::ffi::OsString>,
    server: JoinHandle<()>,
}

impl StorageFixture {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let previous_directory = codelet_common::get_data_dir().ok();
        let previous_key = std::env::var_os("ANTHROPIC_API_KEY");
        std::env::set_var("ANTHROPIC_API_KEY", "test-offline-session-storage");
        std::fs::create_dir_all(directory.path().join("cache"))?;
        std::fs::write(
            directory.path().join("cache/models.json"),
            include_str!("../fixtures/prov101_models.json"),
        )?;
        std::fs::create_dir_all(directory.path().join("spec"))?;
        std::fs::write(
            directory.path().join("spec/work-units.json"),
            "{\"workUnits\":{}}",
        )?;
        codelet_test_helpers::isolated_user_directory()?;
        codelet_common::set_data_directory(directory.path().to_path_buf())?;
        codelet_core::persistence::reset_stores_for_tests();
        let manager = SessionManager::new();
        // BUG-437: sessions are created in the manager's start workspace.
        manager.set_default_project(directory.path().to_path_buf())?;
        let watcher = Arc::new(WorkUnitsWatcher::new(directory.path())?);
        let service = Arc::new(SharedFspecService::with_session_manager(
            watcher,
            manager.clone(),
        ));
        let embedded = Arc::new(EmbeddedFspecBackend::new(
            tokio::runtime::Handle::current(),
            service.clone(),
        ));
        let (address, _, server) =
            codelet_rpc_server::bind_and_serve("127.0.0.1:0", service).await?;
        let websocket = Arc::new(
            WebSocketFspecBackend::connect(url::Url::parse(&format!("ws://{address}/"))?).await?,
        );
        Ok(Self {
            directory,
            manager,
            backends: vec![embedded, websocket],
            labels: vec!["embedded", "websocket"],
            previous_directory,
            previous_key,
            server,
        })
    }

    pub fn bad_manifest(&self, id: &str) -> Result<PathBuf, std::io::Error> {
        let sessions = self.directory.path().join("sessions");
        std::fs::create_dir_all(&sessions)?;
        let path = sessions.join(format!("{id}.json"));
        std::fs::write(&path, b"")?;
        Ok(path)
    }

    pub fn select_model(&self) {
        assert_eq!(
            self.manager.set_default_model("anthropic/claude-opus-4-5"),
            Ok(())
        );
    }
}

impl Drop for StorageFixture {
    fn drop(&mut self) {
        self.server.abort();
        if let Some(directory) = &self.previous_directory {
            let isolated = codelet_test_helpers::isolated_user_directory();
            assert!(
                isolated.is_ok(),
                "BUG-640: isolate the user directory: {isolated:?}"
            );
            if let Err(error) = codelet_common::set_data_directory(directory.clone()) {
                tracing::error!(%error, "session storage fixture could not restore the data directory");
            }
        }
        codelet_core::persistence::reset_stores_for_tests();
        match &self.previous_key {
            Some(key) => std::env::set_var("ANTHROPIC_API_KEY", key),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
    }
}

/// Run every spawned app task and dispatch every queued action until both
/// are empty. A panicked task is returned as an error; a task the App
/// cancelled is finished.
pub async fn drain(app: &mut App) -> Result<(), codelet_test_helpers::Failure> {
    loop {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        let Some(action) = app.try_recv_action() else {
            return Ok(());
        };
        app.dispatch(action);
    }
}

pub fn key(app: &mut App, code: KeyCode) {
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

pub fn screen(app: &mut App) -> String {
    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    buffer
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

pub fn pages(app: &mut App) -> String {
    let mut output = String::new();
    for _page in 0..64 {
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        app.render(area, &mut buffer);
        // Read actual body cells, excluding the title/footer/border and the
        // reserved scrollbar column. Chrome is not part of wrapped path text.
        let rect = codelet_fspec_tui::components::help_dialog_scroll::fill_rect(area);
        let visible = codelet_fspec_tui::components::dialog_theme_rows::body_content_rows(
            rect.height,
            1,
            false,
        )
        .max(1);
        if let Some(gutter) =
            codelet_fspec_tui::components::help_dialog_scroll::gutter_rect(rect, visible)
        {
            for y in gutter.y..gutter.y + gutter.height {
                for x in rect.x + 2..gutter.x {
                    output.push_str(buffer[(x, y)].symbol());
                }
            }
        }
        key(app, KeyCode::Down);
    }
    output
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

pub fn compact(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace() && !"│╭╮╰╯─".contains(*character))
        .collect()
}

/// One user input in a creation path.
pub enum Step {
    Dispatch(Box<Action>),
    Press(KeyCode),
}

/// Each user creation path as its inputs. Entering a board card opens the
/// creation dialog for that card (626eee1ec); confirming the dialog with
/// Enter is what creates the session.
pub fn creation_flows() -> Vec<(&'static str, Vec<Step>)> {
    vec![
        (
            "dialog",
            vec![Step::Dispatch(Box::new(Action::CreateSessionSubmitted {
                isolated: false,
            }))],
        ),
        (
            "work unit",
            vec![
                Step::Dispatch(Box::new(Action::EnterWorkUnit(BOARD_CARD.into()))),
                Step::Press(KeyCode::Enter),
            ],
        ),
        (
            "model selection",
            vec![Step::Dispatch(Box::new(Action::ModelSelected(
                None,
                "anthropic".into(),
                "claude-opus-4-5".into(),
            )))],
        ),
    ]
}

/// Apply `step` to `app` and run everything it spawns.
pub async fn perform(app: &mut App, step: Step) -> Result<(), codelet_test_helpers::Failure> {
    match step {
        Step::Dispatch(action) => app.dispatch(*action),
        Step::Press(code) => key(app, code),
    }
    drain(app).await
}

/// The board card the work-unit creation path enters.
pub const BOARD_CARD: &str = "BUG-271";

/// An app on `backend` whose board shows `BOARD_CARD`.
pub fn board_app(backend: Arc<dyn FspecBackend>) -> App {
    let mut app = App::new(backend);
    app.dispatch(Action::WorkUnitsLoaded(vec![
        codelet_rpc_types::WorkUnitInfo {
            id: BOARD_CARD.into(),
            title: "Session storage diagnostics".into(),
            status: "implementing".into(),
            work_type: "bug".into(),
            description: None,
            estimate: None,
            epic: None,
            attachments: Vec::new(),
            last_state_change_at: None,
        },
    ]));
    app
}
