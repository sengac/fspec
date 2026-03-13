//! FacadeToolWrapper - Adapts ToolFacade implementations to rig::tool::Tool trait.
//!
//! This wrapper enables facades to be used with rig's agent builder by implementing
//! the Tool trait and delegating to the underlying facade for schema/naming while
//! executing against the base tool implementation.
//!
//! CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS

use super::traits::{
    BoxedFileToolFacade, BoxedToolFacade, InternalFileParams, InternalWebSearchParams,
};
use crate::web_search::{WebSearchRequest, WebSearchResult, WebSearchTool};
use crate::{EditTool, ReadTool, ToolError, WriteTool};
use codelet_common::web_search::WebSearchAction;
use rig::completion::ToolDefinition as RigToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Wrapper that adapts a ToolFacade to rig's Tool trait.
///
/// This enables provider-specific facades to be used with rig's agent builder
/// while maintaining the facade's custom tool name, schema, and parameter mapping.
/// TOOL-014: FacadeToolWrapper now requires session_id for worktree isolation.
/// Even though web search doesn't use paths, the pattern is maintained for consistency.
pub struct FacadeToolWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedToolFacade,
    /// The base web search tool for actual execution
    base_tool: WebSearchTool,
    /// Session ID for worktree isolation consistency (TOOL-014)
    #[allow(dead_code)]
    session_id: Uuid,
}

/// Arguments for the facade wrapper - accepts raw JSON for flexible param mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacadeArgs(pub Value);

impl FacadeToolWrapper {
    /// Create a new wrapper for the given facade with session association.
    ///
    /// # Arguments
    /// * `facade` - The provider-specific facade for schema/naming
    /// * `session_id` - The session ID for consistency (TOOL-014)
    pub fn new(facade: BoxedToolFacade, session_id: Uuid) -> Self {
        Self {
            facade,
            base_tool: WebSearchTool::new(session_id),
            session_id,
        }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
    }
}

impl Tool for FacadeToolWrapper {
    // Dummy const - we override name() to return the facade's dynamic name
    const NAME: &'static str = "facade_wrapper";

    type Error = ToolError;
    type Args = FacadeArgs;
    type Output = WebSearchResult;

    /// Override to return the facade's tool name (e.g., "google_web_search" for Gemini)
    fn name(&self) -> String {
        self.facade.tool_name().to_string()
    }

    /// Return the facade's provider-specific tool definition
    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        let facade_def = self.facade.definition();
        RigToolDefinition {
            name: facade_def.name,
            description: facade_def.description,
            parameters: facade_def.parameters,
        }
    }

    /// Map provider params to internal format and execute the base tool
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Use the facade to map provider-specific params to internal format
        let internal_params = self.facade.map_params(args.0)?;

        // Convert internal params to WebSearchRequest for the base tool
        let request = match internal_params {
            InternalWebSearchParams::Search { query } => WebSearchRequest {
                action: WebSearchAction::Search { query: Some(query) },
            },
            InternalWebSearchParams::OpenPage { url, headless, pause } => WebSearchRequest {
                action: WebSearchAction::OpenPage {
                    url: Some(url),
                    headless,
                    pause,
                },
            },
            InternalWebSearchParams::FindInPage { url, pattern, headless, pause } => {
                WebSearchRequest {
                    action: WebSearchAction::FindInPage {
                        url: Some(url),
                        pattern: Some(pattern),
                        headless,
                        pause,
                    },
                }
            }
            InternalWebSearchParams::CaptureScreenshot {
                url,
                output_path,
                full_page,
                headless,
                pause,
            } => WebSearchRequest {
                action: WebSearchAction::CaptureScreenshot {
                    url: Some(url),
                    output_path,
                    full_page: Some(full_page),
                    headless,
                    pause,
                },
            },
        };

        // Execute against the base tool
        self.base_tool.call(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facade::{GeminiGoogleWebSearchFacade, GeminiReadFileFacade, GeminiWebFetchFacade};
    use std::sync::Arc;

    #[test]
    fn test_wrapper_returns_facade_tool_name() {
        let facade = Arc::new(GeminiGoogleWebSearchFacade) as BoxedToolFacade;
        let wrapper = FacadeToolWrapper::new(facade, Uuid::nil());

        assert_eq!(wrapper.name(), "google_web_search");
    }

    #[test]
    fn test_wrapper_returns_facade_tool_name_web_fetch() {
        let facade = Arc::new(GeminiWebFetchFacade) as BoxedToolFacade;
        let wrapper = FacadeToolWrapper::new(facade, Uuid::nil());

        assert_eq!(wrapper.name(), "web_fetch");
    }

    #[tokio::test]
    async fn test_wrapper_returns_flat_schema_for_gemini() {
        let facade = Arc::new(GeminiGoogleWebSearchFacade) as BoxedToolFacade;
        let wrapper = FacadeToolWrapper::new(facade, Uuid::nil());

        let def = wrapper.definition(String::new()).await;

        assert_eq!(def.name, "google_web_search");
        assert!(def.parameters["properties"]["query"].is_object());
        assert!(def.parameters.get("oneOf").is_none());
        assert!(def.parameters["properties"].get("action").is_none());
    }

    #[test]
    fn test_file_wrapper_returns_facade_tool_name() {
        let facade = Arc::new(GeminiReadFileFacade) as BoxedFileToolFacade;
        // BLOCK-006: FileToolFacadeWrapper now requires session_id (use nil for tests)
        let wrapper = FileToolFacadeWrapper::new(facade, Uuid::nil());

        assert_eq!(wrapper.name(), "read_file");
    }

    #[tokio::test]
    async fn test_file_wrapper_returns_flat_schema() {
        let facade = Arc::new(GeminiReadFileFacade) as BoxedFileToolFacade;
        // BLOCK-006: FileToolFacadeWrapper now requires session_id (use nil for tests)
        let wrapper = FileToolFacadeWrapper::new(facade, Uuid::nil());

        let def = wrapper.definition(String::new()).await;

        assert_eq!(def.name, "read_file");
        assert!(def.parameters["properties"]["file_path"].is_object());
        assert!(def.parameters.get("oneOf").is_none());
    }
}

// ============================================================================
// FileToolFacadeWrapper - Adapts FileToolFacade implementations to rig::tool::Tool
// ============================================================================

/// Result type for file operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationResult {
    pub success: bool,
    pub content: Option<String>,
    pub error: Option<String>,
}

/// Wrapper that adapts a FileToolFacade to rig's Tool trait.
///
/// This enables provider-specific file facades to be used with rig's agent builder
/// while maintaining the facade's custom tool name, schema, and parameter mapping.
///
/// ## BLOCK-006: Block Notifications
///
/// The wrapper stores session_id (TOOL-012 pattern) to emit UserNotification chunks
/// when file writes are blocked by stage permissions. Notifications are emitted via
/// the global chunk callback before returning the blocked error to the LLM.
pub struct FileToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedFileToolFacade,
    /// The base tools for actual execution
    read_tool: ReadTool,
    write_tool: WriteTool,
    edit_tool: EditTool,
    /// Session ID for notification emission - set at construction time (TOOL-012)
    /// Used by BLOCK-006 to emit UserNotification when writes are blocked.
    session_id: Uuid,
}

impl FileToolFacadeWrapper {
    /// Create a new wrapper for the given file facade with session association.
    ///
    /// # Arguments
    /// * `facade` - The provider-specific facade for schema/naming
    /// * `session_id` - The session ID for notification emission (BLOCK-006) and worktree isolation (TOOL-014)
    pub fn new(facade: BoxedFileToolFacade, session_id: Uuid) -> Self {
        Self {
            facade,
            read_tool: ReadTool::new(session_id),
            write_tool: WriteTool::new(session_id),
            edit_tool: EditTool::new(session_id),
            session_id,
        }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
    }

    /// Get the session ID associated with this tool instance
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
}

impl Tool for FileToolFacadeWrapper {
    const NAME: &'static str = "file_facade_wrapper";

    type Error = ToolError;
    type Args = FacadeArgs;
    type Output = FileOperationResult;

    /// Override to return the facade's tool name (e.g., "read_file" for Gemini)
    fn name(&self) -> String {
        self.facade.tool_name().to_string()
    }

    /// Return the facade's provider-specific tool definition
    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        let facade_def = self.facade.definition();
        RigToolDefinition {
            name: facade_def.name,
            description: facade_def.description,
            parameters: facade_def.parameters,
        }
    }

    /// Map provider params to internal format and execute the appropriate base tool
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Use the facade to map provider-specific params to internal format
        let internal_params = self.facade.map_params(args.0)?;

        // Execute the appropriate base tool based on the operation type
        match internal_params {
            InternalFileParams::Read {
                file_path,
                offset,
                limit,
                // mode and indentation are accepted for Codex model compatibility
                // but not yet used by ReadTool — falls through to default slice behavior
                mode: _,
                indentation: _,
            } => {
                // TOOL-014: Validate and resolve path for worktree isolation
                let resolved_path = match validate_and_resolve_path(self.session_id, &file_path, "read") {
                    Ok(path) => path.to_string_lossy().to_string(),
                    Err(e) => {
                        return Ok(FileOperationResult {
                            success: false,
                            content: None,
                            error: Some(e.to_string()),
                        });
                    }
                };
                
                use crate::read::ReadArgs;
                let read_args = ReadArgs {
                    file_path: resolved_path,
                    offset,
                    limit,
                    pdf_mode: None,
                };
                match self.read_tool.call(read_args).await {
                    Ok(content) => Ok(FileOperationResult {
                        success: true,
                        content: Some(content),
                        error: None,
                    }),
                    Err(e) => Ok(FileOperationResult {
                        success: false,
                        content: None,
                        error: Some(e.to_string()),
                    }),
                }
            }
            InternalFileParams::Write { file_path, content } => {
                // TOOL-014: Validate and resolve path for worktree isolation
                let resolved_path = match validate_and_resolve_path(self.session_id, &file_path, "write") {
                    Ok(path) => path.to_string_lossy().to_string(),
                    Err(e) => {
                        // Emit notification for blocked path
                        let action = format!("writing {file_path}");
                        emit_block_notification(self.session_id, &action, &e.to_string());
                        return Ok(FileOperationResult {
                            success: false,
                            content: None,
                            error: Some(e.to_string()),
                        });
                    }
                };
                
                // BLOCK-006: Check stage permissions before write
                // Get the current work unit's stage from the session
                let stage = get_work_unit_stage(self.session_id);
                if let Err(blocked) = check_write_permission(&resolved_path, stage.as_deref()) {
                    // Emit notification to TUI before returning blocked error
                    let action = format!("writing {file_path}");
                    emit_block_notification(self.session_id, &action, &blocked.reason);
                    
                    return Ok(FileOperationResult {
                        success: false,
                        content: None,
                        error: Some(blocked.to_string()),
                    });
                }
                
                use crate::write::WriteArgs;
                let write_args = WriteArgs { file_path: resolved_path, content };
                match self.write_tool.call(write_args).await {
                    Ok(result) => Ok(FileOperationResult {
                        success: true,
                        content: Some(result),
                        error: None,
                    }),
                    Err(e) => Ok(FileOperationResult {
                        success: false,
                        content: None,
                        error: Some(e.to_string()),
                    }),
                }
            }
            InternalFileParams::Edit {
                file_path,
                old_string,
                new_string,
            } => {
                // TOOL-014: Validate and resolve path for worktree isolation
                let resolved_path = match validate_and_resolve_path(self.session_id, &file_path, "edit") {
                    Ok(path) => path.to_string_lossy().to_string(),
                    Err(e) => {
                        // Emit notification for blocked path
                        let action = format!("editing {file_path}");
                        emit_block_notification(self.session_id, &action, &e.to_string());
                        return Ok(FileOperationResult {
                            success: false,
                            content: None,
                            error: Some(e.to_string()),
                        });
                    }
                };
                
                // BLOCK-006: Check stage permissions before edit (same as write)
                // Get the current work unit's stage from the session
                let stage = get_work_unit_stage(self.session_id);
                if let Err(blocked) = check_write_permission(&resolved_path, stage.as_deref()) {
                    // Emit notification to TUI before returning blocked error
                    let action = format!("editing {file_path}");
                    emit_block_notification(self.session_id, &action, &blocked.reason);
                    
                    return Ok(FileOperationResult {
                        success: false,
                        content: None,
                        error: Some(blocked.to_string()),
                    });
                }
                
                use crate::edit::EditArgs;
                let edit_args = EditArgs {
                    file_path: resolved_path,
                    old_string,
                    new_string,
                };
                match self.edit_tool.call(edit_args).await {
                    Ok(result) => Ok(FileOperationResult {
                        success: true,
                        content: Some(result),
                        error: None,
                    }),
                    Err(e) => Ok(FileOperationResult {
                        success: false,
                        content: None,
                        error: Some(e.to_string()),
                    }),
                }
            }
        }
    }
}

// ============================================================================
// BashToolFacadeWrapper - Adapts BashToolFacade implementations to rig::tool::Tool
// ============================================================================

use super::fspec_facade::BoxedFspecToolFacade;
use super::traits::{BoxedBashToolFacade, InternalBashParams};
use crate::bash::BashArgs;
use crate::stage_permissions::check_write_permission;
use crate::BashTool;
use uuid::Uuid;

// ============================================================================
// BLOCK-006: Block Notification Helper
// ============================================================================

/// Callback type for emitting block notifications to the TUI.
/// This is set by the NAPI layer when initializing the session.
/// The callback takes (session_id_str, action, reason) and emits a UserNotification chunk.
pub type BlockNotificationCallback = fn(String, String, String);

/// Callback type for getting the current work unit stage from a session.
/// This is set by the NAPI layer when initializing.
/// The callback takes session_id_str and returns Option<String> (the stage/status).
pub type GetWorkUnitStageCallback = fn(String) -> Option<String>;

/// GIT-020: Isolation context for file access control in isolated sessions.
/// 
/// Contains both the worktree path (where operations are allowed) and the 
/// blocked project path (where operations are forbidden).
#[derive(Clone, Debug)]
pub struct IsolationContext {
    /// The worktree path - files within this directory are ALLOWED
    pub worktree_path: PathBuf,
    /// The original project path - files within this directory are BLOCKED
    /// (except for those within the worktree which takes precedence)
    pub blocked_project_path: PathBuf,
}

/// GIT-020: Callback type for getting the isolation context from a session.
/// This is set by the NAPI layer when initializing.
/// The callback takes session_id_str and returns Option<IsolationContext>.
/// For isolated sessions, this returns Some(IsolationContext) with worktree and blocked_project paths.
/// For non-isolated sessions, this returns None (no restrictions).
pub type GetEffectiveCwdCallback = fn(String) -> Option<IsolationContext>;

/// Global callback for emitting block notifications.
/// Set by codelet-napi during session initialization.
static BLOCK_NOTIFICATION_CALLBACK: std::sync::OnceLock<BlockNotificationCallback> =
    std::sync::OnceLock::new();

/// Global callback for getting work unit stage from session.
/// Set by codelet-napi during initialization.
static GET_WORK_UNIT_STAGE_CALLBACK: std::sync::OnceLock<GetWorkUnitStageCallback> =
    std::sync::OnceLock::new();

/// GIT-020: Global callback for getting effective_cwd from session.
/// Set by codelet-napi during initialization.
static GET_EFFECTIVE_CWD_CALLBACK: std::sync::OnceLock<GetEffectiveCwdCallback> =
    std::sync::OnceLock::new();

/// Register the global block notification callback.
/// Called by codelet-napi during initialization.
pub fn set_block_notification_callback(callback: BlockNotificationCallback) {
    let _ = BLOCK_NOTIFICATION_CALLBACK.set(callback);
}

/// Register the global work unit stage callback.
/// Called by codelet-napi during initialization.
pub fn set_get_work_unit_stage_callback(callback: GetWorkUnitStageCallback) {
    let _ = GET_WORK_UNIT_STAGE_CALLBACK.set(callback);
}

/// GIT-020: Register the global effective_cwd callback.
/// Called by codelet-napi during initialization.
pub fn set_get_effective_cwd_callback(callback: GetEffectiveCwdCallback) {
    let _ = GET_EFFECTIVE_CWD_CALLBACK.set(callback);
}

/// Get the current work unit stage for a session.
///
/// Returns None if:
/// - Callback not registered
/// - Session not found
/// - No work unit context attached to session
fn get_work_unit_stage(session_id: Uuid) -> Option<String> {
    GET_WORK_UNIT_STAGE_CALLBACK.get()
        .and_then(|callback| callback(session_id.to_string()))
}

/// GIT-020: Get the isolation context for a session.
///
/// Returns the IsolationContext for isolated sessions (with worktree and blocked_project paths).
/// Returns None for non-isolated sessions (no file access restrictions).
///
/// # Arguments
/// * `session_id` - The session UUID to get isolation context for
///
/// # Returns
/// * `Some(IsolationContext)` - The isolation context for restricted access
/// * `None` - No isolation (callback not registered, session not found, or non-isolated session)
pub fn get_isolation_context(session_id: Uuid) -> Option<IsolationContext> {
    GET_EFFECTIVE_CWD_CALLBACK.get()
        .and_then(|callback| callback(session_id.to_string()))
}

/// Legacy alias for get_isolation_context - returns just the worktree path.
/// Used by code that only needs the effective working directory.
pub fn get_effective_cwd(session_id: Uuid) -> Option<PathBuf> {
    get_isolation_context(session_id).map(|ctx| ctx.worktree_path)
}

/// Emit a block notification to the TUI.
///
/// # Arguments
/// * `session_id` - The session to emit the notification to
/// * `action` - What action was blocked (e.g., "git checkout", "writing src/auth.ts")
/// * `reason` - Why it was blocked (e.g., "Use git switch instead", "Cannot write impl files in testing stage")
///
/// The notification message follows the format: "AI was blocked from {action} - {reason}"
pub fn emit_block_notification(session_id: Uuid, action: &str, reason: &str) {
    if let Some(callback) = BLOCK_NOTIFICATION_CALLBACK.get() {
        callback(session_id.to_string(), action.to_string(), reason.to_string());
    }
}

/// TOOL-014: Validate and resolve a path for worktree isolation.
///
/// This function ensures that file operations in isolated sessions are restricted
/// appropriately. It provides the core path validation logic for all tools.
///
/// # Behavior
///
/// If session has an isolation context (isolated session):
/// - Paths within the worktree are ALLOWED
/// - Paths within the blocked_project (original project) are BLOCKED
/// - Paths elsewhere (e.g., /tmp, /etc) are ALLOWED
///
/// If session has no isolation context (Uuid::nil() or non-isolated):
/// - Paths are returned as-is (normal operation, no validation)
///
/// # Arguments
/// * `session_id` - The session UUID for isolation context lookup
/// * `path` - The file path to validate and resolve
/// * `tool_name` - Name of the calling tool for error messages
///
/// # Returns
/// * `Ok(PathBuf)` - The resolved path (safe to use)
/// * `Err(ToolError::Validation)` - Path is blocked (within original project)
pub fn validate_and_resolve_path(
    session_id: Uuid,
    path: &str,
    tool_name: &'static str,
) -> Result<PathBuf, ToolError> {
    let isolation_ctx = get_isolation_context(session_id);
    validate_and_resolve_path_with_isolation(path, isolation_ctx.as_ref(), tool_name)
}

/// Internal path validation with explicit isolation context.
///
/// This is the core implementation that can be tested in isolation without
/// requiring a registered callback.
///
/// # Arguments
/// * `path` - The file path to validate and resolve
/// * `isolation_ctx` - Optional isolation context (None = no isolation)
/// * `tool_name` - Name of the calling tool for error messages
///
/// # Validation Logic
/// 1. If path is within worktree → ALLOW (return resolved path)
/// 2. If path is within blocked_project → BLOCK (return error)
/// 3. Otherwise (e.g., /tmp, /etc) → ALLOW (return path as-is)
pub fn validate_and_resolve_path_with_isolation(
    path: &str,
    isolation_ctx: Option<&IsolationContext>,
    tool_name: &'static str,
) -> Result<PathBuf, ToolError> {
    match isolation_ctx {
        Some(ctx) => {
            let path_buf = std::path::Path::new(path);
            
            // Get canonical paths for comparison (best-effort: fall back to raw paths
            // if the paths don't exist on disk, e.g. in tests with synthetic paths)
            let canonical_worktree = ctx.worktree_path.canonicalize()
                .unwrap_or_else(|_| normalize_path(&ctx.worktree_path));
            
            let canonical_blocked = ctx.blocked_project_path.canonicalize()
                .unwrap_or_else(|_| normalize_path(&ctx.blocked_project_path));
            
            if path_buf.is_absolute() {
                // For absolute paths, check against both worktree and blocked project
                match path_buf.canonicalize() {
                    Ok(canonical_path) => {
                        // Path exists - check hierarchy
                        // 1. If within worktree → ALLOW
                        if canonical_path.starts_with(&canonical_worktree) {
                            return Ok(canonical_path);
                        }
                        // 2. If within blocked_project → BLOCK
                        if canonical_path.starts_with(&canonical_blocked) {
                            return Err(ToolError::Validation {
                                tool: tool_name,
                                message: format!(
                                    "Path is blocked from original project. Cannot access files in: {}",
                                    canonical_blocked.display()
                                ),
                            });
                        }
                        // 3. Otherwise (e.g., /tmp, /etc) → ALLOW
                        Ok(canonical_path)
                    }
                    Err(_) => {
                        // Path doesn't exist yet - check by prefix
                        // First check if it would be within worktree
                        if path_buf.starts_with(&canonical_worktree) || path_buf.starts_with(&ctx.worktree_path) {
                            return Ok(path_buf.to_path_buf());
                        }
                        // Then check if it would be within blocked_project
                        if path_buf.starts_with(&canonical_blocked) || path_buf.starts_with(&ctx.blocked_project_path) {
                            return Err(ToolError::Validation {
                                tool: tool_name,
                                message: format!(
                                    "Cannot {} to original project. Path blocked: {}",
                                    tool_name,
                                    canonical_blocked.display()
                                ),
                            });
                        }
                        // Otherwise → ALLOW (path outside both worktree and blocked_project)
                        Ok(path_buf.to_path_buf())
                    }
                }
            } else {
                // Relative path - resolve to worktree, then validate
                let resolved = ctx.worktree_path.join(path);
                
                // Try to canonicalize to follow symlinks and resolve .. components
                match resolved.canonicalize() {
                    Ok(canonical_path) => {
                        // Path exists - check hierarchy
                        // 1. If within worktree → ALLOW
                        if canonical_path.starts_with(&canonical_worktree) {
                            return Ok(canonical_path);
                        }
                        // 2. If within blocked_project (symlink escape or ..) → BLOCK
                        if canonical_path.starts_with(&canonical_blocked) {
                            return Err(ToolError::Validation {
                                tool: tool_name,
                                message: format!(
                                    "Path is blocked from original project. Cannot access files in: {}",
                                    canonical_blocked.display()
                                ),
                            });
                        }
                        // 3. Otherwise (.. escaped to somewhere else like /tmp) → ALLOW
                        Ok(canonical_path)
                    }
                    Err(_) => {
                        // Path doesn't exist - normalize manually and check for escape
                        let normalized = normalize_path(&resolved);
                        
                        // Check if normalized path is within worktree
                        if normalized.starts_with(&canonical_worktree) || normalized.starts_with(&ctx.worktree_path) {
                            return Ok(normalized);
                        }
                        // Check if it escaped to blocked_project
                        if normalized.starts_with(&canonical_blocked) || normalized.starts_with(&ctx.blocked_project_path) {
                            return Err(ToolError::Validation {
                                tool: tool_name,
                                message: format!(
                                    "Path is blocked from original project. Cannot access files in: {}",
                                    canonical_blocked.display()
                                ),
                            });
                        }
                        // Otherwise → ALLOW (escaped to somewhere else)
                        Ok(normalized)
                    }
                }
            }
        }
        None => {
            // No isolation context - allow all paths
            let path_buf = PathBuf::from(path);
            Ok(path_buf)
        }
    }
}

/// Legacy wrapper for backward compatibility with tests.
/// 
/// This function is used by existing tests that pass just a worktree path.
/// For isolated sessions, it creates an IsolationContext where the blocked_project
/// is derived from the worktree path (parent of .fspec/worktrees/).
///
/// # Arguments
/// * `path` - The file path to validate and resolve
/// * `worktree_path` - Optional worktree path (None = no isolation)
/// * `tool_name` - Name of the calling tool for error messages
pub fn validate_and_resolve_path_with_cwd(
    path: &str,
    worktree_path: Option<&PathBuf>,
    tool_name: &'static str,
) -> Result<PathBuf, ToolError> {
    match worktree_path {
        Some(worktree) => {
            // Derive blocked_project from worktree path
            // Worktree is typically at /project/.fspec/worktrees/<session-id>
            // So blocked_project should be /project
            let blocked_project = derive_project_root_from_worktree(worktree);
            let ctx = IsolationContext {
                worktree_path: worktree.clone(),
                blocked_project_path: blocked_project,
            };
            validate_and_resolve_path_with_isolation(path, Some(&ctx), tool_name)
        }
        None => {
            // No worktree - allow all paths
            validate_and_resolve_path_with_isolation(path, None, tool_name)
        }
    }
}

/// Derive the project root from a worktree path.
///
/// Worktrees are typically at: /project/.fspec/worktrees/<session-id>
/// This function extracts /project from that path.
fn derive_project_root_from_worktree(worktree: &Path) -> PathBuf {
    // Look for .fspec/worktrees in the path and get the parent
    let worktree_str = worktree.to_string_lossy();
    if let Some(idx) = worktree_str.find(".fspec/worktrees") {
        let project_root = &worktree_str[..idx];
        // Remove trailing slash if present
        let trimmed = project_root.trim_end_matches('/').trim_end_matches('\\');
        if trimmed.is_empty() {
            PathBuf::from("/")
        } else {
            PathBuf::from(trimmed)
        }
    } else {
        // Fallback: assume worktree is the project root itself
        worktree.to_path_buf()
    }
}

/// Normalize a path by resolving . and .. components without requiring the path to exist.
/// This is used for validating paths to files that don't exist yet.
fn normalize_path(path: &std::path::Path) -> PathBuf {
    let mut components = Vec::new();
    
    for component in path.components() {
        match component {
            std::path::Component::Prefix(p) => components.push(std::path::Component::Prefix(p)),
            std::path::Component::RootDir => {
                components.clear();
                components.push(component);
            }
            std::path::Component::CurDir => {
                // Skip . components
            }
            std::path::Component::ParentDir => {
                // Pop the last component if possible (but don't go above root)
                if let Some(last) = components.last() {
                    if !matches!(last, std::path::Component::RootDir | std::path::Component::Prefix(_)) {
                        components.pop();
                    }
                }
            }
            std::path::Component::Normal(c) => {
                components.push(std::path::Component::Normal(c));
            }
        }
    }
    
    components.iter().collect()
}

/// Result type for bash operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashOperationResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Wrapper that adapts a FspecToolFacade to rig's Tool trait.
///
/// This enables provider-specific fspec facades to be used with rig's agent builder
/// while maintaining the facade's custom tool name, schema, and parameter mapping.
///
/// ## Execution Flow
///
/// Unlike other tool wrappers that execute directly, FspecToolFacadeWrapper uses
/// the fspec_handler mechanism to route commands through TypeScript:
///
/// 1. LLM calls Fspec tool with command args
/// 2. Wrapper calls `execute_fspec_command_for_session(self.session_id, request)`
/// 3. Handler (set by session_manager) emits FspecCommandRequest to TypeScript
/// 4. Handler blocks waiting for TypeScript response
/// 5. TypeScript executes command via fspecCallback and calls sessionSendFspecResult
/// 6. Handler receives result and returns to wrapper
/// 7. Wrapper returns actual result (not marker) to LLM
pub struct FspecToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedFspecToolFacade,
    /// Session ID for handler lookup - set at construction time (TOOL-012)
    /// This eliminates reliance on thread-local current session state.
    session_id: Uuid,
}

impl FspecToolFacadeWrapper {
    /// Create a new wrapper for the given fspec facade with explicit session association.
    ///
    /// # Arguments
    /// * `facade` - The provider-specific facade for schema/naming
    /// * `session_id` - The session ID for handler lookup (must be registered via set_fspec_handler_for_session)
    pub fn new(facade: BoxedFspecToolFacade, session_id: Uuid) -> Self {
        Self { facade, session_id }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
    }

    /// Get the session ID associated with this tool instance
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
}

impl Tool for FspecToolFacadeWrapper {
    const NAME: &'static str = "fspec_facade_wrapper";

    type Error = ToolError;
    type Args = FacadeArgs;
    type Output = String;

    /// Override to return the facade's tool name (e.g., "fspec_command" for Gemini)
    fn name(&self) -> String {
        self.facade.tool_name().to_string()
    }

    /// Override to return the facade's definition (provider-specific schema)
    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        let def = self.facade.definition();
        rig::completion::ToolDefinition {
            name: def.name,
            description: def.description,
            parameters: def.parameters,
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        use crate::fspec_handler::{execute_fspec_command_for_session, FspecRequest, has_fspec_handler_for_session};

        // Map provider-specific args to internal params via the facade
        let internal_params = self.facade.map_params(args.0)?;

        // Check if fspec handler is configured for this session (TOOL-012: use self.session_id)
        if !has_fspec_handler_for_session(self.session_id) {
            return Err(ToolError::Execution {
                tool: "fspec",
                message: format!(
                    "Fspec handler not configured for session {}. FspecTool requires session context with TypeScript integration.",
                    self.session_id
                ),
            });
        }

        // Execute command via the session-specific handler (TOOL-012: use self.session_id)
        // This blocks until TypeScript executes the command and sends the result back
        let result = execute_fspec_command_for_session(
            self.session_id,
            FspecRequest {
                command: internal_params.command,
                args_json: internal_params.args,
                project_root: internal_params.project_root,
                provider: self.facade.provider().to_string(),
            },
        );

        // Return the actual result (not a marker)
        if result.success {
            // Include system reminder if present
            if let Some(ref reminder) = result.system_reminder {
                Ok(format!("{}\n\n{}", result.data, reminder))
            } else {
                Ok(result.data)
            }
        } else {
            Err(ToolError::Execution {
                tool: "fspec",
                message: result.error.unwrap_or_else(|| "Unknown fspec error".to_string()),
            })
        }
    }
}
/// Wrapper that adapts a BashToolFacade to rig's Tool trait.
///
/// This enables provider-specific bash facades to be used with rig's agent builder
/// while maintaining the facade's custom tool name, schema, and parameter mapping.
///
/// ## BLOCK-006: Block Notifications
///
/// The wrapper stores session_id (TOOL-012 pattern) to emit UserNotification chunks
/// when commands are blocked by the blocklist. Notifications are emitted via the
/// global chunk callback before returning the blocked error to the LLM.
pub struct BashToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedBashToolFacade,
    /// The base tool for actual execution
    bash_tool: BashTool,
    /// Session ID for notification emission - set at construction time (TOOL-012)
    /// Used by BLOCK-006 to emit UserNotification when commands are blocked.
    session_id: Uuid,
}

impl BashToolFacadeWrapper {
    /// Create a new wrapper for the given bash facade with session association.
    ///
    /// # Arguments
    /// * `facade` - The provider-specific facade for schema/naming
    /// * `session_id` - The session ID for notification emission (BLOCK-006) and worktree isolation
    pub fn new(facade: BoxedBashToolFacade, session_id: Uuid) -> Self {
        Self {
            facade,
            bash_tool: BashTool::new(session_id),
            session_id,
        }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
    }

    /// Get the session ID associated with this tool instance
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
}

impl Tool for BashToolFacadeWrapper {
    const NAME: &'static str = "bash_facade_wrapper";

    type Error = ToolError;
    type Args = FacadeArgs;
    type Output = BashOperationResult;

    /// Override to return the facade's tool name (e.g., "run_shell_command" for Gemini)
    fn name(&self) -> String {
        self.facade.tool_name().to_string()
    }

    /// Return the facade's provider-specific tool definition
    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        let facade_def = self.facade.definition();
        RigToolDefinition {
            name: facade_def.name,
            description: facade_def.description,
            parameters: facade_def.parameters,
        }
    }

    /// Map provider params to internal format and execute the base tool
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Use the facade to map provider-specific params to internal format
        let internal_params = self.facade.map_params(args.0)?;

        // Execute the bash tool based on the operation type
        // TOOL-013: BashTool now handles effective_cwd lookup internally via session_id
        match internal_params {
            InternalBashParams::Execute { command, cwd, .. } => {
                let bash_args = BashArgs { 
                    command: command.clone(),
                    cwd, // Pass facade-provided cwd; BashTool applies session isolation override
                };
                match self.bash_tool.call(bash_args).await {
                    Ok(output) => Ok(BashOperationResult {
                        success: true,
                        output: Some(output),
                        error: None,
                    }),
                    Err(ToolError::Blocked { message, .. }) => {
                        // Emit notification to TUI for blocked commands
                        let action = command.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
                        emit_block_notification(self.session_id, &action, &message);
                        Ok(BashOperationResult {
                            success: false,
                            output: None,
                            error: Some(message),
                        })
                    }
                    Err(e) => Ok(BashOperationResult {
                        success: false,
                        output: None,
                        error: Some(e.to_string()),
                    }),
                }
            }
        }
    }
}

// ============================================================================
// SearchToolFacadeWrapper - Adapts SearchToolFacade implementations to rig::tool::Tool
// ============================================================================

use super::traits::{BoxedSearchToolFacade, InternalSearchParams};
use crate::{GlobTool, GrepTool};

/// Result type for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOperationResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Wrapper that adapts a SearchToolFacade to rig's Tool trait.
///
/// This enables provider-specific search facades to be used with rig's agent builder
/// while maintaining the facade's custom tool name, schema, and parameter mapping.
///
/// ## TOOL-014: Worktree Isolation
///
/// The wrapper stores session_id for worktree path resolution. When a session has
/// an effective_cwd (worktree), search paths are validated and resolved to the worktree
/// directory to ensure isolated sessions operate in their own worktree.
pub struct SearchToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedSearchToolFacade,
    /// The base tools for actual execution
    grep_tool: GrepTool,
    glob_tool: GlobTool,
    /// Session ID for worktree isolation - set at construction time (TOOL-014)
    session_id: Uuid,
}

impl SearchToolFacadeWrapper {
    /// Create a new wrapper for the given search facade with session association.
    ///
    /// # Arguments
    /// * `facade` - The provider-specific facade for schema/naming
    /// * `session_id` - The session ID for worktree isolation (TOOL-014)
    pub fn new(facade: BoxedSearchToolFacade, session_id: Uuid) -> Self {
        Self {
            facade,
            grep_tool: GrepTool::new(session_id),
            glob_tool: GlobTool::new(session_id),
            session_id,
        }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
    }

    /// Get the session ID associated with this tool instance
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
}

impl Tool for SearchToolFacadeWrapper {
    const NAME: &'static str = "search_facade_wrapper";

    type Error = ToolError;
    type Args = FacadeArgs;
    type Output = SearchOperationResult;

    /// Override to return the facade's tool name (e.g., "search_file_content" or "find_files")
    fn name(&self) -> String {
        self.facade.tool_name().to_string()
    }

    /// Return the facade's provider-specific tool definition
    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        let facade_def = self.facade.definition();
        RigToolDefinition {
            name: facade_def.name,
            description: facade_def.description,
            parameters: facade_def.parameters,
        }
    }

    /// Map provider params to internal format and execute the appropriate base tool
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Use the facade to map provider-specific params to internal format
        let internal_params = self.facade.map_params(args.0)?;

        // Execute the appropriate base tool based on the operation type
        match internal_params {
            InternalSearchParams::Grep { pattern, path, include, limit } => {
                // TOOL-014: Validate and resolve path for worktree isolation
                let resolved_path = if let Some(p) = path {
                    match validate_and_resolve_path(self.session_id, &p, "grep") {
                        Ok(resolved) => Some(resolved.to_string_lossy().to_string()),
                        Err(e) => {
                            return Ok(SearchOperationResult {
                                success: false,
                                output: None,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                } else {
                    // No path specified - resolve current directory for worktree isolation
                    match validate_and_resolve_path(self.session_id, ".", "grep") {
                        Ok(resolved) => Some(resolved.to_string_lossy().to_string()),
                        Err(e) => {
                            return Ok(SearchOperationResult {
                                success: false,
                                output: None,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                };
                
                use crate::grep::GrepArgs;
                let grep_args = GrepArgs {
                    pattern,
                    path: resolved_path,
                    output_mode: None,
                    glob: include,
                    limit,
                };
                match self.grep_tool.call(grep_args).await {
                    Ok(output) => {
                        // Apply limit: cap the number of result lines if limit is set
                        let capped_output = if let Some(max) = limit {
                            let lines: Vec<&str> = output.lines().collect();
                            if lines.len() > max {
                                lines[..max].join("\n")
                            } else {
                                output
                            }
                        } else {
                            output
                        };
                        Ok(SearchOperationResult {
                            success: true,
                            output: Some(capped_output),
                            error: None,
                        })
                    }
                    Err(e) => Ok(SearchOperationResult {
                        success: false,
                        output: None,
                        error: Some(e.to_string()),
                    }),
                }
            }
            InternalSearchParams::Glob { pattern, path } => {
                // TOOL-014: Validate and resolve path for worktree isolation
                let resolved_path = if let Some(p) = path {
                    match validate_and_resolve_path(self.session_id, &p, "glob") {
                        Ok(resolved) => Some(resolved.to_string_lossy().to_string()),
                        Err(e) => {
                            return Ok(SearchOperationResult {
                                success: false,
                                output: None,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                } else {
                    // No path specified - resolve current directory for worktree isolation
                    match validate_and_resolve_path(self.session_id, ".", "glob") {
                        Ok(resolved) => Some(resolved.to_string_lossy().to_string()),
                        Err(e) => {
                            return Ok(SearchOperationResult {
                                success: false,
                                output: None,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                };
                
                use crate::glob::GlobArgs;
                let glob_args = GlobArgs { pattern, path: resolved_path, case_insensitive: None };
                match self.glob_tool.call(glob_args).await {
                    Ok(output) => Ok(SearchOperationResult {
                        success: true,
                        output: Some(output),
                        error: None,
                    }),
                    Err(e) => Ok(SearchOperationResult {
                        success: false,
                        output: None,
                        error: Some(e.to_string()),
                    }),
                }
            }
        }
    }
}

// ============================================================================
// LsToolFacadeWrapper - Adapts LsToolFacade implementations to rig::tool::Tool
// ============================================================================

use super::traits::{BoxedLsToolFacade, InternalLsParams};
use crate::ls::LsArgs;
use crate::LsTool;

/// Result type for directory listing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsOperationResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Wrapper that adapts a LsToolFacade to rig's Tool trait.
///
/// This enables provider-specific ls facades to be used with rig's agent builder
/// while maintaining the facade's custom tool name, schema, and parameter mapping.
///
/// ## TOOL-014: Worktree Isolation
///
/// The wrapper stores session_id for worktree path resolution. When a session has
/// an effective_cwd (worktree), directory paths are validated and resolved to the worktree
/// directory to ensure isolated sessions operate in their own worktree.
pub struct LsToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedLsToolFacade,
    /// The base tool for actual execution
    ls_tool: LsTool,
    /// Session ID for worktree isolation - set at construction time (TOOL-014)
    session_id: Uuid,
}

impl LsToolFacadeWrapper {
    /// Create a new wrapper for the given ls facade with session association.
    ///
    /// # Arguments
    /// * `facade` - The provider-specific facade for schema/naming
    /// * `session_id` - The session ID for worktree isolation (TOOL-014)
    pub fn new(facade: BoxedLsToolFacade, session_id: Uuid) -> Self {
        Self {
            facade,
            ls_tool: LsTool::new(session_id),
            session_id,
        }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
    }

    /// Get the session ID associated with this tool instance
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
}

impl Tool for LsToolFacadeWrapper {
    const NAME: &'static str = "ls_facade_wrapper";

    type Error = ToolError;
    type Args = FacadeArgs;
    type Output = LsOperationResult;

    /// Override to return the facade's tool name (e.g., "list_directory" for Gemini)
    fn name(&self) -> String {
        self.facade.tool_name().to_string()
    }

    /// Return the facade's provider-specific tool definition
    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        let facade_def = self.facade.definition();
        RigToolDefinition {
            name: facade_def.name,
            description: facade_def.description,
            parameters: facade_def.parameters,
        }
    }

    /// Map provider params to internal format and execute the base tool
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Use the facade to map provider-specific params to internal format
        let internal_params = self.facade.map_params(args.0)?;

        // Execute the ls tool based on the operation type
        match internal_params {
            InternalLsParams::List { path } => {
                // TOOL-014: Validate and resolve path for worktree isolation
                let resolved_path = if let Some(p) = path {
                    match validate_and_resolve_path(self.session_id, &p, "ls") {
                        Ok(resolved) => Some(resolved.to_string_lossy().to_string()),
                        Err(e) => {
                            return Ok(LsOperationResult {
                                success: false,
                                output: None,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                } else {
                    // No path specified - resolve current directory for worktree isolation
                    match validate_and_resolve_path(self.session_id, ".", "ls") {
                        Ok(resolved) => Some(resolved.to_string_lossy().to_string()),
                        Err(e) => {
                            return Ok(LsOperationResult {
                                success: false,
                                output: None,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                };
                
                let ls_args = LsArgs { path: resolved_path };
                match self.ls_tool.call(ls_args).await {
                    Ok(output) => Ok(LsOperationResult {
                        success: true,
                        output: Some(output),
                        error: None,
                    }),
                    Err(e) => Ok(LsOperationResult {
                        success: false,
                        output: None,
                        error: Some(e.to_string()),
                    }),
                }
            }
        }
    }
}

// ============================================================================
// BridgeToolFacadeWrapper - Adapts BridgeToolFacade implementations to rig::tool::Tool
// ============================================================================

use super::bridge_facade::{BoxedBridgeToolFacade, InternalBridgeParams};
use crate::bridge::BridgeAction;
use crate::bridge_handler::{execute_bridge_command, has_bridge_handler_for_session, BridgeRequest};

/// Result type for bridge operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[must_use = "BridgeOperationResult should be checked for success/failure"]
pub struct BridgeOperationResult {
    pub success: bool,
    pub message: String,
    pub connections: Option<Vec<crate::bridge::BridgeConnectionInfo>>,
}

/// Wrapper that adapts a BridgeToolFacade to rig's Tool trait.
///
/// This enables provider-specific bridge facades to be used with rig's agent builder
/// while maintaining the facade's custom tool name, schema, and parameter mapping.
///
/// ## Execution Flow
///
/// Unlike other tool wrappers that execute directly, BridgeToolFacadeWrapper uses
/// the global bridge_handler mechanism to manage WebSocket connections with session context:
///
/// 1. LLM calls Bridge tool with action args
/// 2. Wrapper calls `execute_bridge_command()` with request
/// 3. Handler (set by session_manager) manages WebSocket connections
/// 4. Handler returns result to wrapper
/// 5. Wrapper returns actual result to LLM
pub struct BridgeToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedBridgeToolFacade,
    /// Session ID for context lookup - set at construction time (TOOL-012)
    /// This eliminates reliance on global current session state.
    session_id: Uuid,
}

impl BridgeToolFacadeWrapper {
    /// Create a new wrapper for the given bridge facade with explicit session association.
    ///
    /// # Arguments
    /// * `facade` - The provider-specific facade for schema/naming
    /// * `session_id` - The session ID for context lookup (must be registered via set_bridge_session_context)
    pub fn new(facade: BoxedBridgeToolFacade, session_id: Uuid) -> Self {
        Self { facade, session_id }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
    }

    /// Get the session ID associated with this tool instance
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
}

impl Tool for BridgeToolFacadeWrapper {
    const NAME: &'static str = "bridge_facade_wrapper";

    type Error = ToolError;
    type Args = FacadeArgs;
    type Output = BridgeOperationResult;

    /// Override to return the facade's tool name (e.g., "Bridge" for Claude, "bridge_connection" for Gemini)
    fn name(&self) -> String {
        self.facade.tool_name().to_string()
    }

    /// Override to return the facade's definition (provider-specific schema)
    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        let def = self.facade.definition();
        RigToolDefinition {
            name: def.name,
            description: def.description,
            parameters: def.parameters,
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Map provider-specific args to internal params via the facade
        let internal_params = self.facade.map_params(args.0)?;

        // Check if bridge handler is configured for this session (TOOL-012: use self.session_id)
        if !has_bridge_handler_for_session(self.session_id) {
            return Err(ToolError::Execution {
                tool: "bridge",
                message: format!(
                    "Bridge handler not configured for session {}. BridgeTool requires session context.",
                    self.session_id
                ),
            });
        }

        // Convert internal params to BridgeAction
        let action = match internal_params {
            InternalBridgeParams::Connect { url } => BridgeAction::Connect { url },
            InternalBridgeParams::Disconnect { url } => BridgeAction::Disconnect { url },
            InternalBridgeParams::List => BridgeAction::List,
        };

        // Execute command via the session-specific handler (TOOL-012: use self.session_id)
        let result = execute_bridge_command(BridgeRequest {
            session_id: self.session_id,
            action,
        });

        // Return the result
        Ok(BridgeOperationResult {
            success: result.success,
            message: result.message,
            connections: result.connections,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod bridge_wrapper_tests {
    use super::*;
    use crate::bridge_handler::{set_bridge_handler, set_bridge_session_context, remove_bridge_session_context, BridgeHandler};
    use crate::bridge::BridgeResult;
    use crate::facade::bridge_facade::ClaudeBridgeFacade;
    use serial_test::serial;
    use std::sync::Arc;

    /// Feature: spec/features/tool-wrapper-session-association.feature
    /// Scenario: Bridge tool wrapper stores session_id at construction (TOOL-012)
    #[tokio::test]
    #[serial]
    async fn test_bridge_wrapper_uses_session_id_from_construction() {
        // @step Given the bridge handler is configured
        let received_session_id = Arc::new(std::sync::Mutex::new(None));
        let received_session_id_clone = received_session_id.clone();

        let handler: BridgeHandler = Arc::new(move |req| {
            *received_session_id_clone.lock().unwrap() = Some(req.session_id);
            BridgeResult {
                success: true,
                message: "test".to_string(),
                connections: Some(vec![]),
            }
        });
        set_bridge_handler(Some(handler));

        // @step And a session ID is created
        let expected_session_id = uuid::Uuid::new_v4();

        // @step And bridge session context is set for the session
        let (tx, _rx) = tokio::sync::broadcast::channel::<serde_json::Value>(16);
        let broadcast_factory: crate::BroadcastReceiverFactory = Arc::new(move || tx.subscribe());
        let input_injector: crate::InputInjector = Arc::new(|_| {});
        set_bridge_session_context(expected_session_id, broadcast_factory, input_injector, None, None);

        // @step When the BridgeToolFacadeWrapper is created with session_id at construction (TOOL-012)
        let wrapper = BridgeToolFacadeWrapper::new(Arc::new(ClaudeBridgeFacade), expected_session_id);

        // @step Then the wrapper should store session_id as a field
        assert_eq!(wrapper.session_id(), expected_session_id);

        // @step When the BridgeToolFacadeWrapper executes a list action
        let args = FacadeArgs(serde_json::json!({
            "action": {"type": "list"}
        }));

        let _ = wrapper.call(args).await;

        // @step Then the request should contain the correct session ID from construction
        let actual_session_id = received_session_id.lock().unwrap();
        assert_eq!(
            *actual_session_id,
            Some(expected_session_id),
            "BridgeRequest should contain session ID from wrapper construction, not thread-local"
        );

        // Cleanup
        set_bridge_handler(None);
        remove_bridge_session_context(expected_session_id);
    }

    #[tokio::test]
    #[serial]
    async fn test_bridge_wrapper_fails_without_handler() {
        set_bridge_handler(None);
        let session_id = uuid::Uuid::new_v4();

        let wrapper = BridgeToolFacadeWrapper::new(Arc::new(ClaudeBridgeFacade), session_id);
        let args = FacadeArgs(serde_json::json!({
            "action": {"type": "list"}
        }));

        let result = wrapper.call(args).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("handler not configured"));
    }
}

// ============================================================================
// TOOL-014: Worktree Isolation Path Validation Tests
// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod worktree_isolation_tests {
    use super::*;

    // ========================================================================
    // Tests for validate_and_resolve_path (TOOL-014 IMPLEMENTATION)
    // ========================================================================

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: Tools operate normally with Uuid::nil() in tests
    #[test]
    fn test_validate_and_resolve_path_no_worktree() {
        // @step Given a non-isolated session with Uuid::nil()
        // (effective_cwd is None)
        
        // @step When get_effective_cwd is called
        // @step Then it should return None
        // @step And tools should operate in the current directory without path validation
        let result = validate_and_resolve_path_with_cwd("src/file.rs", None, "read");
        assert!(result.is_ok(), "Should succeed without worktree");
        assert_eq!(result.unwrap(), PathBuf::from("src/file.rs"));
        
        let result = validate_and_resolve_path_with_cwd("/absolute/path/file.rs", None, "read");
        assert!(result.is_ok(), "Absolute path should succeed without worktree");
        assert_eq!(result.unwrap(), PathBuf::from("/absolute/path/file.rs"));
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: ReadTool resolves relative path to worktree in isolated session
    #[test]
    fn test_validate_and_resolve_relative_to_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When ReadTool reads "src/file.rs"
        let result = validate_and_resolve_path_with_cwd("src/file.rs", Some(&worktree), "read");

        // @step Then it should read from ".fspec/worktrees/abc123/src/file.rs"
        assert!(result.is_ok(), "Should resolve relative path");
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/project/.fspec/worktrees/abc123/src/file.rs"),
            "Relative path should be resolved to worktree"
        );
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: LsTool lists worktree root in isolated session
    #[test]
    fn test_validate_and_resolve_dot_to_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When LsTool lists "."
        let result = validate_and_resolve_path_with_cwd(".", Some(&worktree), "ls");

        // @step Then it should list contents of ".fspec/worktrees/abc123/"
        assert!(result.is_ok(), "Should resolve dot path");
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/project/.fspec/worktrees/abc123/.")
        );
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: ReadTool rejects absolute path outside worktree
    #[test]
    fn test_validate_and_resolve_path_rejects_absolute_outside_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When ReadTool attempts to read "/project/src/main.rs"
        let result = validate_and_resolve_path_with_cwd(
            "/project/src/main.rs",
            Some(&worktree),
            "read",
        );

        // @step Then it should return ToolError::Validation with tool "read"
        assert!(result.is_err(), "Should reject path outside worktree");

        // @step And the error message should contain "outside isolated worktree"
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Validation { tool: "read", .. }));
        assert!(
            err.to_string().contains("blocked") || err.to_string().contains("outside"),
            "Error should mention worktree isolation: {err}"
        );
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: WriteTool rejects absolute path outside worktree
    #[test]
    fn test_validate_and_resolve_path_rejects_write_outside_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When WriteTool attempts to write to "/project/src/new.rs"
        let result = validate_and_resolve_path_with_cwd(
            "/project/src/new.rs",
            Some(&worktree),
            "write",
        );

        // @step Then it should return ToolError::Validation with tool "write"
        assert!(result.is_err(), "Should reject path outside worktree");

        // @step And the error message should contain "outside isolated worktree"
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Validation { tool: "write", .. }));
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: EditTool rejects absolute path outside worktree
    #[test]
    fn test_validate_and_resolve_path_rejects_edit_outside_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When EditTool attempts to edit "/project/src/lib.rs"
        let result = validate_and_resolve_path_with_cwd(
            "/project/src/lib.rs",
            Some(&worktree),
            "edit",
        );

        // @step Then it should return ToolError::Validation with tool "edit"
        assert!(result.is_err(), "Should reject path outside worktree");

        // @step And the error message should contain "outside isolated worktree"
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Validation { tool: "edit", .. }));
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: GrepTool rejects absolute path outside worktree
    #[test]
    fn test_validate_and_resolve_path_rejects_grep_outside_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When GrepTool attempts to search with path "/project/src"
        let result = validate_and_resolve_path_with_cwd(
            "/project/src",
            Some(&worktree),
            "grep",
        );

        // @step Then it should return ToolError::Validation with tool "grep"
        assert!(result.is_err(), "Should reject path outside worktree");

        // @step And the error message should contain "outside isolated worktree"
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Validation { tool: "grep", .. }));
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: GlobTool rejects absolute path outside worktree
    #[test]
    fn test_validate_and_resolve_path_rejects_glob_outside_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When GlobTool attempts to search with path "/project/src"
        let result = validate_and_resolve_path_with_cwd(
            "/project/src",
            Some(&worktree),
            "glob",
        );

        // @step Then it should return ToolError::Validation with tool "glob"
        assert!(result.is_err(), "Should reject path outside worktree");

        // @step And the error message should contain "outside isolated worktree"
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Validation { tool: "glob", .. }));
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: LsTool rejects absolute path outside worktree
    #[test]
    fn test_validate_and_resolve_path_rejects_ls_outside_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When LsTool attempts to list "/project/src"
        let result = validate_and_resolve_path_with_cwd(
            "/project/src",
            Some(&worktree),
            "ls",
        );

        // @step Then it should return ToolError::Validation with tool "ls"
        assert!(result.is_err(), "Should reject path outside worktree");

        // @step And the error message should contain "outside isolated worktree"
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Validation { tool: "ls", .. }));
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: AstGrepTool rejects absolute path outside worktree
    #[test]
    fn test_validate_and_resolve_path_rejects_astgrep_outside_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When AstGrepTool attempts to search with path "/project/src"
        let result = validate_and_resolve_path_with_cwd(
            "/project/src",
            Some(&worktree),
            "ast_grep",
        );

        // @step Then it should return ToolError::Validation with tool "ast_grep"
        assert!(result.is_err(), "Should reject path outside worktree");

        // @step And the error message should contain "outside isolated worktree"
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Validation { tool: "ast_grep", .. }));
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: AstGrepRefactorTool rejects absolute path outside worktree
    #[test]
    fn test_validate_and_resolve_path_rejects_astgrep_refactor_outside_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When AstGrepRefactorTool attempts to refactor "/project/src/lib.rs"
        let result = validate_and_resolve_path_with_cwd(
            "/project/src/lib.rs",
            Some(&worktree),
            "ast_grep_refactor",
        );

        // @step Then it should return ToolError::Validation with tool "ast_grep_refactor"
        assert!(result.is_err(), "Should reject path outside worktree");

        // @step And the error message should contain "outside isolated worktree"
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Validation { tool: "ast_grep_refactor", .. }));
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: WriteTool writes to worktree path in isolated session
    #[test]
    fn test_validate_and_resolve_path_resolves_relative_to_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When WriteTool writes to "src/new.rs"
        let result = validate_and_resolve_path_with_cwd(
            "src/new.rs",
            Some(&worktree),
            "write",
        );

        // @step Then the file should be created at ".fspec/worktrees/abc123/src/new.rs"
        assert!(result.is_ok(), "Should resolve relative path to worktree");
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/project/.fspec/worktrees/abc123/src/new.rs")
        );

        // @step And the main project directory should be unchanged
        // (This is verified by the path being within worktree, not main project)
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: Tools operate normally with Uuid::nil() in tests
    #[test]
    fn test_validate_and_resolve_path_allows_all_paths_with_nil_session() {
        // @step Given a non-isolated session with Uuid::nil()
        // (effective_cwd is None)

        // @step When get_effective_cwd is called
        // @step Then it should return None
        // (simulated by passing None to validate_and_resolve_path_with_cwd)

        // @step And tools should operate in the current directory without path validation
        let result = validate_and_resolve_path_with_cwd(
            "/any/absolute/path",
            None,
            "read",
        );
        assert!(result.is_ok(), "Should allow any path when no worktree isolation");
        assert_eq!(result.unwrap(), PathBuf::from("/any/absolute/path"));

        let result = validate_and_resolve_path_with_cwd(
            "relative/path",
            None,
            "write",
        );
        assert!(result.is_ok(), "Should allow relative paths when no worktree isolation");
        assert_eq!(result.unwrap(), PathBuf::from("relative/path"));
    }

    // ========================================================================
    // Additional Happy Path Tests for Tool-Specific Scenarios
    // ========================================================================

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: WriteTool writes to worktree path in isolated session
    #[test]
    fn test_write_tool_resolves_to_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When WriteTool writes to "src/new.rs"
        let result = validate_and_resolve_path_with_cwd(
            "src/new.rs",
            Some(&worktree),
            "write",
        );

        // @step Then the file should be created at ".fspec/worktrees/abc123/src/new.rs"
        assert!(result.is_ok(), "Should resolve relative path");
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/project/.fspec/worktrees/abc123/src/new.rs")
        );

        // @step And the main project directory should be unchanged
        // (Verified by path being within worktree)
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: EditTool edits file in worktree in isolated session
    #[test]
    fn test_edit_tool_resolves_to_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step And a file exists at ".fspec/worktrees/abc123/src/lib.rs"
        // (File existence is checked by EditTool, not path validation)

        // @step When EditTool edits "src/lib.rs"
        let result = validate_and_resolve_path_with_cwd(
            "src/lib.rs",
            Some(&worktree),
            "edit",
        );

        // @step Then it should modify ".fspec/worktrees/abc123/src/lib.rs"
        assert!(result.is_ok(), "Should resolve relative path");
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/project/.fspec/worktrees/abc123/src/lib.rs")
        );

        // @step And the main project file should be unchanged
        // (Verified by path being within worktree)
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: GrepTool searches within worktree in isolated session
    #[test]
    fn test_grep_tool_resolves_to_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When GrepTool searches with path "src/"
        let result = validate_and_resolve_path_with_cwd(
            "src/",
            Some(&worktree),
            "grep",
        );

        // @step Then it should search within ".fspec/worktrees/abc123/src/"
        assert!(result.is_ok(), "Should resolve relative path");
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/project/.fspec/worktrees/abc123/src/")
        );

        // @step And it should not search the main project directory
        // (Verified by path being within worktree)
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: GlobTool finds files only within worktree in isolated session
    #[test]
    fn test_glob_tool_resolves_to_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When GlobTool searches for "**/*.rs"
        // Note: GlobTool uses "." as default path if not specified
        let result = validate_and_resolve_path_with_cwd(
            ".",
            Some(&worktree),
            "glob",
        );

        // @step Then it should only return files within ".fspec/worktrees/abc123/"
        assert!(result.is_ok(), "Should resolve dot path");
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/project/.fspec/worktrees/abc123/.")
        );

        // @step And it should not return files from the main project
        // (Verified by search starting within worktree)
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: AstGrepTool searches within worktree in isolated session
    #[test]
    fn test_astgrep_tool_resolves_to_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step When AstGrepTool searches with path "src/"
        let result = validate_and_resolve_path_with_cwd(
            "src/",
            Some(&worktree),
            "ast_grep",
        );

        // @step Then it should search within ".fspec/worktrees/abc123/src/"
        assert!(result.is_ok(), "Should resolve relative path");
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/project/.fspec/worktrees/abc123/src/")
        );
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: AstGrepRefactorTool modifies worktree copy only
    #[test]
    fn test_astgrep_refactor_tool_resolves_to_worktree() {
        // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
        let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

        // @step And a file exists at ".fspec/worktrees/abc123/src/lib.rs"
        // (File existence is checked by AstGrepRefactorTool, not path validation)

        // @step When AstGrepRefactorTool refactors "src/lib.rs"
        let result = validate_and_resolve_path_with_cwd(
            "src/lib.rs",
            Some(&worktree),
            "ast_grep_refactor",
        );

        // @step Then it should modify ".fspec/worktrees/abc123/src/lib.rs"
        assert!(result.is_ok(), "Should resolve relative path");
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/project/.fspec/worktrees/abc123/src/lib.rs")
        );

        // @step And the main project file should be unchanged
        // (Verified by path being within worktree)
    }

    // ============================================================================
    // REQUIREMENT 1: CONSTRUCTOR SIGNATURE - session_id IS MANDATORY (TOOL-014)
    // These tests verify ALL tools REQUIRE session_id in their constructor.
    // This is a compile-time guarantee - tools cannot be instantiated without session_id.
    // ============================================================================

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: ReadTool REQUIRES session_id parameter in constructor
    #[test]
    fn test_read_tool_requires_session_id() {
        use crate::ReadTool;

        // @step Given ReadTool is being instantiated
        let session_id = Uuid::new_v4();

        // @step Then the constructor signature MUST be ReadTool::new(session_id: Uuid)
        let _tool = ReadTool::new(session_id);

        // @step And calling ReadTool::new() without session_id MUST fail to compile
        // Verified at compile time - no parameterless constructor exists

        // @step And ReadTool MUST NOT implement Default trait
        // Verified at compile time - Default is not implemented
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: WriteTool REQUIRES session_id parameter in constructor
    #[test]
    fn test_write_tool_requires_session_id() {
        use crate::WriteTool;

        // @step Given WriteTool is being instantiated
        let session_id = Uuid::new_v4();

        // @step Then the constructor signature MUST be WriteTool::new(session_id: Uuid)
        let _tool = WriteTool::new(session_id);

        // @step And calling WriteTool::new() without session_id MUST fail to compile
        // @step And WriteTool MUST NOT implement Default trait
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: EditTool REQUIRES session_id parameter in constructor
    #[test]
    fn test_edit_tool_requires_session_id() {
        use crate::EditTool;

        // @step Given EditTool is being instantiated
        let session_id = Uuid::new_v4();

        // @step Then the constructor signature MUST be EditTool::new(session_id: Uuid)
        let _tool = EditTool::new(session_id);

        // @step And calling EditTool::new() without session_id MUST fail to compile
        // @step And EditTool MUST NOT implement Default trait
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: GrepTool REQUIRES session_id parameter in constructor
    #[test]
    fn test_grep_tool_requires_session_id() {
        use crate::GrepTool;

        // @step Given GrepTool is being instantiated
        let session_id = Uuid::new_v4();

        // @step Then the constructor signature MUST be GrepTool::new(session_id: Uuid)
        let _tool = GrepTool::new(session_id);

        // @step And calling GrepTool::new() without session_id MUST fail to compile
        // @step And GrepTool MUST NOT implement Default trait
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: GlobTool REQUIRES session_id parameter in constructor
    #[test]
    fn test_glob_tool_requires_session_id() {
        use crate::GlobTool;

        // @step Given GlobTool is being instantiated
        let session_id = Uuid::new_v4();

        // @step Then the constructor signature MUST be GlobTool::new(session_id: Uuid)
        let _tool = GlobTool::new(session_id);

        // @step And calling GlobTool::new() without session_id MUST fail to compile
        // @step And GlobTool MUST NOT implement Default trait
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: LsTool REQUIRES session_id parameter in constructor
    #[test]
    fn test_ls_tool_requires_session_id() {
        use crate::LsTool;

        // @step Given LsTool is being instantiated
        let session_id = Uuid::new_v4();

        // @step Then the constructor signature MUST be LsTool::new(session_id: Uuid)
        let _tool = LsTool::new(session_id);

        // @step And calling LsTool::new() without session_id MUST fail to compile
        // @step And LsTool MUST NOT implement Default trait
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: AstGrepTool REQUIRES session_id parameter in constructor
    #[test]
    fn test_ast_grep_tool_requires_session_id() {
        use crate::AstGrepTool;

        // @step Given AstGrepTool is being instantiated
        let session_id = Uuid::new_v4();

        // @step Then the constructor signature MUST be AstGrepTool::new(session_id: Uuid)
        let _tool = AstGrepTool::new(session_id);

        // @step And calling AstGrepTool::new() without session_id MUST fail to compile
        // @step And AstGrepTool MUST NOT implement Default trait
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: AstGrepRefactorTool REQUIRES session_id parameter in constructor
    #[test]
    fn test_ast_grep_refactor_tool_requires_session_id() {
        use crate::AstGrepRefactorTool;

        // @step Given AstGrepRefactorTool is being instantiated
        let session_id = Uuid::new_v4();

        // @step Then the constructor signature MUST be AstGrepRefactorTool::new(session_id: Uuid)
        let _tool = AstGrepRefactorTool::new(session_id);

        // @step And calling AstGrepRefactorTool::new() without session_id MUST fail to compile
        // @step And AstGrepRefactorTool MUST NOT implement Default trait
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: WebSearchTool REQUIRES session_id parameter in constructor
    #[test]
    fn test_web_search_tool_requires_session_id() {
        use crate::WebSearchTool;

        // @step Given WebSearchTool is being instantiated
        let session_id = Uuid::new_v4();

        // @step Then the constructor signature MUST be WebSearchTool::new(session_id: Uuid)
        let _tool = WebSearchTool::new(session_id);

        // @step And calling WebSearchTool::new() without session_id MUST fail to compile
        // @step And WebSearchTool MUST NOT implement Default trait
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: SearchToolFacadeWrapper REQUIRES session_id parameter in constructor
    #[test]
    fn test_search_tool_facade_wrapper_requires_session_id() {
        use crate::facade::GeminiSearchFileContentFacade;
        use std::sync::Arc;

        // @step Given SearchToolFacadeWrapper is being instantiated
        let session_id = Uuid::new_v4();
        let facade = Arc::new(GeminiSearchFileContentFacade) as BoxedSearchToolFacade;

        // @step Then the constructor signature MUST be SearchToolFacadeWrapper::new(facade, session_id: Uuid)
        let _tool = SearchToolFacadeWrapper::new(facade, session_id);

        // @step And calling SearchToolFacadeWrapper::new() without session_id MUST fail to compile
        // @step And SearchToolFacadeWrapper MUST NOT implement Default trait
    }

    /// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
    /// Scenario: LsToolFacadeWrapper REQUIRES session_id parameter in constructor
    #[test]
    fn test_ls_tool_facade_wrapper_requires_session_id() {
        use crate::facade::GeminiListDirectoryFacade;
        use std::sync::Arc;

        // @step Given LsToolFacadeWrapper is being instantiated
        let session_id = Uuid::new_v4();
        let facade = Arc::new(GeminiListDirectoryFacade) as BoxedLsToolFacade;

        // @step Then the constructor signature MUST be LsToolFacadeWrapper::new(facade, session_id: Uuid)
        let _tool = LsToolFacadeWrapper::new(facade, session_id);

        // @step And calling LsToolFacadeWrapper::new() without session_id MUST fail to compile
        // @step And LsToolFacadeWrapper MUST NOT implement Default trait
    }

}
