//! BUG-449: BLOCK-006 stage write permissions for a session.
//!
//! Every file-writing tool calls [`check_session_write`] after resolving its
//! path. The session's stage and project root come from the registered
//! work-unit stage callback; the project's stage-permissions config is
//! loaded for that root and the path is matched relative to it.
//!
//! Feature: spec/features/stage-write-permissions-are-enforced.feature

use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::facade::emit_block_notification;
use crate::facade::wrapper::{unrestricted_tool_access_declared, work_unit_stage_callback};
use crate::stage_permissions::{load_stage_permissions_config, StagePermissionsMatcher};
use crate::ToolError;

/// A session's work unit stage and the root its files live under (the
/// worktree for an isolated session, else the project).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStage {
    pub stage: String,
    pub project_root: PathBuf,
}

/// Refuse a write the session's stage does not allow. `Uuid::nil()` (no
/// session) and a session without a work unit stage are not gated. A failed
/// stage lookup, no stage callback in a process that did not declare
/// unrestricted tool access (BUG-447), and an unreadable stage-permissions
/// config refuse the write.
pub fn check_session_write(
    session_id: Uuid,
    tool: &'static str,
    resolved_path: &str,
) -> Result<(), ToolError> {
    if session_id.is_nil() {
        return Ok(());
    }
    let refuse = |message: String| {
        emit_block_notification(session_id, &format!("{tool} {resolved_path}"), &message);
        ToolError::Blocked { tool, message }
    };
    let stage = match work_unit_stage_callback() {
        Some(callback) => callback(session_id.to_string()).map_err(|error| {
            refuse(format!(
                "cannot look up the session's work unit stage: {error}"
            ))
        })?,
        None if unrestricted_tool_access_declared() => return Ok(()),
        None => {
            return Err(refuse(
                "no work-unit stage callback is registered in this process and unrestricted \
                 tool access was not declared"
                    .to_string(),
            ))
        }
    };
    let Some(SessionStage {
        stage,
        project_root,
    }) = stage
    else {
        return Ok(());
    };
    let config = load_stage_permissions_config(Some(&project_root)).map_err(refuse)?;
    let relative = relative_to(Path::new(resolved_path), &project_root);
    let result = StagePermissionsMatcher::new(config).check_write(&relative, &stage);
    if result.blocked {
        let reason = result
            .reason
            .unwrap_or_else(|| format!("writing is not allowed in the {stage} stage"));
        let guidance = result.guidance.map(|guidance| format!(" {guidance}"));
        return Err(refuse(format!("{reason}{}", guidance.unwrap_or_default())));
    }
    Ok(())
}

/// The path relative to the project root (stage patterns such as
/// `src/**/*.rs` are project-relative); a path outside it is matched as is.
fn relative_to(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
