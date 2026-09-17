//! GenerateCompaction Tool — on-demand compaction of a target session (CMPCT-045).
//!
//! Feature: spec/features/generate-compaction-tool.feature
//!
//! A rig Tool that builds a hierarchical compaction DAG for a target
//! session (default: the calling session) in a clean ephemeral sub-agent
//! and returns the DAG as the tool result.
//!
//! The pin (clear-to-reminders + push wrapped DAG + recalculate the token
//! tracker) is performed handler-side by production code, never by the
//! sub-agent.
//!
//! Uses the handler pattern (like `DeepSearchTool`):
//! - Tool definition and JSON schema live here in codelet-tools
//! - A handler type alias is defined for the actual compaction execution
//! - A global per-session handler registry stores handlers
//! - The actual sub-agent construction and execution lives in
//!   `codelet-agent-loop` (`generate_compaction_handler`)
//! - When call() is invoked, the tool dispatches to the registered handler
//!
//! DeepSearch clone contract: the handler is looked up per CALLING session
//! (the tool's construction `session_id`), mirroring `execute_deep_search`.


use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;

use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::ToolError;

/// Arguments for the GenerateCompaction tool.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerateCompactionArgs {
    /// The target session to compact (UUID string).
    ///
    /// Optional — when omitted or `null`, the target is the CALLING session
    /// (the session whose tool registry entry invoked the tool). On parse
    /// failure the tool returns `ToolError::Validation` and nothing is
    /// dispatched or modified.
    #[serde(default)]
    pub session_id: Option<String>,
}

/// Handler function type for generate-compaction execution.
///
/// Takes the RESOLVED target session id (the tool has already validated the
/// argument and defaulted it to the calling session) and returns a future
/// resolving to the DAG text that was pinned to the target.
///
/// The `codelet-agent-loop` closure performs the full lifecycle:
/// 1. Capture the target's existing DAG (`detect_existing_dag`) before any clear
/// 2. Ephemeral sub-agent (DeepSearch shape, 7 read-only tools, BUG-102
///    provider inheritance, AMGR-016 wall-clock timeout)
/// 3. Pin the DAG handler-side (target == caller ⇒ `pending_dag_content` stash;
///    target != caller ⇒ immediate pin under the target's inner lock)
/// 4. On timeout / failure / unparseable output: free force-inject fallback DAG
///
/// NOTE: returns a Future (not a sync Result) — the sub-agent makes async LLM
/// API calls (same contract as `DeepSearchHandler`).
pub type GenerateCompactionHandler = Arc<
    dyn Fn(Uuid) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> + Send + Sync,
>;

/// Per-session handler storage.
static GENERATE_COMPACTION_HANDLERS: once_cell::sync::Lazy<
    RwLock<HashMap<Uuid, GenerateCompactionHandler>>,
> = once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Set the generate-compaction handler for a specific session.
///
/// Called by the agent loop at session creation (and after `/model` /
/// `/provider` changes). `None` removes the handler (end-of-turn cleanup).
pub fn set_generate_compaction_handler(
    session_id: Uuid,
    handler: Option<GenerateCompactionHandler>,
) {
    if let Ok(mut guard) = GENERATE_COMPACTION_HANDLERS.write() {
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

/// Check if a generate-compaction handler is configured for a specific session.
pub fn has_generate_compaction_handler(session_id: Uuid) -> bool {
    GENERATE_COMPACTION_HANDLERS
        .read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false)
}

/// Clear all generate-compaction handlers (for testing).
pub fn clear_all_generate_compaction_handlers() {
    if let Ok(mut guard) = GENERATE_COMPACTION_HANDLERS.write() {
        guard.clear();
    }
}

/// Dispatch to the registered handler for the CALLING session.
///
/// Returns `Err` with the canonical missing-handler message when no handler
/// is registered (the tool maps this to `ToolError::Execution`).
async fn execute_generate_compaction(
    caller_session_id: Uuid,
    target_session_id: Uuid,
) -> Result<String, String> {
    let handler = match GENERATE_COMPACTION_HANDLERS.read() {
        Ok(guard) => guard.get(&caller_session_id).cloned(),
        Err(_) => {
            return Err("Failed to acquire generate-compaction handlers lock".to_string());
        }
    };

    match handler {
        Some(h) => h(target_session_id).await,
        None => Err(format!(
            "handler not configured for session {caller_session_id} — \
             GenerateCompactionTool requires session context"
        )),
    }
}

/// GenerateCompaction Tool — rig Tool implementation.
///
/// Allows any session to compact a target session (default: itself) on
/// demand by building a compaction DAG in a clean ephemeral sub-agent.
///
/// Uses the handler pattern — the actual sub-agent construction and
/// execution is delegated to a registered handler
/// (`set_generate_compaction_handler`).
#[derive(Clone, Debug)]
pub struct GenerateCompactionTool {
    /// Calling session ID — used for handler lookup (mirrors DeepSearchTool).
    pub session_id: Uuid,
}

impl GenerateCompactionTool {
    /// Create a new GenerateCompactionTool instance.
    ///
    /// # Arguments
    /// * `session_id` - The calling session ID (for handler lookup)
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

impl Tool for GenerateCompactionTool {
    const NAME: &'static str = "GenerateCompaction";

    type Error = ToolError;
    type Args = GenerateCompactionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: concat!(
                "Build a hierarchical DAG compaction summary of a target session in a clean ",
                "ephemeral sub-agent and pin the DAG to that session. Use to compact session ",
                "context on demand — including a DIFFERENT session's context (pass its session_id). ",
                "When session_id is omitted, the calling session is compacted. Returns the pinned ",
                "DAG text as the result."
            )
            .to_string(),
            parameters: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": ["string", "null"],
                        "description": "The target session to compact (UUID string). Optional — when omitted or null, the calling session is compacted."
                    }
                },
                "required": []
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
                tool: Self::NAME,
                message: reason,
            });
        }

        // Validate session_id — an argument-validation failure, not an
        // execution failure (mirrors DeepSearch's empty-query validation).
        // On parse failure: fail fast with a usage hint; do NOT fall back to
        // the calling session (CMPCT-045 Rule [1]).
        let target = match args.session_id {
            Some(raw) => {
                match raw.trim().parse::<Uuid>() {
                    Ok(uuid) => uuid,
                    Err(_) => {
                        let schema = self.definition(String::new()).await.parameters;
                        return Err(ToolError::Validation {
                            tool: Self::NAME,
                            message: codelet_common::tool_usage::append_usage_to_message(
                                Self::NAME,
                                &schema,
                                "session_id must be a UUID string (or omitted to compact the \
                                 calling session)",
                            ),
                        });
                    }
                }
            }
            None => self.session_id, // CMPCT-045 Rule [2]: omitted ⇒ calling session
        };

        // Dispatch to the registered handler (async — sub-agent makes LLM
        // API calls). The handler performs the pin handler-side and returns
        // the pinned DAG text (success or fallback).
        execute_generate_compaction(self.session_id, target)
            .await
            .map_err(|e| ToolError::Execution {
                tool: Self::NAME,
                message: e,
            })
    }
}
