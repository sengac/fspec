use codelet_rpc_types::SessionId;

use crate::SessionManager;

pub(crate) fn create_session_checked(
    manager: &SessionManager,
    role: Option<String>,
) -> Result<SessionId, String> {
    let Some(model) = manager.get_default_model() else {
        tracing::warn!("create_session declined: no default model set");
        return Ok(SessionId::new(String::new()));
    };
    let project = std::env::current_dir()
        .map_err(|error| format!("Failed to resolve session project directory: {error}"))?
        .to_string_lossy()
        .into_owned();
    tracing::info!(%model, %project, ?role, "create_session: starting checked creation");
    let id = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(SessionManager::create_session(manager, &model, &project))
    })?;
    if let Some(role) = role.filter(|value| !value.is_empty()) {
        manager.get_session(&id)?.set_role(role);
    }
    Ok(SessionId::new(id))
}
