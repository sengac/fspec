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

/// BUG-614: one limit of validation attempts per inject_summary turn.
pub const MAX_SUMMARY_ATTEMPTS: usize = 3;

/// The exact DAG node syntax every summary candidate must follow.
pub const DAG_NODE_SYNTAX: &str = "\
Return a complete candidate, not prose, a patch, or an unfinished fragment.
Each <dag-node> requires exactly depth, turns and label attributes with double-quoted values,
a supported depth (D0, D1 or D2), numeric persisted turn coordinates and a nonempty label and body.
Close every <dag-node> with </dag-node>. Do not nest DAG structures inside node bodies.
Example: <dag-node depth=\"D1\" turns=\"0-1\" label=\"Work\">Evidence-based summary</dag-node>.
Use actual authoritative coordinates and evidence, never copy example bounds blindly.";

/// The inclusive turn-range rules for DAG peers.
pub const DAG_RANGE_RULES: &str = "\
Persisted turn ranges are inclusive. Same-depth peers must be disjoint:
0-10 and 10-20 overlap; 0-10 and 11-20 are disjoint.
Cross-depth overlap is valid, including parent/child nesting.
Recheck same-depth conflicts after promotion, including preserved D2 nodes.
Regenerate a complete evidence-backed candidate when peers conflict; do not silently drop nodes,
clamp bounds, invent coverage or rewrite preserved durable payloads. Their union must cover the captured prefix.";

/// The strict `<dag-files>` metadata syntax.
pub const FILE_METADATA_INSTRUCTION: &str = "\
File metadata syntax is strict. Each nonempty line inside <dag-files> must be
exactly a unique '- path (Operation)' entry, with Operation exactly Created, Modified, or Deleted:
<dag-files>
- path/to/file.rs (Created)
- path/to/other.rs (Modified)
</dag-files>
No prose, backticks, or explanations belong inside <dag-files>. Put explanations in dag-node bodies.
Do not write '(Created; explanation)' or a bullet saying no files changed.
If no files changed and no existing file identities must be preserved, use <dag-files></dag-files>.
Keep existing file identities unless an authoritative later operation updates them.";

const NODE_OPEN: &str = "<dag-node";
const NODE_END: &str = "</dag-node>";

/// Number of complete `<dag-node>` blocks in `text`.
pub fn complete_dag_nodes(text: &str) -> usize {
    text.matches(NODE_END).count()
}

/// Whether `text` ends inside a `<dag-node` that was never closed.
pub fn ends_inside_dag_node(text: &str) -> bool {
    text.rfind(NODE_OPEN)
        .is_some_and(|open| text.rfind(NODE_END).is_none_or(|end| end < open))
}

/// BUG-614 (Q5): feedback for a DAG cut off before its last node closed.
pub fn truncated_dag_feedback(complete_nodes: usize) -> String {
    format!(
        "Your DAG was truncated after {complete_nodes} complete nodes, so nothing was installed. \
         Resubmit a whole new, shorter DAG in one inject_summary call: fewer, shorter nodes. \
         There is no continuation; do not send only the missing part."
    )
}

/// BUG-614: a candidate rejected by DAG validation. Nothing was installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InjectSummaryRejection {
    /// 1-based attempt number within this turn.
    pub attempt: usize,
    pub max_attempts: usize,
    /// The exact validator error.
    pub error: String,
    /// Last persisted turn the DAG must cover, when known.
    pub coverage_end: Option<usize>,
}

/// Why an inject_summary submission was not queued.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InjectSummaryError {
    /// The candidate failed validation; the model may correct it.
    #[error("{}", .0.error)]
    Rejected(InjectSummaryRejection),
    /// The submission could not be considered at all.
    #[error("{0}")]
    Failed(String),
}

impl From<String> for InjectSummaryError {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

impl From<&str> for InjectSummaryError {
    fn from(message: &str) -> Self {
        Self::Failed(message.to_owned())
    }
}

/// Handler function type for inject_summary execution.
/// Takes session_id and DAG content, returns the injection result.
pub type InjectSummaryHandler =
    Arc<dyn Fn(Uuid, String) -> Result<InjectSummaryResult, InjectSummaryError> + Send + Sync>;

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
) -> Result<InjectSummaryResult, InjectSummaryError> {
    let handler = match INJECT_SUMMARY_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => {
            return Err("Failed to acquire inject_summary handlers lock".into());
        }
    };

    match handler {
        Some(h) => h(session_id, content),
        None => Err(InjectSummaryError::Failed(format!(
            "inject_summary handler not configured for session {session_id} — \
             InjectSummaryTool requires session context"
        ))),
    }
}

/// The non-error tool result the model receives for a rejected candidate.
fn rejection_result(rejection: &InjectSummaryRejection, content: &str) -> serde_json::Value {
    let last = rejection.attempt >= rejection.max_attempts;
    let truncation = if ends_inside_dag_node(content) {
        format!("{} ", truncated_dag_feedback(complete_dag_nodes(content)))
    } else {
        String::new()
    };
    let instruction = if last {
        format!(
            "{truncation}Nothing was installed and no further summary is accepted in this turn. \
             Continue the task on the current context."
        )
    } else {
        let coverage = rejection
            .coverage_end
            .map(|end| format!(" whose same-depth nodes together cover persisted turns 0-{end}"))
            .unwrap_or_default();
        format!(
            "{truncation}Nothing was installed. Call inject_summary again with a complete corrected \
             DAG (not a patch){coverage}."
        )
    };
    json!({
        "status": if last { "rejected_final" } else { "rejected" },
        "attempt": rejection.attempt,
        "max_attempts": rejection.max_attempts,
        "error": rejection.error,
        "installed": false,
        "syntax": format!("{DAG_NODE_SYNTAX}\n{DAG_RANGE_RULES}\n{FILE_METADATA_INSTRUCTION}"),
        "instruction": instruction,
    })
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
            description: format!(
                "Pin a hierarchical DAG summary as system-level content, \
                 dropping all builder turns from the active context. \
                 Call this after building your session summary via SessionSearch. \
                 The content is wrapped as a system-reminder (type: compaction-dag) \
                 and persisted across future compactions. The content must be a DAG, \
                 not prose: a rejected candidate comes back with the exact error, and you \
                 may correct it {MAX_SUMMARY_ATTEMPTS} times in total.\n\
                 {DAG_NODE_SYNTAX}\n{DAG_RANGE_RULES}\n{FILE_METADATA_INSTRUCTION}"
            ),
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

        let value = match execute_inject_summary(self.session_id, args.content.clone()) {
            Ok(result) => json!({
                "status": "queued",
                "injected_tokens": result.injected_tokens,
                "token_basis": "summary_estimate",
            }),
            // BUG-614: a rejection is not a tool failure; the model reads the
            // exact error and corrects its DAG in the same turn.
            Err(InjectSummaryError::Rejected(rejection)) => {
                rejection_result(&rejection, &args.content)
            }
            Err(InjectSummaryError::Failed(message)) => {
                return Err(ToolError::Execution {
                    tool: "inject_summary",
                    message,
                })
            }
        };

        serde_json::to_string_pretty(&value).map_err(|e| ToolError::Execution {
            tool: "inject_summary",
            message: format!("Failed to serialize result: {e}"),
        })
    }
}

#[cfg(test)]
mod tests;
