use codelet_rpc_types::SessionId;

use crate::SessionManager;

/// BUG-613: how to choose a default model, named by every refusal for want of one.
pub(crate) const CHOOSE_A_DEFAULT: &str =
    "choose one with /model in the TUI or set_default_model over the RPC";

pub(crate) fn create_session_checked(
    manager: &SessionManager,
    role: Option<String>,
) -> Result<SessionId, String> {
    // BUG-410: a missing default model is an error, never an empty id.
    let model = manager.get_default_model().ok_or_else(|| {
        format!("create_session declined: no default model is set: {CHOOSE_A_DEFAULT}")
    })?;
    let project = manager.default_project()?.to_string_lossy().into_owned();
    create_session_in_project(manager, &model, project, role)
}

/// RPC-444: create a session in an explicit project directory.
pub(crate) fn create_session_in_project(
    manager: &SessionManager,
    model: &str,
    project: String,
    role: Option<String>,
) -> Result<SessionId, String> {
    tracing::info!(%model, %project, ?role, "create_session: starting checked creation");
    let id = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(SessionManager::create_session(manager, model, &project))
    })?;
    if let Some(role) = role.filter(|value| !value.is_empty()) {
        manager.get_session(&id)?.set_role(role);
    }
    Ok(SessionId::new(id))
}

/// BUG-427: create an isolated (worktree-backed) session in `project`.
///
/// **Runtime requirement (RPC-070):** bridges sync→async via
/// `tokio::task::block_in_place(|| Handle::current().block_on(...))`; MUST
/// be invoked from a multi-thread tokio runtime. A role that cannot be
/// applied is an error.
pub(crate) fn create_isolated_session_in_project(
    manager: &SessionManager,
    project: &std::path::Path,
    role: Option<String>,
) -> Result<codelet_rpc_types::IsolatedSessionInfo, String> {
    let id = uuid::Uuid::new_v4();
    let model = manager.get_default_model().ok_or_else(|| {
        format!("create_isolated_session declined: no default model is set: {CHOOSE_A_DEFAULT}")
    })?;
    let name = format!("isolated-{}", &id.to_string()[..8]);
    let project = project.to_string_lossy().into_owned();
    let info = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            manager
                .create_isolated_session_with_id(&id.to_string(), &model, &project, &name)
                .await
        })
    })?;
    if let Some(role) = role.filter(|role| !role.is_empty()) {
        manager.get_session(&id.to_string())?.set_role(role);
    }
    Ok(info)
}
