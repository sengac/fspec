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

/// Wrapper that adapts a ToolFacade to rig's Tool trait.
///
/// This enables provider-specific facades to be used with rig's agent builder
/// while maintaining the facade's custom tool name, schema, and parameter mapping.
pub struct FacadeToolWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedToolFacade,
    /// The base web search tool for actual execution
    base_tool: WebSearchTool,
}

/// Arguments for the facade wrapper - accepts raw JSON for flexible param mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacadeArgs(pub Value);

impl FacadeToolWrapper {
    /// Create a new wrapper for the given facade
    pub fn new(facade: BoxedToolFacade) -> Self {
        Self {
            facade,
            base_tool: WebSearchTool::new(),
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
        let wrapper = FacadeToolWrapper::new(facade);

        assert_eq!(wrapper.name(), "google_web_search");
    }

    #[test]
    fn test_wrapper_returns_facade_tool_name_web_fetch() {
        let facade = Arc::new(GeminiWebFetchFacade) as BoxedToolFacade;
        let wrapper = FacadeToolWrapper::new(facade);

        assert_eq!(wrapper.name(), "web_fetch");
    }

    #[tokio::test]
    async fn test_wrapper_returns_flat_schema_for_gemini() {
        let facade = Arc::new(GeminiGoogleWebSearchFacade) as BoxedToolFacade;
        let wrapper = FacadeToolWrapper::new(facade);

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
    /// * `session_id` - The session ID for notification emission (BLOCK-006)
    pub fn new(facade: BoxedFileToolFacade, session_id: Uuid) -> Self {
        Self {
            facade,
            read_tool: ReadTool::new(),
            write_tool: WriteTool::new(),
            edit_tool: EditTool::new(),
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
            } => {
                use crate::read::ReadArgs;
                let read_args = ReadArgs {
                    file_path,
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
                // BLOCK-006: Check stage permissions before write
                // Get the current work unit's stage from the session
                let stage = get_work_unit_stage(self.session_id);
                if let Err(blocked) = check_write_permission(&file_path, stage.as_deref()) {
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
                let write_args = WriteArgs { file_path, content };
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
                // BLOCK-006: Check stage permissions before edit (same as write)
                // Get the current work unit's stage from the session
                let stage = get_work_unit_stage(self.session_id);
                if let Err(blocked) = check_write_permission(&file_path, stage.as_deref()) {
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
                    file_path,
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

/// Global callback for emitting block notifications.
/// Set by codelet-napi during session initialization.
static BLOCK_NOTIFICATION_CALLBACK: std::sync::OnceLock<BlockNotificationCallback> =
    std::sync::OnceLock::new();

/// Global callback for getting work unit stage from session.
/// Set by codelet-napi during initialization.
static GET_WORK_UNIT_STAGE_CALLBACK: std::sync::OnceLock<GetWorkUnitStageCallback> =
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
    /// * `session_id` - The session ID for notification emission (BLOCK-006)
    pub fn new(facade: BoxedBashToolFacade, session_id: Uuid) -> Self {
        Self {
            facade,
            bash_tool: BashTool::new(),
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
        match internal_params {
            InternalBashParams::Execute { command } => {
                let bash_args = BashArgs { command: command.clone() };
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
pub struct SearchToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedSearchToolFacade,
    /// The base tools for actual execution
    grep_tool: GrepTool,
    glob_tool: GlobTool,
}

impl SearchToolFacadeWrapper {
    /// Create a new wrapper for the given search facade
    pub fn new(facade: BoxedSearchToolFacade) -> Self {
        Self {
            facade,
            grep_tool: GrepTool::new(),
            glob_tool: GlobTool::new(),
        }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
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
            InternalSearchParams::Grep { pattern, path } => {
                use crate::grep::GrepArgs;
                let grep_args = GrepArgs {
                    pattern,
                    path,
                    output_mode: None,
                };
                match self.grep_tool.call(grep_args).await {
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
            InternalSearchParams::Glob { pattern, path } => {
                use crate::glob::GlobArgs;
                let glob_args = GlobArgs { pattern, path, case_insensitive: None };
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
pub struct LsToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedLsToolFacade,
    /// The base tool for actual execution
    ls_tool: LsTool,
}

impl LsToolFacadeWrapper {
    /// Create a new wrapper for the given ls facade
    pub fn new(facade: BoxedLsToolFacade) -> Self {
        Self {
            facade,
            ls_tool: LsTool::new(),
        }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
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
                let ls_args = LsArgs { path };
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
        set_bridge_session_context(expected_session_id, broadcast_factory, input_injector, None);

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
