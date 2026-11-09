use crate::SessionManager;
use codelet_tools::tool_admission::{register_session, AdmissionHandler, ContextValidator};
use std::{
    path::PathBuf,
    sync::{Arc, Weak},
};
use uuid::Uuid;

pub(crate) fn register(session_id: Uuid, project: &str, manager: Weak<SessionManager>) {
    // BUG-405: the session's owning manager is the only one consulted; an
    // empty or dropped back-reference refuses admission (no singleton).
    let missing = move || format!("session {session_id} has no owning session manager");
    let id = session_id.to_string().into();
    let admission_manager = manager.clone();
    let handler: AdmissionHandler = Arc::new(move |attribution| {
        let manager = admission_manager.upgrade().ok_or_else(missing)?;
        manager
            .admit_tool_invocation(&id, attribution)
            .map_err(|error| error.to_string())
    });
    let validator_id = session_id.to_string();
    let validator: ContextValidator = Arc::new(move || {
        let manager = manager.upgrade().ok_or_else(missing)?;
        manager.get_session(&validator_id).map(|_| ())
    });
    register_session(session_id, PathBuf::from(project), handler, validator);
}
