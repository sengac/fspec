//! DeepSearch Tool — Ephemeral Sub-Agent for Scoped Corpus Analysis
//!
//! Feature: spec/features/deep-search.feature
//!
//! A rig Tool that spawns an ephemeral sub-agent to explore user-scoped corpora
//! (code files, session histories) using read-only tools (Read, Grep, AstGrep,
//! Glob, Ls, Bash, SessionSearch) and returns a text answer.
//!
//! Uses the handler pattern (like SessionSearchTool and InjectSummaryTool):
//! - Tool definition and JSON schema live here in codelet-tools
//! - A handler type alias is defined for the actual deep search execution
//! - A global per-session handler registry stores handlers
//! - The actual agent construction and execution lives in the NAPI handler
//! - When call() is invoked, the tool dispatches to the registered handler
//!
//! Based on the RLM paper (MIT CSAIL, arXiv:2512.24601).

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::needless_collect)]
mod tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::needless_collect)]
mod recursive_tests;

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

/// Default maximum depth for the sub-agent's tool call rounds.
/// Bounded at 50 to prevent runaway sub-agents (not usize::MAX).
pub const DEFAULT_DEEP_SEARCH_MAX_DEPTH: usize = 50;

/// Default maximum recursion depth for DeepSearch self-invocation.
/// Controls how many nested DeepSearch levels are allowed (depth 0, 1, 2).
/// A value of 2 means: parent (depth 0) → child (depth 1) → grandchild (depth 2, base case).
pub const DEFAULT_MAX_RECURSION_DEPTH: usize = 2;

/// Canonical list of tool names the sub-agent gets.
///
/// This constant is the single source of truth. The handler in
/// `codelet/napi/src/deep_search_handler.rs::build_and_run_agent()` adds exactly
/// these tools (and asserts `SUB_AGENT_TOOL_COUNT` at compile time).
/// If tools are added or removed, update BOTH this array AND the handler.
pub const SUB_AGENT_TOOL_NAMES: [&str; 7] = [
    "Read",
    "Grep",
    "AstGrep",
    "Glob",
    "Ls",
    "Bash",
    "SessionSearch",
];

/// Number of tools the sub-agent gets. Used for compile-time assertions in the
/// handler to catch tool list drift.
pub const SUB_AGENT_TOOL_COUNT: usize = SUB_AGENT_TOOL_NAMES.len();

/// Arguments for the DeepSearch tool
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeepSearchArgs {
    /// The question to answer (required)
    pub query: String,

    /// Colon-separated paths or glob patterns (PATH-style).
    /// Optional — when omitted, the sub-agent uses only SessionSearch.
    /// Examples: "src/", "src/auth/:tests/auth/", "**/*.rs:spec/**/*.feature"
    ///
    /// Accepts both a plain string (preferred, PATH-style) and, for backwards
    /// compatibility with callers emitting array JSON, a `Vec<String>` that is
    /// joined with `:`.
    #[serde(default, deserialize_with = "deserialize_scope")]
    pub scope: Option<String>,

    /// Maximum tool call depth before stopping (default: 50).
    /// Prevents runaway sub-agents.
    #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
    pub max_depth: Option<usize>,

    /// Maximum recursion depth for nested DeepSearch calls (default: 2).
    /// Controls how many levels of DeepSearch-within-DeepSearch are allowed.
    /// Separate from max_depth which controls tool-call rounds per agent.
    #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
    pub max_recursion_depth: Option<usize>,
}

impl DeepSearchArgs {
    /// Parse `scope` into an ordered, deduplicated list of paths.
    ///
    /// The wire format is PATH-style: segments separated by `:`.
    /// Whitespace around segments is trimmed; empty segments are dropped.
    pub fn scope_paths(&self) -> Vec<String> {
        split_scope(self.scope.as_deref())
    }
}

/// Split a colon-separated scope string into a list of non-empty trimmed paths.
///
/// Shared helper so both `DeepSearchArgs::scope_paths` and the handler layer
/// treat `scope` identically.
pub fn split_scope(scope: Option<&str>) -> Vec<String> {
    let raw = match scope {
        Some(s) => s,
        None => return Vec::new(),
    };
    let mut out: Vec<String> = Vec::new();
    for seg in raw.split(':') {
        let trimmed = seg.trim();
        if trimmed.is_empty() {
            continue;
        }
        let owned = trimmed.to_string();
        if !out.contains(&owned) {
            out.push(owned);
        }
    }
    out
}

/// Deserialize the `scope` field.
///
/// Accepts three shapes for robustness across capable and weak LLMs:
///   - `null` / missing           → `None`
///   - `"a:b:c"` (string)         → `Some("a:b:c".to_string())`
///   - `["a", "b", "c"]` (array)  → `Some("a:b:c".to_string())`  (joined)
///
/// The array branch is a compatibility shim for frontier models that have
/// been trained to emit JSON arrays for list-valued params. Internally we
/// standardise on the colon-separated string form.
fn deserialize_scope<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value = serde_json::Value::deserialize(d)?;
    match value {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(s) => {
            if s.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        }
        serde_json::Value::Array(items) => {
            let parts: Vec<String> = items
                .into_iter()
                .filter_map(|v| match v {
                    serde_json::Value::String(s) => {
                        let t = s.trim().to_string();
                        if t.is_empty() { None } else { Some(t) }
                    }
                    _ => None,
                })
                .collect();
            if parts.is_empty() {
                Ok(None)
            } else {
                Ok(Some(parts.join(":")))
            }
        }
        other => Err(D::Error::custom(format!(
            "scope must be a colon-separated string (or array of strings); got {other:?}"
        ))),
    }
}

/// Handler function type for deep search execution.
/// Takes query, optional colon-separated scope string, max_depth, and
/// max_recursion_depth. Returns a future resolving to the synthesized answer.
///
/// The `scope` argument is the raw PATH-style string (e.g. `"src/:tests/"`) —
/// callers use `split_scope()` to resolve it into a `Vec<String>` of paths.
///
/// The handler is registered by session_manager before the agent run. It captures:
/// - project_path: for creating the ephemeral SessionSearch handler
/// - Provider access: for building the sub-agent with the right LLM
///
/// The handler is responsible for the full lifecycle:
/// 1. Create ephemeral session_id (Uuid::new_v4())
/// 2. Register SessionSearch handler for ephemeral session
/// 3. Build system prompt with scope description
/// 4. Build rig agent with read-only tools
/// 5. Call RigAgent::prompt(query) (non-streaming, blocking)
/// 6. Cleanup SessionSearch handler
/// 7. Return final answer
///
/// NOTE: This handler returns a Future (not a sync Result) because the sub-agent
/// executes async LLM API calls via RigAgent::prompt(). Unlike SessionSearchHandler
/// and InjectSummaryHandler (which do sync persistence work), DeepSearch must be
/// async to avoid creating a nested tokio runtime inside the parent agent's runtime.
pub type DeepSearchHandler = Arc<
    dyn Fn(
            String,
            Option<String>,
            usize,
            usize,
        ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync,
>;

/// Per-session handler storage
static DEEP_SEARCH_HANDLERS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, DeepSearchHandler>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Set the deep search handler for a specific session
///
/// Called by session manager before agent run to configure how deep search
/// operations are executed for this session.
pub fn set_deep_search_handler(session_id: Uuid, handler: Option<DeepSearchHandler>) {
    if let Ok(mut guard) = DEEP_SEARCH_HANDLERS.write() {
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

/// Check if a deep search handler is configured for a specific session
pub fn has_deep_search_handler(session_id: Uuid) -> bool {
    DEEP_SEARCH_HANDLERS
        .read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false)
}

/// Execute a deep search via the handler for a specific session
///
/// Called by DeepSearchTool when the LLM invokes the tool.
/// Async because the handler returns a Future (LLM API calls are async).
async fn execute_deep_search(
    session_id: Uuid,
    query: String,
    scope: Option<String>,
    max_depth: usize,
    max_recursion_depth: usize,
) -> Result<String, String> {
    let handler = match DEEP_SEARCH_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => {
            return Err("Failed to acquire deep search handlers lock".to_string());
        }
    };

    match handler {
        Some(h) => h(query, scope, max_depth, max_recursion_depth).await,
        None => Err(format!(
            "Deep search handler not configured for session {session_id} — \
             DeepSearchTool requires session context"
        )),
    }
}

/// Clear all deep search handlers (for testing)
pub fn clear_all_deep_search_handlers() {
    if let Ok(mut guard) = DEEP_SEARCH_HANDLERS.write() {
        guard.clear();
    }
}

/// Build the system prompt for the ephemeral sub-agent.
///
/// Describes available tools, code scope (if any), and search strategy.
/// When `can_recurse` is true, includes DeepSearch in the tool list and
/// teaches the RLM decompose-delegate-aggregate strategy.
pub fn build_system_prompt(scope: &[String], can_recurse: bool) -> String {
    let scope_section = if scope.is_empty() {
        "No code scope specified. Use SessionSearch to explore session history only.".to_string()
    } else {
        let paths = scope
            .iter()
            .map(|p| format!("  - {p}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "YOUR CODE SCOPE:\n{paths}\n\n\
             Files are accessible via Read, Grep, AstGrep, Glob, Ls, and Bash tools.\n\
             Do NOT try to read all files — explore strategically."
        )
    };

    let deep_search_tool_section = if can_recurse {
        "\n- DeepSearch: Spawn a recursive sub-agent for sub-problems. \
         DeepSearch with no scope is a lightweight one-shot LLM call for reasoning. \
         DeepSearch with scope spawns a full sub-agent with its own tools for exploration."
    } else {
        ""
    };

    let recursion_strategy = if can_recurse {
        "\n\n\
         RECURSIVE DECOMPOSITION STRATEGY (decompose → delegate → aggregate):\n\
         When a question is too complex or the scope too large for a single pass:\n\
         1. DECOMPOSE: Break the question into independent sub-questions. Use Bash \
         with python3 -c to programmatically split file lists or data into chunks \
         (e.g. `find src/ -name '*.rs' | python3 -c 'import sys; files=sys.stdin.read().split(); \
         n=len(files)//3; [print(chr(10).join(files[i:i+n])) for i in range(0,len(files),n)]'`).\n\
         2. DELEGATE: Use DeepSearch for each sub-question or chunk with narrowed scope. \
         Each DeepSearch call spawns a sub-agent that can itself recurse further.\n\
         3. AGGREGATE: Combine sub-answers into a coherent final answer.\n\n\
         Use DeepSearch WITHOUT scope for lightweight reasoning sub-tasks (acts as a plain LLM call).\n\
         Use DeepSearch WITH scope to explore specific directories or files.\n\
         Use Bash to orchestrate: enumerate files, split into chunks, then call DeepSearch per chunk.\n\
         Prefer narrow scopes over broad ones to keep sub-agents focused."
    } else {
        ""
    };

    format!(
        "You are a research assistant tasked with answering a query by exploring a scoped \
         corpus of files and session history. You have access to tools for reading files, \
         searching with regex, structural code search, and session history exploration.\n\n\
         {scope_section}\n\n\
         AVAILABLE TOOLS:\n\
         - Read: Read file contents (use offset/limit for large files)\n\
         - Grep: Search file contents by regex pattern\n\
         - AstGrep: AST-based structural code search (for code files)\n\
         - Glob: Find files matching patterns\n\
         - Ls: List directory contents\n\
         - Bash: Execute shell commands for data processing\n\
         - SessionSearch: Search and view session conversation history \
           (use recent/search/show actions){deep_search_tool_section}\n\n\
         STRATEGY:\n\
         1. Start by understanding the scope — use Grep or Glob to find relevant files\n\
         2. Read targeted sections, not entire files\n\
         3. For code: use AstGrep to find structural patterns (functions, types, etc.)\n\
         4. Use SessionSearch to find relevant past conversations\n\
         5. Build up your answer incrementally\n\
         6. When you have enough information, provide your final answer\n\n\
         IMPORTANT:\n\
         - Do NOT try to read all files at once — explore strategically\n\
         - Use Grep/AstGrep to narrow down before reading\n\
         - Your answer should directly address the original query\n\
         - If the answer is not in scope, say so explicitly{recursion_strategy}"
    )
}

/// Returns the list of tool names the sub-agent gets.
///
/// Delegates to `SUB_AGENT_TOOL_NAMES` — the single source of truth.
pub fn sub_agent_tool_names() -> Vec<&'static str> {
    SUB_AGENT_TOOL_NAMES.to_vec()
}

/// DeepSearch Tool — Rig Tool implementation
///
/// Allows AI agents to spawn an ephemeral sub-agent for deep corpus exploration.
/// The sub-agent uses read-only tools to explore code and session history,
/// then returns a synthesized text answer.
///
/// Uses the handler pattern — the actual agent construction and execution
/// is delegated to a registered handler (set via `set_deep_search_handler`).
#[derive(Clone, Debug)]
pub struct DeepSearchTool {
    /// Parent session ID — used for handler lookup
    pub session_id: Uuid,
}

impl DeepSearchTool {
    /// Create a new DeepSearchTool instance
    ///
    /// # Arguments
    /// * `session_id` - The parent session ID (for handler lookup)
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

impl Tool for DeepSearchTool {
    const NAME: &'static str = "DeepSearch";

    type Error = ToolError;
    type Args = DeepSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "DeepSearch".to_string(),
            description: concat!(
                "Execute a deep search over a scoped corpus of code files and session history. ",
                "Spawns an ephemeral sub-agent that explores the specified scope using read-only ",
                "tools (Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch) and returns a ",
                "synthesized text answer. Use for questions requiring exploration of many files ",
                "or past conversations. Specify scope as a colon-separated list of paths or glob ",
                "patterns (PATH-style), e.g. \"src/auth/:tests/auth/\". Session history is always ",
                "searchable via SessionSearch."
            )
            .to_string(),
            parameters: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The question to answer by exploring the scoped corpus"
                    },
                    "scope": {
                        "type": ["string", "null"],
                        "description": "Colon-separated paths or glob patterns (PATH-style). Optional — when omitted, the sub-agent uses only SessionSearch for session history exploration. Examples: \"src/auth/\", \"src/auth/:tests/auth/\", \"**/*.rs:spec/**/*.feature\""
                    },
                    "max_depth": {
                        "type": ["integer", "null"],
                        "description": "Maximum tool call depth before stopping (default: 50). Prevents runaway sub-agents.",
                        "minimum": 1
                    },
                    "max_recursion_depth": {
                        "type": ["integer", "null"],
                        "description": "Maximum recursion depth for nested DeepSearch calls (default: 2). Controls how many levels of DeepSearch-within-DeepSearch are allowed.",
                        "minimum": 0
                    }
                },
                "required": ["query"]
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
                tool: "DeepSearch",
                message: reason,
            });
        }

        // Validate query is not empty
        if args.query.trim().is_empty() {
            return Err(ToolError::Execution {
                tool: "DeepSearch",
                message: "query is required and must not be empty".to_string(),
            });
        }

        let max_depth = args.max_depth.unwrap_or(DEFAULT_DEEP_SEARCH_MAX_DEPTH);
        let max_recursion_depth = args
            .max_recursion_depth
            .unwrap_or(DEFAULT_MAX_RECURSION_DEPTH);

        // Dispatch to registered handler (async — sub-agent makes LLM API calls)
        execute_deep_search(
            self.session_id,
            args.query,
            args.scope,
            max_depth,
            max_recursion_depth,
        )
        .await
        .map_err(|e| ToolError::Execution {
            tool: "DeepSearch",
            message: e,
        })
    }
}
