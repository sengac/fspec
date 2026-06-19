//! fspec-command runner — pure async helpers extracted from
//! `dispatch_stream_chunks.rs` so that parent file honours the 300-LoC source-
//! shape ceiling pinned by `source_shape_rpc049.rs`.
//!
//! `App::spawn_fspec_command_runner` (in `dispatch_stream_chunks.rs`) invokes
//! [`run_fspec_command`] inside a `tokio::spawn` and forwards the result
//! back to the originating session via `backend.send_fspec_result`. The
//! runner is intentionally minimal — happy-path commands `list-work-units`
//! and `show-work-unit` only; everything else returns an unsupported-
//! command error so requesting sessions don't hang.

use codelet_rpc_types::{FspecRequest, FspecResult};

/// Execute `request` against `backend` and produce an `FspecResult`.
/// Pure async helper — kept outside any `impl App` block so the runner
/// task can call it without holding an `App` reference.
pub(crate) async fn run_fspec_command(
    backend: &dyn crate::transport::FspecBackend,
    request: &FspecRequest,
) -> FspecResult {
    match request.command.as_str() {
        "list-work-units" => match backend.list_work_units().await {
            Ok(units) => match serde_json::to_string(&units) {
                Ok(data) => FspecResult {
                    success: true,
                    data,
                    error: None,
                    system_reminder: None,
                    tool_call_id: request.tool_call_id.clone(),
                },
                Err(e) => FspecResult {
                    success: false,
                    data: String::new(),
                    error: Some(format!("serialise list-work-units result: {e}")),
                    system_reminder: None,
                    tool_call_id: request.tool_call_id.clone(),
                },
            },
            Err(e) => FspecResult {
                success: false,
                data: String::new(),
                error: Some(format!("list-work-units: {e}")),
                system_reminder: None,
                tool_call_id: request.tool_call_id.clone(),
            },
        },
        "show-work-unit" => {
            let target_id = parse_show_work_unit_id(&request.args_json);
            match backend.list_work_units().await {
                Ok(units) => match target_id {
                    Some(id) => match units.iter().find(|u| u.id == id) {
                        Some(unit) => match serde_json::to_string(unit) {
                            Ok(data) => FspecResult {
                                success: true,
                                data,
                                error: None,
                                system_reminder: None,
                                tool_call_id: request.tool_call_id.clone(),
                            },
                            Err(e) => FspecResult {
                                success: false,
                                data: String::new(),
                                error: Some(format!("serialise show-work-unit: {e}")),
                                system_reminder: None,
                                tool_call_id: request.tool_call_id.clone(),
                            },
                        },
                        None => FspecResult {
                            success: false,
                            data: String::new(),
                            error: Some(format!("work unit not found: {id}")),
                            system_reminder: None,
                            tool_call_id: request.tool_call_id.clone(),
                        },
                    },
                    None => FspecResult {
                        success: false,
                        data: String::new(),
                        error: Some("show-work-unit: missing `id` in args_json".to_string()),
                        system_reminder: None,
                        tool_call_id: request.tool_call_id.clone(),
                    },
                },
                Err(e) => FspecResult {
                    success: false,
                    data: String::new(),
                    error: Some(format!("show-work-unit: {e}")),
                    system_reminder: None,
                    tool_call_id: request.tool_call_id.clone(),
                },
            }
        }
        other => FspecResult {
            success: false,
            data: String::new(),
            error: Some(format!("unsupported command: {other}")),
            system_reminder: None,
            tool_call_id: request.tool_call_id.clone(),
        },
    }
}

/// Best-effort `id` extraction from `show-work-unit`'s `args_json`.
/// Accepts both `{"id":"AUTH-001"}` and the fspec-CLI-style
/// `{"_":["AUTH-001"]}`. Returns `None` when neither shape matches.
fn parse_show_work_unit_id(args_json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(args_json).ok()?;
    if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
        return Some(id.to_string());
    }
    if let Some(arr) = value.get("_").and_then(|v| v.as_array()) {
        if let Some(first) = arr.first().and_then(|v| v.as_str()) {
            return Some(first.to_string());
        }
    }
    None
}
