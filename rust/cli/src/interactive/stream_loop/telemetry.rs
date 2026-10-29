use super::super::output::StreamOutput;
use crate::session::Session;
use codelet_common::debug_capture::{get_debug_capture_manager, CaptureOptions};
use codelet_core::TokenState;
use std::sync::{Arc, Mutex};

pub(super) fn start<Output: StreamOutput>(
    session: &Session,
    prompt: &str,
    output: &Output,
    request_id: &str,
    session_id: uuid::Uuid,
    preparation: &super::preflight::Preparation,
    token_state: &Arc<Mutex<TokenState>>,
) -> Arc<Mutex<Option<String>>> {
    let threshold = preparation.threshold;
    let context_window = preparation.context_window;
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                if let Ok(state) = token_state.lock() {
                    manager.capture(
                        "compaction.check",
                        serde_json::json!({
                            "timing": "hook-setup", "inputTokens": state.input_tokens,
                            "cacheReadInputTokens": state.cache_read_input_tokens,
                            "threshold": threshold, "contextWindow": context_window,
                            "messageCount": session.messages.len(),
                        }),
                        None,
                    );
                }
            }
        }
    }
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture("api.request", serde_json::json!({
                    "provider": session.current_provider_name(),
                    "model": session.current_model_id().unwrap_or_else(|| session.current_provider_name().to_string()),
                    "prompt": prompt, "promptLength": prompt.len(),
                    "messageCount": session.messages.len(),
                }), Some(CaptureOptions { request_id: Some(request_id.to_owned()) }));
            }
        }
    }
    let active_tool_call_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    if let Some(emitter) = output.progress_emitter() {
        let active_tool_call_id_cb = active_tool_call_id.clone();
        codelet_tools::set_tool_progress_callback(
            session_id,
            Some(Arc::new(move |chunk: &str, is_stderr: bool| {
                let id = active_tool_call_id_cb
                    .lock()
                    .ok()
                    .and_then(|guard| guard.clone())
                    .unwrap_or_default();
                emitter.emit_tool_progress(&id, "bash", chunk, is_stderr);
            })),
        );
    }
    active_tool_call_id
}

pub(super) fn response_start(session: &Session, request_id: &str) {
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "api.response.start",
                    serde_json::json!({
                        "provider": session.current_provider_name(),
                    }),
                    Some(CaptureOptions {
                        request_id: Some(request_id.to_owned()),
                    }),
                );
            }
        }
    }
}
