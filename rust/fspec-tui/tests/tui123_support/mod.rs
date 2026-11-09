//! TUI-123 test support: an App over a real saved-daemons file plus local
//! daemons, with scripted connect outcomes, and key / mouse / screen helpers.
//!
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::common::TaskOutcome;
use async_trait::async_trait;
use codelet_common::saved_daemons;
use codelet_fspec_tui::daemon_connect::DaemonConnectError;
use codelet_fspec_tui::daemons::{DaemonChoice, DaemonDirectory, DaemonForm};
use codelet_fspec_tui::{App, FspecBackend};
use codelet_test_helpers::TestError;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::common::MockBackend;

/// What connecting to a daemon does.
#[derive(Clone)]
pub enum Outcome {
    Backend(Arc<MockBackend>),
    Refused,
    TokenRejected,
}

/// Local daemons plus the saved daemons in `path`.
pub struct FileDirectory {
    pub path: PathBuf,
    pub locals: Vec<(String, String)>,
    pub outcomes: HashMap<String, Outcome>,
    pub connects: Arc<AtomicUsize>,
}

#[async_trait]
impl DaemonDirectory for FileDirectory {
    fn list(&self) -> anyhow::Result<Vec<DaemonChoice>> {
        let mut choices: Vec<DaemonChoice> = self
            .locals
            .iter()
            .map(|(name, url)| DaemonChoice {
                name: name.clone(),
                url: url.clone(),
                saved: false,
            })
            .collect();
        choices.extend(
            saved_daemons::list(&self.path)?
                .into_iter()
                .map(|daemon| DaemonChoice {
                    name: daemon.name,
                    url: daemon.url,
                    saved: true,
                }),
        );
        Ok(choices)
    }

    async fn connect(&self, choice: &DaemonChoice) -> anyhow::Result<Arc<dyn FspecBackend>> {
        self.connects.fetch_add(1, Ordering::SeqCst);
        let url = choice.url.clone();
        match self.outcomes.get(&choice.name) {
            Some(Outcome::Backend(backend)) => Ok(backend.clone()),
            Some(Outcome::TokenRejected) => Err(DaemonConnectError::TokenRejected { url }.into()),
            Some(Outcome::Refused) | None => Err(DaemonConnectError::Refused { url }.into()),
        }
    }

    fn save(&self, previous: Option<&str>, form: &DaemonForm) -> anyhow::Result<()> {
        saved_daemons::update(
            &self.path,
            previous,
            &form.name,
            &form.url,
            form.token.clone(),
        )?;
        Ok(())
    }

    fn remove(&self, name: &str) -> anyhow::Result<()> {
        if !saved_daemons::remove(&self.path, name)? {
            anyhow::bail!("no saved daemon named {name}");
        }
        Ok(())
    }
}

pub struct Fixture {
    pub app: App,
    pub home: tempfile::TempDir,
    pub path: PathBuf,
    pub backend: Arc<MockBackend>,
    pub connects: Arc<AtomicUsize>,
}

/// An App connected to `connected`, local daemon "alpha" and the saved
/// daemons already in `<home>/.fspec/daemons.json`.
pub struct Setup {
    pub connected: String,
    pub outcomes: HashMap<String, Outcome>,
    pub saved: Vec<saved_daemons::SavedDaemon>,
    pub raw_file: Option<String>,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            connected: "alpha".into(),
            outcomes: HashMap::new(),
            saved: Vec::new(),
            raw_file: None,
        }
    }
}

pub async fn fixture(setup: Setup) -> Result<Fixture, Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let path = home.path().join(".fspec/daemons.json");
    for daemon in setup.saved {
        saved_daemons::save(&path, daemon)?;
    }
    if let Some(raw) = setup.raw_file {
        std::fs::create_dir_all(home.path().join(".fspec"))?;
        std::fs::write(&path, raw)?;
    }
    let backend = Arc::new(MockBackend::new());
    backend.seed_workspaces(vec![codelet_rpc_types::WorkspaceEntry {
        path: "/work/start".into(),
        name: "start".into(),
        start: true,
    }]);
    let dyn_backend: Arc<dyn FspecBackend> = backend.clone();
    let mut app = App::new(dyn_backend);
    let connects = Arc::new(AtomicUsize::new(0));
    let directory = FileDirectory {
        path: path.clone(),
        locals: vec![("alpha".into(), "ws://127.0.0.1:4001".into())],
        outcomes: setup.outcomes,
        connects: connects.clone(),
    };
    app.set_daemon_directory(Arc::new(directory), setup.connected);
    app.bootstrap().await?;
    settle(&mut app).await?;
    Ok(Fixture {
        app,
        home,
        path,
        backend,
        connects,
    })
}

pub fn saved(name: &str, url: &str, token: Option<&str>) -> saved_daemons::SavedDaemon {
    saved_daemons::SavedDaemon {
        name: name.into(),
        url: url.into(),
        token: token.map(str::to_string),
        ca: None,
    }
}

pub async fn settle(app: &mut App) -> Result<(), TestError> {
    for _ in 0..5 {
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            while let Some(handle) = app.next_pending_task() {
                handle.await.completed_or_cancelled()?;
            }
        }
        // TEST-042: scheduler rounds, not a time window: on the
        // current-thread test runtime every ready task (a spawned backend
        // call, a subscriber forwarding a push) runs before this resumes.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}

pub fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

pub async fn press(app: &mut App, code: KeyCode) -> Result<(), TestError> {
    app.handle_event(&key(code));
    settle(app).await
}

pub async fn type_text(app: &mut App, text: &str) -> Result<(), TestError> {
    for character in text.chars() {
        app.handle_event(&key(KeyCode::Char(character)));
    }
    settle(app).await
}

pub async fn backspace(app: &mut App, times: usize) -> Result<(), TestError> {
    for _ in 0..times {
        app.handle_event(&key(KeyCode::Backspace));
    }
    settle(app).await
}

pub async fn click(app: &mut App, column: u16, row: u16) -> Result<(), TestError> {
    app.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }));
    settle(app).await
}

pub fn render(app: &mut App, width: u16, height: u16) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    buffer
}

pub fn row(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect()
}

pub fn text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first row containing `needle`.
pub fn find_row(buf: &Buffer, needle: &str) -> Option<u16> {
    (0..buf.area.height).find(|&y| row(buf, y).contains(needle))
}

/// The column where `needle` starts on row `y` (cells, not bytes).
pub fn find_column(buf: &Buffer, y: u16, needle: &str) -> Option<u16> {
    let cells: Vec<String> = (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect();
    (0..cells.len())
        .find(|&start| cells[start..].concat().starts_with(needle))
        .and_then(|start| u16::try_from(start).ok())
}

/// Header row 1 (workspace and daemon labels): the row after the
/// checkpoint status row.
pub fn header_row(buf: &Buffer) -> Option<u16> {
    find_row(buf, "Checkpoints:").map(|y| y + 1)
}

/// Open the switcher and fill the add form: name, Tab, URL, Tab, token.
pub async fn fill_add_form(
    app: &mut App,
    name: &str,
    url: &str,
    token: &str,
) -> Result<(), TestError> {
    press(app, KeyCode::Char('s')).await?;
    press(app, KeyCode::Char('a')).await?;
    type_text(app, name).await?;
    press(app, KeyCode::Tab).await?;
    type_text(app, url).await?;
    press(app, KeyCode::Tab).await?;
    type_text(app, token).await
}
