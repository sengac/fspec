use codelet_rpc_types::{FspecRequest, FspecResult, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use codelet_tools::execution_context::{current_invocation, WorkUnitAttribution};
use codelet_tools::{FspecHandler, FspecHandlerRequest, FspecHandlerResult};
use std::sync::Arc;

pub fn create_handler(session: Arc<BackgroundSession>, use_callback: bool) -> FspecHandler {
    Arc::new(move |request: FspecHandlerRequest| {
        let owning = session.owning_manager();
        let manager = owning
            .as_deref()
            .unwrap_or_else(|| codelet_sessions::SessionManager::instance());
        let receipt = current_invocation().map(Ok).unwrap_or_else(|| {
            manager
                .admit_tool_invocation(&session.id.to_string().into(), WorkUnitAttribution::Absent)
        });
        let receipt = match receipt {
            Ok(receipt) if receipt.session_id == session.id => receipt,
            result => {
                return FspecHandlerResult {
                    success: false,
                    data: String::new(),
                    error: Some(format!("invalid invoking session context: {result:?}")),
                    system_reminder: None,
                }
            }
        };
        let tool_call_id = uuid::Uuid::new_v4().to_string();
        session.handle_output(StreamChunk::fspec_command_request(FspecRequest {
            command: request.command.clone(),
            args_json: request.args_json.clone(),
            project_root: request.project_root.clone(),
            tool_call_id: tool_call_id.clone(),
        }));
        let result = if use_callback {
            session.wait_for_fspec_response()
        } else {
            let result =
                codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
                    command: request.command.clone(),
                    args_json: request.args_json.clone(),
                    project_root: request.project_root.clone().into(),
                });
            FspecResult {
                success: result.success,
                data: result.data,
                error: result.error,
                system_reminder: result.system_reminder,
                tool_call_id,
            }
        };
        let project = std::path::Path::new(&request.project_root);
        let same_project = project
            .canonicalize()
            .ok()
            .zip(receipt.project.canonicalize().ok())
            .is_some_and(|(first, second)| first == second);
        if result.success && same_project {
            let reconciled = if request.command == "update-work-unit-status" {
                serde_json::from_str::<serde_json::Value>(&request.args_json)
                    .ok()
                    .and_then(|args| args["workUnitId"].as_str().map(str::to_owned))
                    .map_or_else(
                        || manager.reconcile_work_units(&receipt.project),
                        |target| manager.reconcile_status_success(&receipt, &target),
                    )
            } else {
                manager.reconcile_work_units(&receipt.project)
            };
            if let Err(error) = reconciled {
                tracing::warn!(%error, "successful Fspec command focus reconciliation failed");
            }
        }
        session.handle_output(StreamChunk::fspec_command_result(result.clone()));
        FspecHandlerResult {
            success: result.success,
            data: result.data,
            error: result.error,
            system_reminder: result.system_reminder,
        }
    })
}
