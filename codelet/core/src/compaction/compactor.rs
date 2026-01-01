//! Main context compaction orchestrator
//!
//! Contains the compaction strategy and orchestrator.

use anyhow::Result;
use codelet_common::token_estimator::count_tokens;

use super::anchor::{AnchorDetector, AnchorPoint, AnchorType};
use super::metrics::{CompactionMetrics, CompactionResult};
use super::model::{ConversationTurn, PreservationContext, ToolCall};
use super::selector::TurnSelector;

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
    /// - Summary message (template-based, matches TypeScript WeightedSummaryProvider)
    /// - Metrics (compression ratio, token counts)
    /// - Warnings (if compression ratio below threshold)
    ///
    /// # Arguments
    /// * `turns` - Conversation turns to compact
    /// * `target_tokens` - Target token count after compaction (budget)
    /// * `_llm_prompt` - UNUSED: Kept for API compatibility, summary is now template-based
    pub async fn compact<F, Fut>(
        &self,
        turns: &[ConversationTurn],
        target_tokens: u64,
        _llm_prompt: F,
    ) -> Result<CompactionResult>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        // Validate parameters
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

        // Step 1b: Create synthetic anchor if no natural anchors found
        // CTX-001 Rule [3]: When no natural anchors are detected, create a synthetic
        // UserCheckpoint at conversation end. This ensures compaction always has a
        // reference point, even for non-coding conversations.
        if anchors.is_empty() && !turns.is_empty() {
            let last_idx = turns.len() - 1;
            let last_turn = &turns[last_idx];
            anchors.push(AnchorPoint {
                turn_index: last_idx,
                anchor_type: AnchorType::UserCheckpoint,
                weight: 0.7,
                confidence: 1.0, // Synthetic anchors have full confidence
                description: "Synthetic checkpoint (no natural anchors detected)".to_string(),
                timestamp: last_turn.timestamp,
            });
        }

        // Step 2: Select turns using TypeScript-matching logic
        let selector = TurnSelector::new();
        let selection = selector.select_turns_with_recent(turns, &anchors)?;

        // Step 3: Calculate original token count
        let original_tokens: u64 = turns.iter().map(|t| t.tokens).sum();

        // Step 4: Generate template-based summary (matches TypeScript WeightedSummaryProvider)
        let summarized_turns: Vec<&ConversationTurn> = selection
            .summarized_turns
            .iter()
            .map(|info| &turns[info.turn_index])
            .collect();

        // Step 5: Collect kept turns (moved up - needed for summary generation)
        let kept_turns: Vec<ConversationTurn> = selection
            .kept_turns
            .iter()
            .map(|info| turns[info.turn_index].clone())
            .collect();

        // Generate summary - extract context from KEPT turns only (not all turns)
        // This prevents completed tasks from appearing as current goals
        let summary = if !summarized_turns.is_empty() {
            self.generate_weighted_summary(&summarized_turns, &anchors, &kept_turns)
        } else {
            "No turns summarized.".to_string()
        };

        // PROV-002: Use tiktoken-rs for accurate token counting
        let summary_tokens = count_tokens(&summary) as u64;

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

        // Step 7: Check compression ratio - WARN instead of FAIL (matches TypeScript)
        let mut warnings = Vec::new();
        if !metrics.meets_threshold(self.min_compression_ratio) {
            warnings.push(format!(
                "Compression ratio below {:.0}% ({:.1}%) - consider starting fresh conversation",
                self.min_compression_ratio * 100.0,
                compression_ratio * 100.0
            ));
        }

        // Use selector's anchor if it found one in older turns,
        // otherwise use the most recent anchor (which may be synthetic)
        // This ensures synthetic anchors are returned even when in "recent" turns
        let result_anchor = selection
            .preserved_anchor
            .or_else(|| anchors.last().cloned());

        Ok(CompactionResult {
            kept_turns,
            warnings,
            summary,
            metrics,
            anchor: result_anchor,
        })
    }

    /// Generate template-based summary (matches TypeScript WeightedSummaryProvider.generateWeightedSummary)
    ///
    /// This is a fast, synchronous, template-based summary - NO LLM CALL.
    /// CTX-001 Rule [9]: Uses dynamic PreservationContext.format_for_summary() - NO hardcoded text.
    ///
    /// # Arguments
    /// * `summarized_turns` - Turns being compressed into outcomes (older turns)
    /// * `anchors` - Detected anchor points
    /// * `kept_turns` - Recent turns being preserved (used for context extraction)
    fn generate_weighted_summary(
        &self,
        summarized_turns: &[&ConversationTurn],
        anchors: &[AnchorPoint],
        kept_turns: &[ConversationTurn],
    ) -> String {
        // Transform summarized turns to outcome descriptions (matches TypeScript turnToOutcome)
        let outcomes: Vec<String> = summarized_turns
            .iter()
            .map(|turn| self.turn_to_outcome(turn, anchors))
            .collect();

        // Extract context from KEPT turns only - prevents completed tasks from appearing as current goals
        let preservation_context = PreservationContext::extract_from_turns(kept_turns);
        let context_summary = preservation_context.format_for_summary();

        format!(
            "{}\n\nKey outcomes:\n{}",
            context_summary,
            outcomes.join("\n")
        )
    }

    /// Convert turn to outcome description (matches TypeScript turnToOutcome)
    fn turn_to_outcome(&self, turn: &ConversationTurn, anchors: &[AnchorPoint]) -> String {
        // Check if this turn is an anchor point
        let is_anchor = anchors.iter().any(|a| {
            // Compare by timestamp (TypeScript uses timestamp comparison)
            a.timestamp == turn.timestamp
        });

        if is_anchor {
            // Anchor points get detailed preservation
            return format!("[ANCHOR] {}", turn.assistant_response);
        }

        // Regular turns get compressed to outcomes
        // Extract modified files (matches TypeScript extractFilesFromTurn)
        let files: Vec<String> = turn
            .tool_calls
            .iter()
            .filter(|call| call.tool == "Edit" || call.tool == "Write")
            .filter_map(ToolCall::filename)
            .collect();

        let success = turn.tool_results.iter().any(|r| r.success);
        let success_marker = if success { "✓" } else { "✗" };

        let file_info = if !files.is_empty() {
            format!("Modified {}: ", files.join(", "))
        } else {
            String::new()
        };

        // Get first sentence of assistant response
        let first_sentence = turn
            .assistant_response
            .split('.')
            .next()
            .unwrap_or(&turn.assistant_response);

        format!("{success_marker} {file_info}{first_sentence}")
    }
}

impl Default for ContextCompactor {
    fn default() -> Self {
        Self::new()
    }
}
