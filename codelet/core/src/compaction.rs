//! Context Compaction with Anchoring System
//!
//! This module implements intelligent context compaction using:
//! - Anchor point detection (error resolution, task completion)
//! - Turn-based architecture (grouping messages into conversation turns)
//! - LLM-based summarization with retry logic
//! - Cache-aware token tracking
//!
//! Reference implementation: codelet's anchor-point-compaction.ts

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tracing::warn;

// ==========================================
// RETRY CONFIGURATION (CLI-018)
// ==========================================

/// Retry configuration for LLM summary generation
///
/// Implements exponential backoff: 0ms, 1000ms, 2000ms
const RETRY_DELAYS_MS: [u64; 3] = [0, 1000, 2000];

/// Fallback summary when all retries fail
const FALLBACK_SUMMARY: &str = "[Summary generation failed after multiple attempts. Conversation context has been preserved but not summarized.]";

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
    /// Tool input parameters
    pub input: serde_json::Value,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether tool execution succeeded
    pub success: bool,
    /// Tool output or error message
    pub output: String,
}

// ==========================================
// ANCHOR POINTS
// ==========================================

/// Anchor point types
///
/// Matches codelet's anchor types with their associated weights:
/// - ErrorResolution: weight 0.9
/// - TaskCompletion: weight 0.8
/// - UserCheckpoint: weight 0.7
/// - FeatureMilestone: weight 0.75
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorType {
    /// Error was resolved (previous error + fix + test pass)
    ErrorResolution,
    /// Task was completed (modify + test + success, no previous error)
    TaskCompletion,
    /// User created explicit checkpoint
    UserCheckpoint,
    /// Feature milestone reached
    FeatureMilestone,
}

impl AnchorType {
    /// Get weight for this anchor type
    ///
    /// Matches codelet's weights in anchor-point-compaction.ts
    pub fn weight(&self) -> f64 {
        match self {
            AnchorType::ErrorResolution => 0.9,
            AnchorType::TaskCompletion => 0.8,
            AnchorType::FeatureMilestone => 0.75,
            AnchorType::UserCheckpoint => 0.7,
        }
    }
}

/// Anchor point in conversation history
///
/// Marks a significant point where context compaction should preserve information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorPoint {
    /// Index of turn in conversation history
    pub turn_index: usize,
    /// Type of anchor
    pub anchor_type: AnchorType,
    /// Weight for preservation (0.7-0.9)
    pub weight: f64,
    /// Detection confidence (0.0-1.0)
    pub confidence: f64,
    /// Human-readable description
    pub description: String,
    /// Timestamp when anchor was created
    pub timestamp: SystemTime,
}

// ==========================================
// COMPACTION METRICS
// ==========================================

/// Metrics from a compaction operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionMetrics {
    /// Original token count before compaction
    pub original_tokens: u64,
    /// Token count after compaction
    pub compacted_tokens: u64,
    /// Compression ratio as percentage (0-100)
    pub compression_ratio: f64,
    /// Number of turns summarized
    pub turns_summarized: usize,
    /// Number of turns kept
    pub turns_kept: usize,
}

impl CompactionMetrics {
    /// Check if compression ratio meets minimum threshold
    pub fn meets_threshold(&self, min_ratio: f64) -> bool {
        self.compression_ratio >= min_ratio
    }
}

// ==========================================
// COMPACTION STRATEGY
// ==========================================

/// Strategy for context compaction
#[derive(Debug, Clone, Copy)]
pub enum CompactionStrategy {
    /// Anchor-based compaction (preserve from last anchor)
    AnchorBased,
    /// Simple truncation (keep last N turns)
    SimpleTruncate { keep_last: usize },
    /// No compaction
    None,
}

// ==========================================
// CONTEXT COMPACTOR
// ==========================================

/// Main context compaction orchestrator
///
/// Implements the Factory AI anchored summary algorithm:
/// 1. Detect anchor points in conversation history
/// 2. Select turns to keep vs summarize based on anchors
/// 3. Generate LLM summary of old turns
/// 4. Reconstruct message history (append-only)
/// 5. Clear prompt cache (context changed)
pub struct ContextCompactor {
    /// Minimum confidence threshold for anchor detection (default: 0.9)
    confidence_threshold: f64,
    /// Minimum compression ratio threshold (default: 0.6 = 60%)
    min_compression_ratio: f64,
    /// Compaction strategy
    strategy: CompactionStrategy,
}

impl ContextCompactor {
    /// Create a new context compactor with default settings
    pub fn new() -> Self {
        Self {
            confidence_threshold: 0.9,
            min_compression_ratio: 0.6,
            strategy: CompactionStrategy::AnchorBased,
        }
    }

    /// Create compactor with custom confidence threshold
    pub fn with_confidence_threshold(mut self, threshold: f64) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    /// Create compactor with custom compression ratio threshold
    pub fn with_compression_threshold(mut self, threshold: f64) -> Self {
        self.min_compression_ratio = threshold;
        self
    }

    /// Create compactor with specific strategy
    pub fn with_strategy(mut self, strategy: CompactionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Compact conversation turns using configured strategy
    ///
    /// Returns CompactionResult containing:
    /// - Kept turns (preserved from anchor point)
    /// - Summary message (LLM-generated summary of old turns)
    /// - Metrics (compression ratio, token counts)
    ///
    /// # Arguments
    /// * `turns` - Conversation turns to compact
    /// * `target_tokens` - Target token count after compaction (budget)
    ///   Note: Currently not used in anchor-based selection, but accepted for API compatibility
    /// * `llm_prompt` - Function to generate summaries via LLM
    pub async fn compact<F, Fut>(
        &self,
        turns: &[ConversationTurn],
        target_tokens: u64,
        llm_prompt: F,
    ) -> Result<CompactionResult>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        // Validate budget parameter (even though not currently used in selection)
        if target_tokens == 0 {
            anyhow::bail!("Target tokens must be positive");
        }
        if turns.is_empty() {
            anyhow::bail!("Cannot compact empty turn history");
        }

        // Step 1: Detect anchor points
        let detector = AnchorDetector::new(self.confidence_threshold);
        let mut anchors: Vec<AnchorPoint> = Vec::new();

        for (idx, turn) in turns.iter().enumerate() {
            if let Some(anchor) = detector.detect(turn, idx)? {
                anchors.push(anchor);
            }
        }

        // Step 2: Select turns based on anchor (use last/strongest anchor if multiple)
        let selector = TurnSelector::new();
        let selected_anchor = anchors.last();
        let selection = selector.select_turns(turns, selected_anchor)?;

        // Step 3: Calculate original token count
        let original_tokens: u64 = turns.iter().map(|t| t.tokens).sum();

        // Step 4: Generate summary for turns that will be summarized
        let summary = if !selection.summarized_turns.is_empty() {
            self.generate_summary(turns, &selection.summarized_turns, &llm_prompt)
                .await?
        } else {
            "No turns summarized.".to_string()
        };

        // Estimate summary tokens (rough approximation: 1 token ≈ 4 characters)
        let summary_tokens = (summary.len() / 4) as u64;

        // Step 5: Collect kept turns
        let kept_turns: Vec<ConversationTurn> = selection
            .kept_turns
            .iter()
            .map(|info| turns[info.turn_index].clone())
            .collect();

        let kept_tokens: u64 = kept_turns.iter().map(|t| t.tokens).sum();
        let compacted_tokens = summary_tokens + kept_tokens;

        // Step 6: Calculate metrics
        let compression_ratio = if original_tokens > 0 {
            1.0 - (compacted_tokens as f64 / original_tokens as f64)
        } else {
            0.0
        };

        let metrics = CompactionMetrics {
            original_tokens,
            compacted_tokens,
            compression_ratio,
            turns_summarized: selection.summarized_turns.len(),
            turns_kept: selection.kept_turns.len(),
        };

        // Check if compression meets minimum threshold
        if !metrics.meets_threshold(self.min_compression_ratio) {
            anyhow::bail!(
                "Compaction did not meet minimum compression ratio: {:.1}% < {:.1}%",
                compression_ratio * 100.0,
                self.min_compression_ratio * 100.0
            );
        }

        Ok(CompactionResult {
            kept_turns,
            summary,
            metrics,
            anchor: selected_anchor.cloned(),
        })
    }

    /// Generate LLM summary of turns being compacted
    ///
    /// CLI-018: Implements retry logic with exponential backoff (0ms, 1000ms, 2000ms)
    /// and fallback behavior if all retries fail.
    async fn generate_summary<F, Fut>(
        &self,
        turns: &[ConversationTurn],
        summarized_turn_infos: &[TurnInfo],
        llm_prompt: &F,
    ) -> Result<String>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        // Build prompt for summarization
        let mut prompt = String::from(
            "Summarize the following conversation turns concisely, preserving key information:\n\n",
        );

        for turn_info in summarized_turn_infos {
            let turn = &turns[turn_info.turn_index];
            prompt.push_str(&format!("User: {}\n", turn.user_message));

            if !turn.tool_calls.is_empty() {
                prompt.push_str("Tools used: ");
                let tools: Vec<String> = turn.tool_calls.iter().map(|tc| tc.tool.clone()).collect();
                prompt.push_str(&tools.join(", "));
                prompt.push('\n');
            }

            prompt.push_str(&format!("Assistant: {}\n\n", turn.assistant_response));
        }

        prompt.push_str(
            "Provide a concise summary (2-3 paragraphs) that captures the essential information.",
        );

        // CLI-018: Retry logic with exponential backoff
        let mut last_error = None;

        for (attempt, &delay_ms) in RETRY_DELAYS_MS.iter().enumerate() {
            // Apply delay before retry (skip delay on first attempt)
            if delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }

            match llm_prompt(prompt.clone()).await {
                Ok(response) => {
                    return Ok(response);
                }
                Err(e) => {
                    warn!(
                        attempt = attempt + 1,
                        max_attempts = RETRY_DELAYS_MS.len(),
                        error = %e,
                        "LLM summary generation failed, retrying..."
                    );
                    last_error = Some(e);
                }
            }
        }

        // CLI-018: All retries exhausted - use fallback summary
        warn!(
            error = ?last_error,
            "All LLM summary retries exhausted, using fallback summary"
        );
        Ok(FALLBACK_SUMMARY.to_string())
    }
}

impl Default for ContextCompactor {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a compaction operation
#[derive(Debug, Clone)]
pub struct CompactionResult {
    /// Turns that were kept (from anchor point forward)
    pub kept_turns: Vec<ConversationTurn>,
    /// LLM-generated summary of summarized turns
    pub summary: String,
    /// Compaction metrics
    pub metrics: CompactionMetrics,
    /// Anchor point used (if any)
    pub anchor: Option<AnchorPoint>,
}

// ==========================================
// ANCHOR DETECTION
// ==========================================

/// Anchor detector for identifying conversation breakpoints
pub struct AnchorDetector {
    confidence_threshold: f64,
}

impl AnchorDetector {
    /// Create new anchor detector with confidence threshold
    pub fn new(confidence_threshold: f64) -> Self {
        Self {
            confidence_threshold,
        }
    }

    /// Detect anchor point in a conversation turn
    ///
    /// Returns Some(AnchorPoint) if confidence >= threshold, None otherwise
    pub fn detect(
        &self,
        turn: &ConversationTurn,
        turn_index: usize,
    ) -> Result<Option<AnchorPoint>> {
        // Analyze completion patterns (matches codelet's analyzeCompletionPatterns)
        let has_test_success = turn.tool_results.iter().any(|result| {
            result.success
                && result.output.to_lowercase().contains("test")
                && (result.output.contains("pass") || result.output.contains("success"))
        });

        let has_file_modification = turn
            .tool_calls
            .iter()
            .any(|call| call.tool == "Edit" || call.tool == "Write");

        let has_previous_error = turn.previous_error.unwrap_or(false);

        // Error resolution pattern: Previous error + Fix + Success
        if has_previous_error && has_file_modification && has_test_success {
            let confidence = 0.95;
            if confidence >= self.confidence_threshold {
                return Ok(Some(AnchorPoint {
                    turn_index,
                    anchor_type: AnchorType::ErrorResolution,
                    weight: 0.9,
                    confidence,
                    description: "Build error fixed and tests now pass".to_string(),
                    timestamp: turn.timestamp,
                }));
            }
        }

        // Task completion pattern: Modify + Test + Success (without previous error)
        if !has_previous_error && has_file_modification && has_test_success {
            let confidence = 0.92;
            if confidence >= self.confidence_threshold {
                return Ok(Some(AnchorPoint {
                    turn_index,
                    anchor_type: AnchorType::TaskCompletion,
                    weight: 0.8,
                    confidence,
                    description: "File changes implemented and tests pass".to_string(),
                    timestamp: turn.timestamp,
                }));
            }
        }

        Ok(None)
    }
}

// ==========================================
// TURN SELECTION
// ==========================================

/// Turn selector for anchor-based compaction strategy
pub struct TurnSelector;

impl TurnSelector {
    /// Create new turn selector
    pub fn new() -> Self {
        Self
    }

    /// Select turns to keep vs summarize based on anchor point
    ///
    /// If anchor exists: keep from anchor forward, summarize before
    /// If no anchor: keep last 2-3 turns, summarize the rest
    pub fn select_turns(
        &self,
        turns: &[ConversationTurn],
        anchor: Option<&AnchorPoint>,
    ) -> Result<TurnSelection> {
        if turns.is_empty() {
            return Ok(TurnSelection {
                kept_turns: Vec::new(),
                summarized_turns: Vec::new(),
            });
        }

        match anchor {
            Some(anchor_point) => {
                // Keep turns from anchor point forward (inclusive)
                let kept_turns: Vec<TurnInfo> = (anchor_point.turn_index..turns.len())
                    .map(|idx| TurnInfo { turn_index: idx })
                    .collect();

                // Summarize turns before anchor point
                let summarized_turns: Vec<TurnInfo> = (0..anchor_point.turn_index)
                    .map(|idx| TurnInfo { turn_index: idx })
                    .collect();

                Ok(TurnSelection {
                    kept_turns,
                    summarized_turns,
                })
            }
            None => {
                // No anchor: keep last 2-3 turns, summarize the rest
                let keep_count = 3.min(turns.len());
                let summarize_count = turns.len().saturating_sub(keep_count);

                let kept_turns: Vec<TurnInfo> = (summarize_count..turns.len())
                    .map(|idx| TurnInfo { turn_index: idx })
                    .collect();

                let summarized_turns: Vec<TurnInfo> = (0..summarize_count)
                    .map(|idx| TurnInfo { turn_index: idx })
                    .collect();

                Ok(TurnSelection {
                    kept_turns,
                    summarized_turns,
                })
            }
        }
    }
}

impl Default for TurnSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of turn selection
#[derive(Debug)]
pub struct TurnSelection {
    /// Turns to keep (from anchor forward)
    pub kept_turns: Vec<TurnInfo>,
    /// Turns to summarize (before anchor)
    pub summarized_turns: Vec<TurnInfo>,
}

/// Information about a turn in selection result
#[derive(Debug)]
pub struct TurnInfo {
    /// Index of turn in conversation history
    pub turn_index: usize,
}
