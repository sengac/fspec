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
        codelet_common::set_data_directory(directory.path().to_path_buf())?;
        codelet_core::persistence::reset_stores_for_tests();
        let manager = Arc::new(SessionManager::new());
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
        self.manager.set_default_model("anthropic/claude-opus-4-5");
    }
}

impl Drop for StorageFixture {
    fn drop(&mut self) {
        self.server.abort();
        if let Some(directory) = &self.previous_directory {
            let _ = codelet_common::set_data_directory(directory.clone());
        }
        codelet_core::persistence::reset_stores_for_tests();
        match &self.previous_key {
            Some(key) => std::env::set_var("ANTHROPIC_API_KEY", key),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
    }
}

pub async fn drain(app: &mut App) {
    loop {
        while let Some(task) = app.next_pending_task() {
            let _ = task.await;
        }
        let Some(action) = app.try_recv_action() else {
            break;
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
    buffer.content().iter().map(|cell| cell.symbol()).collect()
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

pub fn creation_actions() -> Vec<Action> {
    vec![
        Action::CreateSessionSubmitted { isolated: false },
        Action::EnterWorkUnit("BUG-271".into()),
        Action::ModelSelected(None, "anthropic".into(), "claude-opus-4-5".into()),
    ]
}
