//! Internal-tool dispatch for the Rhai custom-provider tool surface
//! (PROV-092).
//!
//! Given a [`DispatchedToolParams`] (produced by the
//! [`super::tool_dispatch::default_to_internal`] helper), this module
//! executes the underlying internal tool and returns a JSON-serialised
//! result string suitable for handing back to a rig agent.
//!
//! The executor reuses the inner tool implementations directly
//! (`ReadTool`, `WriteTool`, `EditTool`, `BashTool`, `GrepTool`,
//! `GlobTool`, `LsTool`, `WebSearchTool`) instead of going through the
//! per-provider facade wrappers in `codelet_tools::facade::wrapper`.
//! That keeps the surface narrow — the dispatch pre-validates params
//! into the canonical `Internal*Params` shape, so we only need a
//! straight-line mapping per variant.

use codelet_tools::facade::{
    InternalBashParams, InternalFileParams, InternalLsParams, InternalSearchParams,
    InternalWebSearchParams,
};
use codelet_tools::{
    astgrep::AstGrepArgs, bash::BashArgs, edit::EditArgs, glob::GlobArgs, grep::GrepArgs,
    ls::LsArgs, read::ReadArgs, web_search::WebSearchRequest, write::WriteArgs, AstGrepTool,
    BashTool, EditTool, GlobTool, GrepTool, LsTool, ReadTool, WebSearchTool, WriteTool,
};
use codelet_common::web_search::WebSearchAction;
use rig::tool::Tool;
use serde_json::json;
use uuid::Uuid;

use super::error::CustomProviderError;
use super::tool_dispatch::{AstGrepParams, DispatchedToolParams};

/// Execute a [`DispatchedToolParams`] variant against the internal
/// tool implementations bound to `session_id`. Returns a JSON string
/// suitable as a rig tool output.
pub async fn execute_dispatched(
    session_id: Uuid,
    params: DispatchedToolParams,
) -> Result<String, CustomProviderError> {
    use DispatchedToolParams as D;
    match params {
        D::File(file) => execute_file(session_id, file).await,
        D::Bash(bash) => execute_bash(session_id, bash).await,
        D::Search(search) => execute_search(session_id, search).await,
        D::AstGrep(ast) => execute_ast_grep(session_id, ast).await,
        D::Ls(ls) => execute_ls(session_id, ls).await,
        D::WebSearch(ws) => execute_web_search(session_id, ws).await,
        D::Fspec(_) | D::Bridge(_) | D::Exec(_) | D::Hitl(_) => {
            // These maps_to categories require facade wrappers with
            // session-scoped state (HITL handlers, fspec handler
            // registrations, exec PTY tables) that are not yet wired
            // through this lightweight executor. Surface a clear error
            // so the script gets actionable feedback.
            Err(CustomProviderError::RhaiRuntimeError(
                "this maps_to category is not yet supported by the Rhai-backed agent — \
                 use a built-in provider or wait for the facade-wrapper bridge"
                    .to_string(),
            ))
        }
    }
}

async fn execute_file(
    session_id: Uuid,
    params: InternalFileParams,
) -> Result<String, CustomProviderError> {
    match params {
        InternalFileParams::Read {
            file_path,
            offset,
            limit,
            mode: _,
            indentation: _,
        } => {
            let tool = ReadTool::new(session_id);
            let args = ReadArgs {
                file_path,
                offset,
                limit,
                pdf_mode: None,
            };
            tool.call(args)
                .await
                .map_err(file_err)
        }
        InternalFileParams::Write { file_path, content } => {
            let tool = WriteTool::new(session_id);
            let args = WriteArgs { file_path, content };
            tool.call(args)
                .await
                .map_err(file_err)
        }
        InternalFileParams::Edit {
            file_path,
            old_string,
            new_string,
        } => {
            let tool = EditTool::new(session_id);
            let args = EditArgs {
                file_path,
                old_string,
                new_string,
            };
            tool.call(args)
                .await
                .map_err(file_err)
        }
    }
}

async fn execute_bash(
    session_id: Uuid,
    params: InternalBashParams,
) -> Result<String, CustomProviderError> {
    match params {
        InternalBashParams::Execute { command, cwd, .. } => {
            let tool = BashTool::new(session_id);
            let args = BashArgs { command, cwd };
            tool.call(args).await.map_err(bash_err)
        }
    }
}

async fn execute_search(
    session_id: Uuid,
    params: InternalSearchParams,
) -> Result<String, CustomProviderError> {
    match params {
        InternalSearchParams::Grep {
            pattern,
            path,
            include,
            limit,
        } => {
            let tool = GrepTool::new(session_id);
            let args = GrepArgs {
                pattern,
                path,
                output_mode: None,
                glob: include,
                limit,
            };
            tool.call(args).await.map_err(search_err)
        }
        InternalSearchParams::Glob { pattern, path } => {
            let tool = GlobTool::new(session_id);
            let args = GlobArgs {
                pattern,
                path,
                case_insensitive: None,
            };
            tool.call(args).await.map_err(search_err)
        }
    }
}

async fn execute_ls(
    session_id: Uuid,
    params: InternalLsParams,
) -> Result<String, CustomProviderError> {
    match params {
        InternalLsParams::List { path, .. } => {
            let tool = LsTool::new(session_id);
            let args = LsArgs { path };
            tool.call(args).await.map_err(ls_err)
        }
    }
}

async fn execute_ast_grep(
    session_id: Uuid,
    params: AstGrepParams,
) -> Result<String, CustomProviderError> {
    let tool = AstGrepTool::new(session_id);
    let args = AstGrepArgs {
        pattern: params.pattern,
        language: params.language,
        path: params.path,
    };
    tool.call(args).await.map_err(ast_grep_err)
}

async fn execute_web_search(
    session_id: Uuid,
    params: InternalWebSearchParams,
) -> Result<String, CustomProviderError> {
    let tool = WebSearchTool::new(session_id);
    let request = match params {
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
        InternalWebSearchParams::FindInPage {
            url,
            pattern,
            headless,
            pause,
        } => WebSearchRequest {
            action: WebSearchAction::FindInPage {
                url: Some(url),
                pattern: Some(pattern),
                headless,
                pause,
            },
        },
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
    let result = tool.call(request).await.map_err(web_err)?;
    serde_json::to_string(&result).map_err(|e| {
        CustomProviderError::RhaiRuntimeError(format!("serialise web_search result: {e}"))
    })
}

fn file_err(e: codelet_tools::ToolError) -> CustomProviderError {
    serialise_tool_error(&e, "file_op")
}

fn bash_err(e: codelet_tools::ToolError) -> CustomProviderError {
    serialise_tool_error(&e, "bash")
}

fn search_err(e: codelet_tools::ToolError) -> CustomProviderError {
    serialise_tool_error(&e, "search")
}

fn ls_err(e: codelet_tools::ToolError) -> CustomProviderError {
    serialise_tool_error(&e, "ls")
}

fn ast_grep_err(e: codelet_tools::ToolError) -> CustomProviderError {
    serialise_tool_error(&e, "ast_grep")
}

fn web_err(e: codelet_tools::ToolError) -> CustomProviderError {
    serialise_tool_error(&e, "web_search")
}

fn serialise_tool_error(e: &codelet_tools::ToolError, category: &str) -> CustomProviderError {
    let payload = json!({
        "error": e.to_string(),
        "category": category,
    });
    CustomProviderError::RhaiRuntimeError(payload.to_string())
}
