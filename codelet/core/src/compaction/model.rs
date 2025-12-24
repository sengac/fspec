//! Data types for context compaction
//!
//! Contains token tracking, conversation turns, tool calls, and results.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// ==========================================
// TOKEN TRACKING
// ==========================================

/// Token usage tracker with cache-aware calculations
///
/// Based on rig's anthropic::completion::Usage but preserves cache granularity
/// that is lost in the generic crate::completion::Usage conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTracker {
    /// Total input tokens (including cached)
    pub input_tokens: u64,
    /// Total output tokens
    pub output_tokens: u64,
    /// Cache read tokens (from Anthropic API)
    pub cache_read_input_tokens: Option<u64>,
    /// Cache creation tokens (from Anthropic API)
    pub cache_creation_input_tokens: Option<u64>,
}

impl TokenTracker {
    /// Calculate effective tokens accounting for 90% cache discount
    ///
    /// Effective tokens = input_tokens - (cache_read_tokens * 0.9)
    ///
    /// This matches codelet's calculateEffectiveTokens (runner.ts:124-129)
    pub fn effective_tokens(&self) -> u64 {
        let cache_read = self.cache_read_input_tokens.unwrap_or(0);
        let cache_discount = (cache_read as f64 * 0.9) as u64;
        self.input_tokens.saturating_sub(cache_discount)
    }

    /// Get total tokens (input + output)
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

// ==========================================
// CONVERSATION TURNS
// ==========================================

/// A conversation turn groups related messages together
///
/// Turns are the unit of compaction, not individual messages.
/// This matches codelet's ConversationTurn structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// User message that started the turn
    pub user_message: String,
    /// Tool calls made during this turn
    pub tool_calls: Vec<ToolCall>,
    /// Results from tool executions
    pub tool_results: Vec<ToolResult>,
    /// Assistant's response
    pub assistant_response: String,
    /// Token count for this turn
    pub tokens: u64,
    /// Timestamp of turn
    pub timestamp: SystemTime,
    /// Whether previous turn had an error
    pub previous_error: Option<bool>,
}

/// Tool call in a conversation turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name (e.g., "Edit", "Write", "Bash")
    pub tool: String,
    /// Tool call ID
    pub id: String,
    /// Tool input parameters (matches TypeScript's 'parameters' field)
    pub parameters: serde_json::Value,
}

impl ToolCall {
    /// Extract file_path from parameters if present
    /// Matches TypeScript: call.parameters.file_path as string
    pub fn file_path(&self) -> Option<String> {
        self.parameters
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }

    /// Extract just the filename from file_path
    /// Matches TypeScript: path.split('/').pop() || path
    pub fn filename(&self) -> Option<String> {
        self.file_path()
            .map(|path| path.split('/').next_back().unwrap_or(&path).to_string())
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether tool execution succeeded
    pub success: bool,
    /// Tool output
    pub output: String,
    /// Optional error message (matches TypeScript interface)
    pub error: Option<String>,
}

// ==========================================
// CTX-001: BUILD STATUS
// ==========================================

/// Build/test status for PreservationContext
///
/// CTX-001 Rule [7]: BuildStatus enum MUST have variants: Passing, Failing, Unknown
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildStatus {
    /// Tests are passing
    Passing,
    /// Tests are failing
    Failing,
    /// Build status unknown
    Unknown,
}

impl Default for BuildStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildStatus::Passing => write!(f, "passing"),
            BuildStatus::Failing => write!(f, "failing"),
            BuildStatus::Unknown => write!(f, "unknown"),
        }
    }
}

// ==========================================
// CTX-001: PRESERVATION CONTEXT
// ==========================================

/// Context to preserve across compaction
///
/// CTX-001 Rule [6]: PreservationContext struct MUST contain:
/// - active_files: Vec<String>
/// - current_goals: Vec<String>
/// - error_states: Vec<String>
/// - build_status: BuildStatus enum
/// - last_user_intent: String
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreservationContext {
    /// Files being actively edited (extracted from Edit/Write/Read tool calls)
    pub active_files: Vec<String>,
    /// Current goals (extracted from user messages)
    pub current_goals: Vec<String>,
    /// Error states (extracted from failed tool results)
    pub error_states: Vec<String>,
    /// Build/test status (detected from test output)
    pub build_status: BuildStatus,
    /// Last user intent (from most recent user message)
    pub last_user_intent: String,
}

impl PreservationContext {
    /// Create a new empty PreservationContext
    pub fn new() -> Self {
        Self::default()
    }

    /// CTX-001 Rule [8]: Extract PreservationContext from conversation turns
    ///
    /// - Extract active files from Edit/Write/Read tool calls
    /// - Extract goals from user messages
    /// - Extract error states from failed tool results
    /// - Detect build status from test outputs
    /// - Extract last user intent from most recent turn
    pub fn extract_from_turns(turns: &[ConversationTurn]) -> Self {
        let mut ctx = Self::new();

        for turn in turns {
            // Extract active files from Edit/Write/Read tool calls
            for call in &turn.tool_calls {
                if call.tool == "Edit" || call.tool == "Write" || call.tool == "Read" {
                    if let Some(filename) = call.filename() {
                        if !ctx.active_files.contains(&filename) {
                            ctx.active_files.push(filename);
                        }
                    }
                }
            }

            // Extract goals from user messages (look for action verbs)
            let goal = Self::extract_goal_from_message(&turn.user_message);
            if let Some(g) = goal {
                if !ctx.current_goals.contains(&g) {
                    ctx.current_goals.push(g);
                }
            }

            // Extract error states from failed tool results
            for result in &turn.tool_results {
                if !result.success {
                    // Look for error messages in output
                    let error_msg = Self::extract_error_message(&result.output);
                    if let Some(e) = error_msg {
                        if !ctx.error_states.contains(&e) {
                            ctx.error_states.push(e);
                        }
                    }
                }

                // Detect build status from test outputs
                ctx.build_status = Self::detect_build_status(&result.output, ctx.build_status);
            }
        }

        // Extract last user intent from most recent turn
        if let Some(last_turn) = turns.last() {
            ctx.last_user_intent = Self::extract_intent(&last_turn.user_message);
        }

        ctx
    }

    /// Extract goal from user message
    ///
    /// Looks for action verbs like "fix", "implement", "add", "update", etc.
    fn extract_goal_from_message(message: &str) -> Option<String> {
        let message_lower = message.to_lowercase();

        // Keywords that indicate a goal
        let action_verbs = [
            "fix", "implement", "add", "update", "create", "build", "deploy", "refactor", "remove",
            "delete", "change", "modify", "help me", "please",
        ];

        for verb in &action_verbs {
            if message_lower.contains(verb) {
                // Capitalize first letter and return trimmed goal
                let trimmed = message.trim();
                if !trimmed.is_empty() {
                    // Capitalize first letter
                    let mut chars = trimmed.chars();
                    if let Some(first) = chars.next() {
                        return Some(format!("{}{}", first.to_uppercase(), chars.collect::<String>()));
                    }
                }
            }
        }

        None
    }

    /// Extract error message from tool output
    fn extract_error_message(output: &str) -> Option<String> {
        // Look for common error patterns
        let error_patterns = ["error:", "Error:", "ERROR:", "failed:", "FAILED:"];

        for pattern in &error_patterns {
            if let Some(pos) = output.find(pattern) {
                // Extract the error line
                let error_start = pos;
                let error_line = output[error_start..]
                    .lines()
                    .next()
                    .unwrap_or(&output[error_start..]);
                return Some(error_line.to_string());
            }
        }

        None
    }

    /// Detect build status from test output
    fn detect_build_status(output: &str, current: BuildStatus) -> BuildStatus {
        let output_lower = output.to_lowercase();

        // Check for passing indicators
        if output_lower.contains("test") || output_lower.contains("tests") {
            if output_lower.contains("passed") || output_lower.contains("pass") {
                // Also check for failures in the same output
                if output_lower.contains("failed") || output_lower.contains("fail") {
                    return BuildStatus::Failing;
                }
                return BuildStatus::Passing;
            }
            if output_lower.contains("failed") || output_lower.contains("fail") {
                return BuildStatus::Failing;
            }
        }

        current
    }

    /// Extract intent from user message
    fn extract_intent(message: &str) -> String {
        // Get key keywords from the message
        let trimmed = message.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        // Return the message as intent (trimmed to reasonable length)
        if trimmed.len() > 200 {
            format!("{}...", &trimmed[..197])
        } else {
            trimmed.to_string()
        }
    }

    /// CTX-001 Rule [9]: Format PreservationContext for summary
    ///
    /// MUST NOT use hardcoded placeholder text like '[from conversation]',
    /// 'Continue development', or 'unknown' when real data is available.
    pub fn format_for_summary(&self) -> String {
        let files = if self.active_files.is_empty() {
            "none".to_string()
        } else {
            self.active_files.join(", ")
        };

        let goals = if self.current_goals.is_empty() {
            "none".to_string()
        } else {
            self.current_goals.join("; ")
        };

        format!(
            "Active files: {}\nGoals: {}\nBuild: {}",
            files, goals, self.build_status
        )
    }
}

// Note: ConversationFlow was removed - synthetic anchor creation is now handled
// directly in ContextCompactor::compact() to avoid dead code.
