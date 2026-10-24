//! inject_summary Tool — Pins a hierarchical DAG summary as system-level content
//!
//! Feature: spec/features/inject-summary-tool.feature
//!
//! This tool is the mechanism through which the agent completes its self-directed
//! compression cycle. After building a hierarchical DAG summary via SessionSearch,
//! the agent calls inject_summary to pin the summary and discard builder turns.
//!
//! The tool uses the handler pattern (like SessionSearchTool and FspecHandler):
//! - Tool definition and JSON schema live here in codelet-tools
//! - A handler type alias is defined for the actual session manipulation
//! - A global per-session handler registry stores handlers
//! - The actual session manipulation code lives in the NAPI handler
//! - When call() is invoked, the tool dispatches to the registered handler

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::ToolError;

/// Result returned by inject_summary handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectSummaryResult {
    /// Token count of the injected DAG
    pub injected_tokens: u64,
    /// Available context budget after injection
    pub remaining_budget: u64,
}

/// Handler function type for inject_summary execution.
/// Takes session_id and DAG content, returns the injection result.
pub type InjectSummaryHandler =
    Arc<dyn Fn(Uuid, String) -> Result<InjectSummaryResult, String> + Send + Sync>;

/// Per-session handler storage
static INJECT_SUMMARY_HANDLERS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, InjectSummaryHandler>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Set the inject_summary handler for a specific session
///
/// Called by session manager before agent run to configure how inject_summary
/// operations are executed for this session.
pub fn set_inject_summary_handler(session_id: Uuid, handler: Option<InjectSummaryHandler>) {
    if let Ok(mut guard) = INJECT_SUMMARY_HANDLERS.write() {
        match handler {
            Some(h) => {
                guard.insert(session_id, h);
            }
            None => {
                guard.remove(&session_id);
            }
        }
    }
}

/// Check if an inject_summary handler is configured for a specific session
pub fn has_inject_summary_handler(session_id: Uuid) -> bool {
    INJECT_SUMMARY_HANDLERS
        .read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false)
}

/// Execute inject_summary via the handler for a specific session
///
/// Called by InjectSummaryTool when the LLM invokes the tool.
pub fn execute_inject_summary(
    session_id: Uuid,
    content: String,
) -> Result<InjectSummaryResult, String> {
    let handler = match INJECT_SUMMARY_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => {
            return Err("Failed to acquire inject_summary handlers lock".to_string());
        }
    };

    match handler {
        Some(h) => h(session_id, content),
        None => Err(format!(
            "inject_summary handler not configured for session {session_id} — \
             InjectSummaryTool requires session context"
        )),
    }
}

/// Clear all inject_summary handlers (for testing)
pub fn clear_all_inject_summary_handlers() {
    if let Ok(mut guard) = INJECT_SUMMARY_HANDLERS.write() {
        guard.clear();
    }
}

/// Arguments for the inject_summary tool
#[derive(Debug, Deserialize, Serialize)]
pub struct InjectSummaryArgs {
    /// The DAG summary content to inject
    pub content: String,
}

/// InjectSummaryTool — Rig Tool implementation
///
/// Allows AI agents to pin a hierarchical DAG summary as persistent
/// system-level content, dropping all builder turns from active context.
#[derive(Clone, Debug)]
pub struct InjectSummaryTool {
    session_id: Uuid,
}

impl InjectSummaryTool {
    /// Create a new InjectSummaryTool instance
    ///
    /// # Arguments
    /// * `session_id` - The session ID for per-session handler lookup
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

impl Tool for InjectSummaryTool {
    const NAME: &'static str = "inject_summary";

    type Error = ToolError;
    type Args = InjectSummaryArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "inject_summary".to_string(),
            description: concat!(
                "Pin a hierarchical DAG summary as system-level content, ",
                "dropping all builder turns from the active context. ",
                "Call this after building your session summary via SessionSearch. ",
                "The content is wrapped as a system-reminder (type: compaction-dag) ",
                "and persisted across future compactions."
            )
            .to_string(),
            parameters: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The DAG summary content to inject"
                    }
                },
                "required": ["content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // HOOK-013: Run pre_tool_use hooks before execution
        if let Err(reason) = crate::pre_tool_hook::pre_tool_hook_check(
            self.session_id,
            &self.name(),
            &serde_json::to_value(&args).unwrap_or_default(),
        ) {
            return Err(ToolError::Blocked {
                tool: "inject_summary",
                message: reason,
            });
        }

        let result = execute_inject_summary(self.session_id, args.content).map_err(|e| {
            ToolError::Execution {
                tool: "inject_summary",
                message: e,
            }
        })?;

        serde_json::to_string_pretty(&json!({
            "status": "queued",
            "injected_tokens": result.injected_tokens,
            "token_basis": "summary_estimate",
        }))
        .map_err(|e| ToolError::Execution {
            tool: "inject_summary",
            message: format!("Failed to serialize result: {e}"),
        })
    }
}

#[cfg(test)]
mod tests;
