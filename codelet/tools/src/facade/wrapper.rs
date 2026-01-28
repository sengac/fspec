//! FacadeToolWrapper - Adapts ToolFacade implementations to rig::tool::Tool trait.
//!
//! This wrapper enables facades to be used with rig's agent builder by implementing
//! the Tool trait and delegating to the underlying facade for schema/naming while
//! executing against the base tool implementation.

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
        let wrapper = FileToolFacadeWrapper::new(facade);

        assert_eq!(wrapper.name(), "read_file");
    }

    #[tokio::test]
    async fn test_file_wrapper_returns_flat_schema() {
        let facade = Arc::new(GeminiReadFileFacade) as BoxedFileToolFacade;
        let wrapper = FileToolFacadeWrapper::new(facade);

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
pub struct FileToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedFileToolFacade,
    /// The base tools for actual execution
    read_tool: ReadTool,
    write_tool: WriteTool,
    edit_tool: EditTool,
}

impl FileToolFacadeWrapper {
    /// Create a new wrapper for the given file facade
    pub fn new(facade: BoxedFileToolFacade) -> Self {
        Self {
            facade,
            read_tool: ReadTool::new(),
            write_tool: WriteTool::new(),
            edit_tool: EditTool::new(),
        }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
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
use crate::fspec::{FspecArgs, FspecTool};
use crate::bash::BashArgs;
use crate::BashTool;

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
pub struct FspecToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedFspecToolFacade,
    /// The base tool for actual execution
    fspec_tool: FspecTool,
}

impl FspecToolFacadeWrapper {
    /// Create a new wrapper for the given fspec facade
    pub fn new(facade: BoxedFspecToolFacade) -> Self {
        Self {
            facade,
            fspec_tool: FspecTool::new(),
        }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
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
        // Map provider-specific args to internal params via the facade
        let internal_params = self.facade.map_params(args.0)?;

        // Convert internal params to FspecArgs
        let fspec_args = FspecArgs {
            command: internal_params.command,
            args: internal_params.args,
            project_root: internal_params.project_root,
        };

        // Execute via the actual FspecTool
        self.fspec_tool.call(fspec_args).await
    }
}
///
/// This enables provider-specific bash facades to be used with rig's agent builder
/// while maintaining the facade's custom tool name, schema, and parameter mapping.
pub struct BashToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedBashToolFacade,
    /// The base tool for actual execution
    bash_tool: BashTool,
}

impl BashToolFacadeWrapper {
    /// Create a new wrapper for the given bash facade
    pub fn new(facade: BoxedBashToolFacade) -> Self {
        Self {
            facade,
            bash_tool: BashTool::new(),
        }
    }

    /// Get the facade's provider name
    pub fn provider(&self) -> &'static str {
        self.facade.provider()
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
                let bash_args = BashArgs { command };
                match self.bash_tool.call(bash_args).await {
                    Ok(output) => Ok(BashOperationResult {
                        success: true,
                        output: Some(output),
                        error: None,
                    }),
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
