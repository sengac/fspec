//! Data types for context compaction
//!
//! Contains token tracking, conversation turns, tool calls, and results.
//!
//! ## Token Tracking Architecture (CMPCT-001)
//!
//! The system uses three related but distinct token types:
//!
//! ### 1. TokenTracker (Session State)
//! Persistent session state stored in `session.token_tracker`. Contains:
//! - `input_tokens`: TOTAL context size from latest API call (for display/thresholds)
//! - `output_tokens`: CUMULATIVE output tokens across all API calls in session
//! - `cumulative_billed_input/output`: Sum of all API calls (for billing analytics)
//! - `cache_read/creation_input_tokens`: Latest cache values (display only)
//!
//! ### 2. ApiTokenUsage (Per-Request)
//! Located in `codelet_core::token_usage`. Holds raw API response values:
//! - `input_tokens`: Fresh tokens (not from cache, not being cached)
//! - `cache_read_input_tokens`: Tokens read from existing cache
//! - `cache_creation_input_tokens`: Tokens being written to new cache
//! - `output_tokens`: Output tokens from this single request
//! - Provides `total_input()` = input + cache_read + cache_creation
//!
//! ### 3. TokenState (Per-Request in CompactionHook)
//! Internal to `CompactionHook` for threshold checking:
//! - Updated by `on_stream_completion_response_finish`
//! - Checked by `on_completion_call` to trigger compaction
//! - NOT used for display
//!
//! ## Key Insight: Input vs Output Semantics
//!
//! - **input_tokens is ABSOLUTE**: The API reports total context size per call
//!   (not incremental). Use `total_input()` for display and thresholds.
//!
//! - **output_tokens is CUMULATIVE**: The session tracks cumulative output
//!   across all API calls so the next turn continues from the correct value.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// ==========================================
// TOKEN TRACKING
// ==========================================

/// Token usage tracker with cache-aware calculations
///
/// Based on rig's anthropic::completion::Usage but preserves cache granularity
/// that is lost in the generic crate::completion::Usage conversion.
///
/// CTX-003: This struct distinguishes between current context size and cumulative billing:
/// - `input_tokens`: Current context size (latest value - for display and threshold checks)
/// - `cumulative_billed_input`: Sum of all API calls (for billing analytics)
///
/// The Anthropic API reports input_tokens as the TOTAL context size per call (absolute),
/// not incremental tokens added. Display should use input_tokens (current context).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenTracker {
    /// Current context input tokens (latest from API - overwritten, not accumulated)
    /// CTX-003: This is what should be displayed to users and used for threshold checks
    pub input_tokens: u64,
    /// Current context output tokens (latest from API)
    pub output_tokens: u64,
    /// Cumulative billed input tokens (sum of all API calls - for billing analytics)
    /// CTX-003: This is the total billed by Anthropic across all API calls
    #[serde(default)]
    pub cumulative_billed_input: u64,
    /// Cumulative billed output tokens (sum of all API calls)
    #[serde(default)]
    pub cumulative_billed_output: u64,
    /// Cache read tokens (from Anthropic API)
    pub cache_read_input_tokens: Option<u64>,
    /// Cache creation tokens (from Anthropic API)
    pub cache_creation_input_tokens: Option<u64>,
}

impl TokenTracker {
    /// Create a new empty TokenTracker
    pub fn new() -> Self {
        Self::default()
    }

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

    /// Update token tracker with API response (CTX-003)
    ///
    /// - `input_tokens` is OVERWRITTEN with the latest value (for display)
    /// - `cumulative_billed_input` is ACCUMULATED (for billing analytics)
    ///
    /// The Anthropic API reports input_tokens as TOTAL context size per call (absolute),
    /// not incremental tokens added.
    pub fn update(
        &mut self,
        input: u64,
        output: u64,
        cache_read: Option<u64>,
        cache_creation: Option<u64>,
    ) {
        // CTX-003: Overwrite current context (for display and threshold checks)
        self.input_tokens = input;
        self.output_tokens = output;
        // CTX-003: Accumulate for billing analytics
        self.cumulative_billed_input += input;
        self.cumulative_billed_output += output;
        // Cache tokens are per-request values
        self.cache_read_input_tokens = cache_read;
        self.cache_creation_input_tokens = cache_creation;
    }

    /// Update token tracker from ApiTokenUsage with cumulative output (CMPCT-001)
    ///
    /// This method consolidates the duplicated token tracker update pattern found
    /// throughout stream_loop.rs. It handles:
    /// - Setting input_tokens from usage.total_input() (total context for display)
    /// - Setting output_tokens from cumulative output (session-wide accumulator)
    /// - Accumulating billing tokens (input_tokens and output_tokens from usage)
    /// - Setting cache tokens (per-request, not cumulative)
    ///
    /// # Arguments
    /// * `usage` - The ApiTokenUsage from the current turn/request
    /// * `cumulative_output` - The session-wide cumulative output token count
    ///
    /// # Example
    /// ```ignore
    /// // Instead of:
    /// session.token_tracker.input_tokens = turn_usage.total_input();
    /// session.token_tracker.output_tokens = turn_cumulative_output;
    /// session.token_tracker.cumulative_billed_input += turn_usage.input_tokens;
    /// session.token_tracker.cumulative_billed_output += turn_usage.output_tokens;
    /// session.token_tracker.cache_read_input_tokens = Some(turn_usage.cache_read_input_tokens);
    /// session.token_tracker.cache_creation_input_tokens = Some(turn_usage.cache_creation_input_tokens);
    ///
    /// // Use:
    /// session.token_tracker.update_from_usage(&turn_usage, turn_cumulative_output);
    /// ```
    pub fn update_from_usage(&mut self, usage: &crate::token_usage::ApiTokenUsage, cumulative_output: u64) {
        // PROV-001: Store TOTAL context for display and threshold checks
        self.input_tokens = usage.total_input();
        // TUI-031: Save CUMULATIVE output tokens so next turn continues from correct value
        self.output_tokens = cumulative_output;
        // Accumulate for billing analytics (raw uncached input, not total context)
        self.cumulative_billed_input += usage.input_tokens;
        self.cumulative_billed_output += usage.output_tokens;
        // Cache tokens are per-request, not cumulative (use latest values)
        self.cache_read_input_tokens = Some(usage.cache_read_input_tokens);
        self.cache_creation_input_tokens = Some(usage.cache_creation_input_tokens);
    }

    /// Update token tracker for display only, without billing accumulation (CMPCT-001)
    ///
    /// This is used when preparing for a continuation/retry where we want to
    /// update the display values but NOT accumulate billing (to avoid double-counting).
    ///
    /// Use cases:
    /// - Before starting a Gemini continuation loop (display current state)
    /// - After compaction resets (display post-compaction state)
    ///
    /// # Arguments
    /// * `usage` - The ApiTokenUsage from the current turn/request
    /// * `cumulative_output` - The session-wide cumulative output token count
    pub fn update_display_only(&mut self, usage: &crate::token_usage::ApiTokenUsage, cumulative_output: u64) {
        // Update display values only (no billing accumulation)
        self.input_tokens = usage.total_input();
        self.output_tokens = cumulative_output;
        // Cache tokens are per-request values
        self.cache_read_input_tokens = Some(usage.cache_read_input_tokens);
        self.cache_creation_input_tokens = Some(usage.cache_creation_input_tokens);
    }

    /// Reset token tracker after compaction (CMPCT-001)
    ///
    /// After successful compaction, reset output and cache values while
    /// preserving cumulative billing (which tracks total spend across session).
    pub fn reset_after_compaction(&mut self) {
        self.output_tokens = 0;
        self.cache_read_input_tokens = None;
        self.cache_creation_input_tokens = None;
        // Note: cumulative_billed_* is NOT reset - it tracks total session spend
        // Note: input_tokens is set by execute_compaction, not reset here
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BuildStatus {
    /// Tests are passing
    Passing,
    /// Tests are failing
    Failing,
    /// Build status unknown
    #[default]
    Unknown,
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
            "fix",
            "implement",
            "add",
            "update",
            "create",
            "build",
            "deploy",
            "refactor",
            "remove",
            "delete",
            "change",
            "modify",
            "help me",
            "please",
        ];

        for verb in &action_verbs {
            if message_lower.contains(verb) {
                // Capitalize first letter and return trimmed goal
                let trimmed = message.trim();
                if !trimmed.is_empty() {
                    // Capitalize first letter
                    let mut chars = trimmed.chars();
                    if let Some(first) = chars.next() {
                        return Some(format!(
                            "{}{}",
                            first.to_uppercase(),
                            chars.collect::<String>()
                        ));
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
