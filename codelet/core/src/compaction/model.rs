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
    /// Reasoning/thinking tokens (OpenAI o-series, Codex extended thinking)
    #[serde(default)]
    pub reasoning_tokens: u64,
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

    /// Get total tokens (input + output + reasoning)
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.reasoning_tokens
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
        // Reasoning tokens from the latest request
        self.reasoning_tokens = usage.reasoning_tokens;
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
        // Reasoning tokens from the latest request
        self.reasoning_tokens = usage.reasoning_tokens;
    }

    /// Reset token tracker after compaction (CMPCT-001)
    ///
    /// After successful compaction, reset output and cache values while
    /// preserving cumulative billing (which tracks total spend across session).
    pub fn reset_after_compaction(&mut self) {
        self.output_tokens = 0;
        self.reasoning_tokens = 0;
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
// STRUCTURAL ANNOTATIONS
// ==========================================

/// File operation type for FileModification annotations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOp {
    /// File was created (new file)
    Created,
    /// File was modified (existing file changed)
    Modified,
    /// File was deleted
    Deleted,
}

/// Per-turn structural annotation for zero-cost metadata during DAG construction.
///
/// These annotations provide structural milestones (fspec status changes, error
/// resolutions, file modifications) that the agent can use as navigation aids
/// during SessionSearch retrieval without reading full turn content.
///
/// Replaces the old flaky heuristic-based state detection
/// with explicit, per-turn structural signals.
///
/// Research: ACON (Kang et al., KAIST/Microsoft, arXiv:2510.00615) demonstrates
/// that preserving WHY decisions were made is critical for continuation quality.
/// HiAgent (Hu et al., ACL 2025) validates that agent-controlled compression
/// boundaries based on structural signals produce better results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StructuralAnnotation {
    /// fspec milestone reached (from Fspec tool call with specific command)
    FspecMilestone {
        /// The fspec command that was called (e.g., "update-work-unit-status")
        command: String,
        /// Arguments passed to the command (e.g., ["AUTH-001", "implementing"])
        args: Vec<String>,
    },
    /// Error was resolved (previous failure + current file modification + all-success)
    ErrorResolution {
        /// The tool that previously failed (e.g., "Bash")
        failed_tool: String,
        /// The file that was modified to resolve the error
        resolved_file: String,
    },
    /// File modification (from Edit/Write tool calls)
    FileModification {
        /// Path of the modified file
        path: String,
        /// Type of file operation
        operation: FileOp,
    },
}

