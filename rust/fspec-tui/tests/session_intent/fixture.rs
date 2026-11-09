use crate::common::TaskOutcome;
use std::{ffi::OsString, path::PathBuf};

use super::*;

pub use codelet_test_helpers::TestResult;

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
            if let Err(error) = codelet_common::set_data_directory(data) {
                eprintln!("cannot restore the data directory: {error}");
            }
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
    // TUI-125: an announced session does not take focus; the creation answer
    // (the user opened this agent) focuses it, as after the New Agent dialog.
    app.dispatch(Action::SessionCreated(sid(value)));
    settle(app).await?;
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

pub async fn settle(app: &mut App) -> codelet_test_helpers::TestResult {
    loop {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        match app.try_recv_action() {
            Some(action) => app.dispatch(action),
            None => break,
        }
    }
    Ok(())
}

pub fn focus(app: &App, value: &str) -> Option<String> {
    app.agent_view_store()
        .work_unit_context_for(&sid(value))
        .map(|context| context.id.clone())
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
