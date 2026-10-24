use crate::SessionManager;
use codelet_tools::tool_admission::{register_session, AdmissionHandler, ContextValidator};
use std::{
    path::PathBuf,
    sync::{Arc, Weak},
};
use uuid::Uuid;

pub(crate) fn register(session_id: Uuid, project: &str, manager: Weak<SessionManager>) {
    let singleton = Weak::ptr_eq(&manager, &Weak::new());
    let id = session_id.to_string().into();
    let admission_manager = manager.clone();
    let handler: AdmissionHandler = Arc::new(move |attribution| {
        let result = match admission_manager.upgrade() {
            Some(manager) => manager.admit_tool_invocation(&id, attribution),
            None if singleton => SessionManager::instance().admit_tool_invocation(&id, attribution),
            None => return Err("invoking session manager is no longer available".into()),
        };
        result.map_err(|error| error.to_string())
    });
    let validator_id = session_id.to_string();
    let validator: ContextValidator = Arc::new(move || {
        let result = match manager.upgrade() {
            Some(manager) => manager.get_session(&validator_id),
            None if singleton => SessionManager::instance().get_session(&validator_id),
            None => return Err("invoking session manager is no longer available".into()),
        };
        result.map(|_| ())
    });
    register_session(session_id, PathBuf::from(project), handler, validator);
}
