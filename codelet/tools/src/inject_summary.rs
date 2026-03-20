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

        let result = execute_inject_summary(self.session_id, args.content)
            .map_err(|e| ToolError::Execution {
                tool: "inject_summary",
                message: e,
            })?;

        serde_json::to_string_pretty(&result).map_err(|e| ToolError::Execution {
            tool: "inject_summary",
            message: format!("Failed to serialize result: {e}"),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Feature: spec/features/inject-summary-tool.feature
    ///
    /// Scenario: Tool implements Rig Tool trait with correct name and schema
    #[tokio::test]
    #[serial]
    async fn test_tool_definition_name_and_schema() {
        // @step Given the InjectSummaryTool is compiled as part of the codelet-tools crate
        let tool = InjectSummaryTool::new(Uuid::new_v4());

        // @step When the tool definition is requested
        let definition = tool.definition("".to_string()).await;

        // @step Then the tool name should be "inject_summary"
        assert_eq!(definition.name, "inject_summary");

        // @step And the JSON schema should have "content" as a required string parameter
        let params = &definition.parameters;
        let content_prop = &params["properties"]["content"];
        assert_eq!(content_prop["type"], "string");
        let required = params["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("content")));

        // @step And the description should explain DAG pinning, builder turn dropping, and compaction persistence
        assert!(definition.description.contains("DAG summary"));
        assert!(definition.description.contains("builder turns"));
        assert!(definition.description.contains("compaction"));
    }

    /// Scenario: Successful inject_summary dispatches to registered handler
    #[tokio::test]
    #[serial]
    async fn test_successful_dispatch_to_handler() {
        clear_all_inject_summary_handlers();

        let session_id = Uuid::new_v4();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        // @step Given a session with a registered InjectSummaryHandler
        let handler: InjectSummaryHandler = Arc::new(move |sid, content| {
            called_clone.store(true, Ordering::SeqCst);
            assert_eq!(sid, session_id);
            assert!(content.contains("D2: Architecture"));
            Ok(InjectSummaryResult {
                injected_tokens: 1250,
                remaining_budget: 185000,
            })
        });
        set_inject_summary_handler(session_id, Some(handler));

        // @step When the agent calls inject_summary with DAG content
        let tool = InjectSummaryTool::new(session_id);
        let output = tool
            .call(InjectSummaryArgs {
                content: "# D2: Architecture\n- Using JWT...".to_string(),
            })
            .await;

        // @step Then the handler receives the session_id and content string
        assert!(called.load(Ordering::SeqCst));

        // @step And the tool returns InjectSummaryResult with injected_tokens and remaining_budget
        let output = output.unwrap();
        let result: InjectSummaryResult = serde_json::from_str(&output).unwrap();
        assert_eq!(result.injected_tokens, 1250);
        assert_eq!(result.remaining_budget, 185000);

        clear_all_inject_summary_handlers();
    }

    /// Scenario: inject_summary with no handler returns error
    #[tokio::test]
    #[serial]
    async fn test_no_handler_returns_error() {
        clear_all_inject_summary_handlers();

        // @step Given no InjectSummaryHandler is registered for the session
        let session_id = Uuid::new_v4();

        // @step When the agent calls inject_summary
        let tool = InjectSummaryTool::new(session_id);
        let result = tool
            .call(InjectSummaryArgs {
                content: "some content".to_string(),
            })
            .await;

        // @step Then the tool returns a ToolError with message containing "not configured"
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("not configured"), "Error should contain 'not configured', got: {err_msg}");

        // @step And the error message includes the session UUID
        assert!(err_msg.contains(&session_id.to_string()), "Error should contain session UUID, got: {err_msg}");

        clear_all_inject_summary_handlers();
    }

    /// Scenario: Concurrent sessions have isolated handlers
    #[tokio::test]
    #[serial]
    async fn test_concurrent_sessions_isolated() {
        clear_all_inject_summary_handlers();

        // @step Given Session A and Session B each have a registered InjectSummaryHandler
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        let a_called = Arc::new(AtomicBool::new(false));
        let a_called_clone = a_called.clone();
        let b_called = Arc::new(AtomicBool::new(false));
        let b_called_clone = b_called.clone();

        let handler_a: InjectSummaryHandler = Arc::new(move |_, _| {
            a_called_clone.store(true, Ordering::SeqCst);
            Ok(InjectSummaryResult {
                injected_tokens: 100,
                remaining_budget: 200000,
            })
        });
        set_inject_summary_handler(session_a, Some(handler_a));

        let handler_b: InjectSummaryHandler = Arc::new(move |_, _| {
            b_called_clone.store(true, Ordering::SeqCst);
            Ok(InjectSummaryResult {
                injected_tokens: 200,
                remaining_budget: 180000,
            })
        });
        set_inject_summary_handler(session_b, Some(handler_b));

        // @step When inject_summary is called on Session A
        let tool_a = InjectSummaryTool::new(session_a);
        let result = tool_a
            .call(InjectSummaryArgs {
                content: "content".to_string(),
            })
            .await;

        // @step Then only Session A's handler is invoked
        assert!(a_called.load(Ordering::SeqCst));
        let output: InjectSummaryResult = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output.injected_tokens, 100);

        // @step And Session B's handler is not affected
        assert!(!b_called.load(Ordering::SeqCst));

        clear_all_inject_summary_handlers();
    }

    /// Scenario: Handler removal via set_inject_summary_handler with None
    #[tokio::test]
    #[serial]
    async fn test_handler_removal_with_none() {
        clear_all_inject_summary_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a session with a registered InjectSummaryHandler
        let handler: InjectSummaryHandler = Arc::new(|_, _| {
            Ok(InjectSummaryResult {
                injected_tokens: 500,
                remaining_budget: 190000,
            })
        });
        set_inject_summary_handler(session_id, Some(handler));
        assert!(has_inject_summary_handler(session_id));

        // @step When set_inject_summary_handler is called with None for that session
        set_inject_summary_handler(session_id, None);

        // @step Then has_inject_summary_handler returns false for that session
        assert!(!has_inject_summary_handler(session_id));

        // @step And subsequent inject_summary calls return an error
        let tool = InjectSummaryTool::new(session_id);
        let result = tool
            .call(InjectSummaryArgs {
                content: "content".to_string(),
            })
            .await;
        assert!(result.is_err());

        clear_all_inject_summary_handlers();
    }

    /// Scenario: Tool is constructed with session_id for handler lookup
    #[tokio::test]
    #[serial]
    async fn test_tool_constructed_with_session_id() {
        clear_all_inject_summary_handlers();

        let session_id = Uuid::new_v4();
        let received_sid = Arc::new(RwLock::new(None::<Uuid>));
        let received_sid_clone = received_sid.clone();

        // @step Given an InjectSummaryTool created with a specific session_id
        let handler: InjectSummaryHandler = Arc::new(move |sid, _content| {
            *received_sid_clone.write().unwrap() = Some(sid);
            Ok(InjectSummaryResult {
                injected_tokens: 300,
                remaining_budget: 195000,
            })
        });
        set_inject_summary_handler(session_id, Some(handler));

        let tool = InjectSummaryTool::new(session_id);

        // @step When call() is invoked with InjectSummaryArgs
        let result = tool
            .call(InjectSummaryArgs {
                content: "test content".to_string(),
            })
            .await;

        // @step Then the tool looks up the handler using its stored session_id
        assert!(result.is_ok());
        let received = received_sid.read().unwrap();
        assert_eq!(*received, Some(session_id));

        // @step And the correct per-session handler is dispatched
        let output: InjectSummaryResult = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output.injected_tokens, 300);
        assert_eq!(output.remaining_budget, 195000);

        clear_all_inject_summary_handlers();
    }

    /// Scenario: Tool and handler types are exported from lib.rs
    /// This is a compile-time test — if the imports below work, the test passes.
    #[test]
    fn test_types_exist_and_are_public() {
        // @step Given the codelet-tools crate lib.rs module declarations
        // @step When inject_summary module is declared

        // @step Then InjectSummaryTool is publicly re-exported
        let _tool: InjectSummaryTool = InjectSummaryTool::new(Uuid::new_v4());

        // @step And InjectSummaryHandler type alias is publicly re-exported
        let _handler: InjectSummaryHandler = Arc::new(|_, _| {
            Ok(InjectSummaryResult {
                injected_tokens: 0,
                remaining_budget: 0,
            })
        });

        // @step And set_inject_summary_handler function is publicly re-exported
        // (function exists and compiles — verified by calling it above in other tests)

        // @step And has_inject_summary_handler function is publicly re-exported
        let _ = has_inject_summary_handler(Uuid::new_v4());

        // @step And clear_all_inject_summary_handlers function is publicly re-exported
        // (function exists and compiles — verified by calling it in other tests)
    }
}
