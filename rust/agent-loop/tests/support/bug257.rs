use codelet_core::{persistence::*, SessionManagerHandle};
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::Value;

#[path = "../../../cli/tests/bug243/fixtures.rs"]
pub mod fixtures;
#[path = "../../../providers/tests/fixtures/prov148.rs"]
pub mod transport;

pub struct Environment {
    pub directory: tempfile::TempDir,
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl Environment {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = fixtures::setup()?;
        let previous = ["ANTHROPIC_API_KEY", "FSPEC_USER_DIR"]
            .into_iter()
            .map(|name| (name, std::env::var_os(name)))
            .collect();
        std::env::set_var("ANTHROPIC_API_KEY", "test-fork-offline");
        std::env::set_var("FSPEC_USER_DIR", directory.path());
        std::fs::create_dir_all(directory.path().join("cache"))?;
        std::fs::write(
            directory.path().join("cache/models.json"),
            include_str!("../../../sessions/tests/fixtures/prov123_models.json"),
        )?;
        Ok(Self {
            directory,
            previous,
        })
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        for (name, previous) in &self.previous {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        reset_stores_for_tests();
    }
}

pub fn inventory() -> Result<Vec<uuid::Uuid>, String> {
    let mut sessions = Vec::new();
    for entry in std::fs::read_dir(codelet_common::get_data_dir()?.join("sessions"))
        .map_err(|error| error.to_string())?
    {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let identity = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or("invalid session filename")?;
            sessions.push(uuid::Uuid::parse_str(identity).map_err(|error| error.to_string())?);
        }
    }
    sessions.sort();
    Ok(sessions)
}

pub async fn next_request(child: &SessionManifest) -> Result<Value, Box<dyn std::error::Error>> {
    reset_stores_for_tests();
    let child = load_session(child.id)?;
    let manager = SessionManager::new();
    manager
        .create_session_from_manifest(&child, "anthropic/claude-sonnet-4")
        .await?;
    let background = manager.get_session(&child.id.to_string())?;
    background
        .inner
        .lock()
        .await
        .provider_session
        .ensure_provider("codex");
    manager.restore_session_messages(
        &SessionId::new(child.id.to_string()),
        get_session_message_envelopes(child.id)?,
    )?;
    let inner = background.inner.lock().await;
    let history = fixtures::history(&inner)?;
    let server = transport::Server::start(
        vec![transport::terminal("completed", Some(true), None)],
        false,
    )
    .await?;
    let model = server.models()?.0.with_history(history);
    let request = CompletionRequestBuilder::new(model.clone(), "child-next-unique")
        .messages(inner.messages.clone())
        .build();
    drop(inner);
    let mut response = model.stream(request).await?;
    while let Some(event) = response.next().await {
        event?;
    }
    let requests = server
        .requests
        .lock()
        .map_err(|_| "capture lock poisoned")?;
    assert_eq!(requests.len(), 1);
    Ok(requests[0].clone())
}
