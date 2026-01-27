//! Glob NAPI bindings
//!
//! Exposes glob file pattern matching functionality to TypeScript via NAPI-RS.
//! Reuses the existing GlobTool implementation from codelet-tools.

use codelet_tools::glob::{GlobArgs, GlobTool};
use napi_derive::napi;
use rig::tool::Tool;

/// Result of a Glob tool search
#[napi(object)]
pub struct GlobResult {
    /// Whether the search was successful
    pub success: bool,
    /// Matching file paths (one per line)
    pub data: Option<String>,
    /// Error message if search failed
    pub error: Option<String>,
}

/// Search for files matching a glob pattern
///
/// Uses the codelet Glob tool for gitignore-aware file pattern matching.
/// 
/// # Arguments
/// * `pattern` - Glob pattern like "src/*.ts" or "**\/component*" 
/// * `path` - Optional directory to search in (defaults to current directory)
///
/// # Returns
/// GlobResult with matching file paths or error message
#[napi]
pub async fn glob_search(pattern: String, path: Option<String>) -> napi::Result<GlobResult> {
    let tool = GlobTool::new();
    let args = GlobArgs { pattern, path };

    match tool.call(args).await {
        Ok(output) => Ok(GlobResult {
            success: true,
            data: Some(output),
            error: None,
        }),
        Err(e) => Ok(GlobResult {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}