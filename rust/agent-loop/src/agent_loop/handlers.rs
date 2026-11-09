use codelet_rpc_types::SessionStatus;
use codelet_sessions::background_session::BackgroundSession;
use codelet_tools::tool_pause::{set_pause_handler, PauseHandler, PauseRequest, PauseState};
use std::sync::Arc;

pub(super) fn register_core_handlers(session: &Arc<BackgroundSession>) {
    let session_for_pause = session.clone();
    let pause_handler: PauseHandler = Arc::new(move |request: PauseRequest| {
        // BUG-291: name the tool call that raised the pause.
        let state = PauseState::for_session(session_for_pause.id, request);
        session_for_pause.set_pause_state(Some(state));
        session_for_pause.set_status(SessionStatus::Paused);
        let response = session_for_pause.wait_for_pause_response();
        // BUG-605: an answer after the turn ended keeps the turn's status.
        session_for_pause.resume_after_interaction();
        response
    });
    set_pause_handler(session.id, Some(pause_handler));
    let fspec_handler = crate::fspec_handler::create_handler(
        session.clone(),
        crate::is_global_chunk_callback_registered(),
    );
    codelet_tools::set_fspec_handler_for_session(session.id, Some(fspec_handler));
    let session_for_hitl = session.clone();
    let hitl_handler: codelet_tools::request_user_input::HitlHandler = Arc::new(
        move |_session_id, request: codelet_tools::request_user_input::HitlRequest| {
            session_for_hitl.set_hitl_request(Some(request));
            session_for_hitl.set_status(SessionStatus::Paused);
            // The answer (or BUG-605's cancel) already cleared the request.
            let response = session_for_hitl.wait_for_hitl_response();
            session_for_hitl.resume_after_interaction();
            Ok(response)
        },
    );
    codelet_tools::set_hitl_handler(session.id, Some(hitl_handler));
    let session_for_exec_stdin = session.clone();
    let exec_stdin_callback: codelet_tools::unified_exec::ExecStdinRequestCallback = Arc::new(
        move |request: Option<codelet_tools::unified_exec::ExecStdinRequest>| {
            // BUG-369: the detector callback has no caller to report to.
            if let Err(error) = session_for_exec_stdin.set_exec_stdin_request(request) {
                tracing::error!(%error, "cannot record the exec-stdin request");
            }
        },
    );
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        session.id,
        Some(exec_stdin_callback),
    );
    let session_search_handler = crate::session_search_handler::create_handler(
        std::path::PathBuf::from(&session.project),
        session.compaction_in_progress.clone(),
    );
    codelet_tools::set_session_search_handler(session.id, Some(session_search_handler));
    // KGRAPH-073: the graph follows the session's working directory.
    crate::graph_search_handler::register_for_session(session);
}

pub(super) fn clear_core_handlers(session: &BackgroundSession) {
    set_pause_handler(session.id, None);
    codelet_tools::set_fspec_handler_for_session(session.id, None);
    codelet_tools::set_session_search_handler(session.id, None);
    codelet_tools::set_graph_search_handler(session.id, None);
}

pub(super) fn clear_input_handlers(session: &BackgroundSession) {
    codelet_tools::set_hitl_handler(session.id, None);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(session.id, None);
}
