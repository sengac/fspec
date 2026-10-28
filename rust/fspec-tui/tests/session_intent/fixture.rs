use std::{ffi::OsString, path::PathBuf};

use codelet_fspec_tui::{MuxConfig, CREATE_SESSION_DIALOG_ID};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect};

use super::*;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct Isolation {
    _root: tempfile::TempDir,
    previous: Vec<(&'static str, Option<OsString>)>,
    data: Option<PathBuf>,
}

impl Isolation {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let root = codelet_test_helpers::setup_temp_directory()?;
        let keys = [
            "HOME",
            "FSPEC_USER_DIR",
            "FSPEC_HOME",
            "CODEX_HOME",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
        ];
        let previous = keys
            .iter()
            .map(|key| (*key, std::env::var_os(key)))
            .collect();
        for key in keys {
            let path = root.path().join(key);
            std::fs::create_dir_all(&path)?;
            std::env::set_var(key, path);
        }
        let data = codelet_common::get_data_dir().ok();
        codelet_common::set_data_directory(root.path().join("data"))?;
        Ok(Self {
            _root: root,
            previous,
            data,
        })
    }
}

impl Drop for Isolation {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        if let Some(data) = self.data.take() {
            let _ = codelet_common::set_data_directory(data);
        }
    }
}

pub fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

pub fn card(value: &str) -> WorkUnitContext {
    WorkUnitContext {
        id: value.into(),
        title: value.into(),
        status: "implementing".into(),
    }
}

pub fn app(backend: Arc<common::MockBackend>) -> App {
    let mut app = App::new(backend);
    app.dispatch(Action::WorkUnitsLoaded(
        ["AUTH-001", "AUTH-002"]
            .map(|value| codelet_rpc_types::WorkUnitInfo {
                id: value.into(),
                title: value.into(),
                status: "implementing".into(),
                work_type: "story".into(),
                description: None,
                estimate: None,
                epic: None,
                attachments: Vec::new(),
                last_state_change_at: None,
            })
            .to_vec(),
    ));
    app.agent_view_store_mut()
        .set_current_work_unit(Some("AUTH-001".into()), Some("implementing".into()));
    app
}

pub async fn add(
    app: &mut App,
    backend: &common::MockBackend,
    value: &str,
    story: Option<&str>,
) -> TestResult {
    backend
        .set_work_unit_context(sid(value), story.map(card))
        .await?;
    app.dispatch(Action::SessionObserved(snapshot(backend, value).await?));
    settle(app).await;
    Ok(())
}

pub async fn snapshot(
    backend: &common::MockBackend,
    value: &str,
) -> Result<codelet_rpc_types::SessionInfo, Box<dyn std::error::Error>> {
    backend.seed_session_snapshot(vec![sid(value)]);
    let mut info = backend
        .session_snapshot()
        .await
        .into_iter()
        .next()
        .ok_or("missing synthetic session")?;
    if let Ok(focus) = backend.get_work_unit_focus(sid(value)).await {
        info.work_unit_focus = focus;
    }
    Ok(info)
}

pub async fn settle(app: &mut App) {
    loop {
        while let Some(task) = app.next_pending_task() {
            assert!(task.await.is_ok());
        }
        match app.try_recv_action() {
            Some(action) => app.dispatch(action),
            None => break,
        }
    }
}

pub fn text(app: &mut App) -> String {
    let area = Rect::new(0, 0, 120, 35);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

pub fn focus(app: &App, value: &str) -> Option<String> {
    app.agent_view_store()
        .work_unit_context_for(&sid(value))
        .map(|context| context.id.clone())
}

pub fn mux(app: &mut App) {
    let sessions = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|context| context.id.clone())
        .collect::<Vec<_>>();
    app.navigator_mut().mux.enable_with_config(
        MuxConfig {
            enabled: true,
            ..Default::default()
        },
        ViewMode::Board,
    );
    app.navigator_mut().mux.sync_window(&sessions);
    app.navigator_mut().active_view = ViewMode::Mux;
}

pub fn enter(app: &mut App, in_mux: bool) {
    if in_mux {
        mux(app);
        app.dispatch(Action::MuxEnterWorkUnit("AUTH-002".into()));
    } else {
        app.navigator_mut().active_view = ViewMode::Board;
        app.dispatch(Action::EnterWorkUnit("AUTH-002".into()));
    }
}

pub async fn confirm(app: &mut App, isolated: bool) {
    if isolated {
        let _ = app.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Right,
            KeyModifiers::NONE,
        )));
    }
    let _ = app.handle_event(&Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    )));
    settle(app).await;
}

pub async fn cancel(app: &mut App) {
    let _ = app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    settle(app).await;
}

pub fn dialog(app: &App) -> bool {
    app.compositor().contains(CREATE_SESSION_DIALOG_ID)
}

pub fn open_generic(app: &mut App) {
    app.dispatch(Action::OpenCreateSessionDialog { preselect: None });
}

pub fn script(backend: &common::MockBackend, _isolated: bool, value: &str) {
    let normal_calls = backend.create_session_calls();
    let isolated_calls = backend.create_isolated_session_calls();
    backend.script_create_session(sid(value));
    backend.seed_create_isolated_session_result(Ok(codelet_rpc_types::IsolatedSessionInfo {
        session_id: sid(value),
        base_commit: "synthetic".into(),
        worktree_path: "/synthetic/unused".into(),
    }));
    assert_eq!(
        backend.create_session_calls(),
        normal_calls,
        "seeding must not create a normal session"
    );
    assert_eq!(
        backend.create_isolated_session_calls(),
        isolated_calls,
        "seeding must not create an isolated session"
    );
}
