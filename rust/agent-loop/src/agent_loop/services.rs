use crate::bridges::{register_agent_manager_handler, register_deep_search_handler};
use crate::terminal_overflow_recovery::CompactorRegistration;
use crate::turn_compaction::TurnCompaction;
use codelet_cli::session::Session;
use codelet_sessions::background_session::BackgroundSession;
use std::sync::Arc;

pub(super) fn register_service_handlers(
    session: &Arc<BackgroundSession>,
    inner_session: &Session,
) -> (Arc<TurnCompaction>, CompactorRegistration) {
    register_deep_search_handler(
        session.id,
        inner_session,
        std::path::PathBuf::from(&session.project),
    );
    crate::generate_compaction_handler::register_compactor_sub_agent_handler(
        session.id,
        inner_session,
        std::path::PathBuf::from(&session.project),
        session.compaction_in_progress.clone(),
    );
    let compactor_registration = CompactorRegistration::new(session.id);
    let turn_compaction = crate::generate_compaction_handler::register_generate_compaction_handler(
        session.id,
        inner_session,
        std::path::PathBuf::from(&session.project),
        session.clone(),
        session.owning_manager(),
    );
    register_agent_manager_handler(
        session.id,
        inner_session,
        session.project.clone(),
        session.owning_manager(),
    );
    {
        let async_handler =
            crate::agent_manager_handler::create_async_handler(session.owning_manager());
        codelet_tools::set_agent_manager_async_handler(session.id, Some(async_handler));
    }
    {
        let schedule_handler = crate::schedule_handler::create_handler(session.project.clone());
        codelet_tools::set_schedule_handler(session.id, Some(schedule_handler));
    }
    (turn_compaction, compactor_registration)
}

pub(super) fn clear_service_handlers(session: &BackgroundSession) {
    codelet_tools::set_deep_search_handler(session.id, None);
    codelet_tools::set_agent_manager_handler(session.id, None);
    codelet_tools::set_agent_manager_async_handler(session.id, None);
    codelet_tools::set_schedule_handler(session.id, None);
}
