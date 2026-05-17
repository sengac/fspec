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
        // TUI-031: Save CUMULATIVE output tokens so next turn continues from correct value.
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

    /// Compute the per-turn `output_tokens` delta from a new cumulative
    /// display value (TOKEN-001).
    ///
    /// The streaming display reports cumulative output tokens across the
    /// entire session. To feed per-turn billing into
    /// [`Self::update_from_usage`], call sites must subtract the previously
    /// observed cumulative (stored in `self.output_tokens`) from the new
    /// cumulative. Using `saturating_sub` guarantees that the delta stays
    /// non-negative even when the cumulative display ticks backward (for
    /// example after `reset_after_compaction` or when a provider briefly
    /// reports a smaller running total).
    ///
    /// This is the SINGLE canonical source of the
    /// `saturating_sub(.., output_tokens)` pattern in the streaming code
    /// paths — all four `update_from_usage` call sites in
    /// `codelet-cli::interactive` delegate to it so the rule stays DRY.
    ///
    /// # Arguments
    /// * `current_cumulative_output` — the new session-wide cumulative
    ///   output token count reported by the streaming display.
    ///
    /// # Returns
    /// The per-turn delta to pass as the `output_tokens` argument to
    /// [`crate::token_usage::ApiTokenUsage::new`].
    #[inline]
    pub fn compute_output_delta(&self, current_cumulative_output: u64) -> u64 {
        current_cumulative_output.saturating_sub(self.output_tokens)
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

// ==========================================
// DAG NODE METADATA (CMPCT-017)
// ==========================================

/// Wrap DAG content in system-reminder compaction-dag markers.
///
/// The resulting message content will be:
/// ```text
/// <system-reminder>
/// <!-- type:compaction-dag -->
/// {dag_content}
/// </system-reminder>
/// ```
///
/// This is the single canonical wrapping function. Used by both
/// `inject_summary_handler::on_injected` (napi) and
/// `compaction_dag::force_inject_fallback_dag` (cli).
pub fn wrap_dag_content(content: &str) -> String {
    format!("<system-reminder>\n<!-- type:compaction-dag -->\n{content}\n</system-reminder>")
}

/// Depth level of a DAG summary node.
///
/// Maps to the hierarchical compaction model:
/// - D0 (Detailed): Granular recent work — exact files, errors, decisions
/// - D1 (Arc): Current work state — promoted from D0 on re-compaction
/// - D2 (Durable): Architecture decisions, milestones that survive many compactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DagDepth {
    /// Detailed — recent work, most granular
    D0,
    /// Arc — current work state, promoted from D0
    D1,
    /// Durable — architecture decisions, milestones
    D2,
}

/// Structured metadata for a parsed `<dag-node>` block.
///
/// Extracted from the agent's DAG content after inject_summary.
/// Provides provenance turn ranges and depth classification for
/// downstream features (scoped queries, incremental condensation,
/// convergence watchdog, file propagation).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DagNodeMeta {
    /// Depth level (D0/D1/D2)
    pub depth: DagDepth,
    /// Start of the turn range this node summarizes (inclusive, 0-based)
    pub turn_start: usize,
    /// End of the turn range this node summarizes (inclusive, 0-based)
    pub turn_end: usize,
    /// Short human-readable label for the node
    pub label: String,
}

/// Parse `<dag-node>` blocks from DAG content and extract structured metadata.
///
/// Uses regex to match XML-like `<dag-node depth="Dx" turns="N-M" label="...">` blocks.
/// Invalid or malformed nodes are silently skipped. If `message_count` is provided,
/// `turn_end` values exceeding `message_count - 1` are clamped.
///
/// **Range validation (CMPCT-035 / FV-003-a):** Blocks where the parsed
/// `turn_start > turn_end` (BEFORE clamping) are rejected and logged with a
/// `tracing::warn!`. This enforces the formal-model invariant
/// `turn_start <= turn_end` at the parse boundary.
///
/// **Out-of-range start rejection (CMPCT-037 / FV-003-c):** When
/// `message_count` is provided and `turn_start >= message_count`, the entire
/// node is rejected and logged with a `tracing::warn!` BEFORE any clamping.
/// This prevents the residual contract `turn_start < message_count` from
/// being violated and, transitively, prevents the `turn_end` clamp from
/// producing an inverted range. With this gate in place, every emitted node
/// satisfies `turn_start <= turn_end` AND `turn_end < message_count`.
///
/// **Same-depth overlap rejection (CMPCT-036 / FV-003-b):** After sorting by
/// `turn_start`, any node whose `[turn_start, turn_end]` interval overlaps a
/// previously-accepted node *at the same depth* is dropped and logged with a
/// `tracing::warn!`. The earlier (lower `turn_start`, then first-encountered)
/// node is kept. Adjacency at the boundary
/// (`next.turn_start == prior.turn_end`) counts as overlap because `turn_end`
/// is inclusive. Cross-depth overlap (e.g., a D2 node spanning the same turns
/// as a D1 node) is intentional and accepted — hierarchical compaction
/// depends on it.
///
/// Returns nodes sorted by `turn_start` ascending.
pub fn parse_dag_nodes(dag_content: &str, message_count: Option<usize>) -> Vec<DagNodeMeta> {
    use once_cell::sync::Lazy;
    use regex::Regex;

    // Compiled once, reused across all calls
    static DAG_NODE_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"<dag-node\s+depth="(D[012])"\s+turns="(\d+)-(\d+)"\s+label="([^"]+)">"#)
            .unwrap_or_else(|_| {
                // SAFETY: "^$" is a trivially valid regex — unwrap is infallible.
                #[allow(clippy::expect_used)]
                Regex::new("^$").expect("infallible fallback regex")
            })
    });

    let mut nodes: Vec<DagNodeMeta> = DAG_NODE_RE
        .captures_iter(dag_content)
        .filter_map(|cap| {
            let depth = match &cap[1] {
                "D0" => DagDepth::D0,
                "D1" => DagDepth::D1,
                "D2" => DagDepth::D2,
                _ => return None, // unreachable given regex, but defensive
            };

            let turn_start: usize = cap[2].parse().ok()?;
            let mut turn_end: usize = cap[3].parse().ok()?;
            let label = &cap[4];

            // CMPCT-035 / FV-003-a: reject reversed turn ranges at the parse
            // boundary. This enforces the formal model's invariant
            // `turn_start <= turn_end`. Validated PRE-clamping so that
            // clamping logic (handled below) never sees a reversed range.
            if turn_start > turn_end {
                tracing::warn!(
                    turn_start,
                    turn_end,
                    label = %label,
                    "Skipping dag-node with inverted turn range (turn_start > turn_end)"
                );
                return None;
            }

            // CMPCT-037 / FV-003-c: when `message_count` is provided, reject
            // any node whose `turn_start` is at or beyond `message_count` —
            // such a range refers to non-existent turns. This MUST happen
            // BEFORE the `turn_end` clamp; otherwise clamping `turn_end` to
            // `message_count - 1` would silently produce an inverted output
            // range (`turn_start > turn_end`). With this gate in place,
            // `turn_start < message_count` holds for every surviving node,
            // so the subsequent `turn_end` clamp can never invert the range.
            //
            // `Some(0)` is the degenerate case (zero turns exist): every
            // node has `turn_start >= 0 == message_count` and is therefore
            // rejected.
            if let Some(mc) = message_count {
                if turn_start >= mc {
                    tracing::warn!(
                        turn_start,
                        message_count = mc,
                        label = %label,
                        "Skipping dag-node whose turn_start is at or beyond message_count"
                    );
                    return None;
                }
                // turn_start < mc here, so clamping `turn_end` down to
                // `mc - 1` preserves `turn_start <= turn_end`.
                let max = mc - 1;
                if turn_end > max {
                    turn_end = max;
                }
            }

            Some(DagNodeMeta {
                depth,
                turn_start,
                turn_end,
                label: label.to_string(),
            })
        })
        .collect();

    // CMPCT-036 / FV-003-b: enforce G2 (SameDepthNonOverlapping) at the parse
    // boundary. Sort by `turn_start` ascending (P1) and reject any node whose
    // interval overlaps a previously-accepted same-depth node. Adjacency at
    // the boundary (`turn_start == prior.turn_end`) counts as overlap because
    // `turn_end` is inclusive. Cross-depth overlap is intentional and
    // accepted.
    nodes.sort_by_key(|n| n.turn_start);

    // Index by `DagDepth` ordinal so we can track the most-recently-accepted
    // node at each depth without requiring `Hash` on the enum.
    fn depth_idx(d: DagDepth) -> usize {
        match d {
            DagDepth::D0 => 0,
            DagDepth::D1 => 1,
            DagDepth::D2 => 2,
        }
    }

    let mut last_per_depth: [Option<DagNodeMeta>; 3] = [None, None, None];
    let mut filtered: Vec<DagNodeMeta> = Vec::with_capacity(nodes.len());

    for node in nodes {
        let idx = depth_idx(node.depth);
        if let Some(last) = &last_per_depth[idx] {
            if node.turn_start <= last.turn_end {
                tracing::warn!(
                    depth = ?node.depth,
                    kept_turn_start = last.turn_start,
                    kept_turn_end = last.turn_end,
                    kept_label = %last.label,
                    dropped_turn_start = node.turn_start,
                    dropped_turn_end = node.turn_end,
                    dropped_label = %node.label,
                    "Dropping overlapping same-depth dag-node"
                );
                continue;
            }
        }
        last_per_depth[idx] = Some(node.clone());
        filtered.push(node);
    }

    filtered
}

