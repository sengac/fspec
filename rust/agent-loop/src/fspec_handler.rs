use codelet_rpc_types::{FspecRequest, FspecResult, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use codelet_tools::execution_context::{current_invocation, WorkUnitAttribution};
use codelet_tools::facade::{validate_and_resolve_path_with_isolation, IsolationContext};
use codelet_tools::{FspecHandler, FspecHandlerRequest, FspecHandlerResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn create_handler(session: Arc<BackgroundSession>, use_callback: bool) -> FspecHandler {
    Arc::new(move |request: FspecHandlerRequest| {
        // BUG-405: the session's owning manager only; no singleton.
        let Some(manager) = session.owning_manager() else {
            return FspecHandlerResult {
                success: false,
                data: String::new(),
                error: Some(format!(
                    "session {} has no owning session manager",
                    session.id
                )),
                system_reminder: None,
            };
        };
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
        // WT-016: the root comes from the session, never the daemon's cwd.
        let project_root = match resolve_project_root(&session, &request.project_root) {
            Ok(root) => root,
            Err(error) => {
                return FspecHandlerResult {
                    success: false,
                    data: String::new(),
                    error: Some(error),
                    system_reminder: None,
                }
            }
        };
        let tool_call_id = uuid::Uuid::new_v4().to_string();
        session.handle_output(StreamChunk::fspec_command_request(FspecRequest {
            command: request.command.clone(),
            args_json: request.args_json.clone(),
            project_root: project_root.to_string_lossy().into_owned(),
            tool_call_id: tool_call_id.clone(),
        }));
        let result = if use_callback {
            session.wait_for_fspec_response()
        } else {
            let result =
                codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
                    command: request.command.clone(),
                    args_json: request.args_json.clone(),
                    project_root: project_root.clone(),
                });
            FspecResult {
                success: result.success,
                data: result.data,
                error: result.error,
                system_reminder: result.system_reminder,
                tool_call_id,
            }
        };
        let same_project = result.success
            && match same_state_root(&project_root, &receipt.project) {
                Ok(same) => same,
                Err(error) => {
                    tracing::warn!(%error, "Fspec command focus reconciliation skipped");
                    false
                }
            };
        if same_project {
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

/// WT-015: whether `project_root`'s state root is the session's `project`
/// (the board the session's focus is reconciled against). A broken
/// state-root pointer is an error, never "another project".
fn same_state_root(project_root: &Path, project: &Path) -> std::io::Result<bool> {
    let state = codelet_common::spec_paths::ProjectRoots::resolve(project_root)?
        .state()
        .canonicalize()?;
    Ok(state == project.canonicalize()?)
}

/// WT-016: the root an Fspec tool call runs against.
///
/// A relative root (the tool's default is ".") joins the session's working
/// directory: the worktree of an isolated session, else the session's
/// project. In an isolated session the root then passes the file tools'
/// isolation check, so a root inside the main checkout is refused with
/// their "blocked from original project" error. A non-isolated session's
/// absolute root is used as given, as its file tools do.
fn resolve_project_root(session: &BackgroundSession, requested: &str) -> Result<PathBuf, String> {
    let project = PathBuf::from(&session.project);
    match session.worktree_path() {
        Some(worktree_path) => {
            let isolation = IsolationContext {
                worktree_path,
                blocked_project_path: project,
            };
            validate_and_resolve_path_with_isolation(requested, Some(&isolation), "fspec")
                .map_err(|error| error.to_string())
        }
        None => {
            let requested = Path::new(requested);
            Ok(if requested.is_absolute() {
                requested.to_path_buf()
            } else {
                project.join(requested)
            })
        }
    }
}
