//! Main context compaction orchestrator
//!
//! Contains the compaction strategy and orchestrator.

use anyhow::Result;
use codelet_common::token_estimator::count_tokens;
use tracing::warn;

use super::anchor::{AnchorDetector, AnchorPoint};
use super::metrics::{CompactionMetrics, CompactionResult};
use super::model::{ConversationTurn, PreservationContext};
use super::selector::TurnSelector;

// ==========================================
// RETRY CONFIGURATION
// ==========================================

/// Retry delays for LLM summary generation (exponential backoff: 0ms, 1000ms, 2000ms)
const RETRY_DELAYS_MS: [u64; 3] = [0, 1000, 2000];

/// Fallback summary when all retries fail
const FALLBACK_SUMMARY: &str = "[Summary generation failed after multiple attempts. Conversation context has been preserved but not summarized.]";

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
    /// - Summary message (LLM-generated summary of compacted turns)
    /// - Metrics (compression ratio, token counts)
    /// - Warnings (if compression ratio below threshold)
    ///
    /// # Arguments
    /// * `turns` - Conversation turns to compact
    /// * `target_tokens` - Target token count after compaction (budget)
    /// * `llm_prompt` - LLM function for anchor detection and summary generation
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
        // Validate parameters
        if target_tokens == 0 {
            anyhow::bail!("Target tokens must be positive");
        }
        if turns.is_empty() {
            anyhow::bail!("Cannot compact empty turn history");
        }

        // Step 1: Detect anchor points using batch LLM analysis
        let detector = AnchorDetector::new(self.confidence_threshold);
        let anchors = detector.detect_batch(turns, &llm_prompt).await?;

        // Step 1b: Create synthetic anchor if no natural anchors found
        let anchors = if anchors.is_empty() && !turns.is_empty() {
            let last_idx = turns.len() - 1;
            let last_turn = &turns[last_idx];
            vec![AnchorPoint::synthetic_checkpoint(
                last_idx,
                last_turn,
                "no natural anchors detected",
            )]
        } else {
            anchors
        };

        // Step 2: Select turns using turn selector
        let selector = TurnSelector::new();
        let selection = selector.select_turns_with_recent(turns, &anchors)?;

        // Step 3: Calculate original token count
        let original_tokens: u64 = turns.iter().map(|t| t.tokens).sum();

        // Step 4: Collect summarized turns
        let summarized_turns: Vec<&ConversationTurn> = selection
            .summarized_turns
            .iter()
            .map(|info| &turns[info.turn_index])
            .collect();

        // Step 5: Collect kept turns
        let kept_turns: Vec<ConversationTurn> = selection
            .kept_turns
            .iter()
            .map(|info| turns[info.turn_index].clone())
            .collect();

        // Step 6: Generate LLM summary with retry logic
        let summary = if !summarized_turns.is_empty() {
            self.generate_llm_summary(&summarized_turns, &anchors, &kept_turns, &llm_prompt)
                .await
        } else {
            "No turns summarized.".to_string()
        };

        // Step 7: Calculate metrics
        let summary_tokens = count_tokens(&summary) as u64;
        let kept_tokens: u64 = kept_turns.iter().map(|t| t.tokens).sum();
        let compacted_tokens = summary_tokens + kept_tokens;

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

        // Step 8: Check compression ratio - WARN instead of FAIL
        let mut warnings = Vec::new();
        if !metrics.meets_threshold(self.min_compression_ratio) {
            warnings.push(format!(
                "Compression ratio below {:.0}% ({:.1}%) - consider starting fresh conversation",
                self.min_compression_ratio * 100.0,
                compression_ratio * 100.0
            ));
        }

        // Use selector's anchor if it found one in older turns,
        // otherwise use the most recent LLM-detected anchor
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

    /// Generate LLM summary of turns being compacted with retry logic
    ///
    /// Implements exponential backoff retry (0ms, 1000ms, 2000ms) and fallback behavior.
    async fn generate_llm_summary<F, Fut>(
        &self,
        summarized_turns: &[&ConversationTurn],
        anchors: &[AnchorPoint],
        kept_turns: &[ConversationTurn],
        llm_prompt: &F,
    ) -> String
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        // Build the summarization prompt
        let prompt = self.build_summary_prompt(summarized_turns, anchors, kept_turns);

        // Retry logic with exponential backoff
        let mut last_error = None;

        for (attempt, &delay_ms) in RETRY_DELAYS_MS.iter().enumerate() {
            // Apply delay before retry (skip delay on first attempt)
            if delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }

            match llm_prompt(prompt.clone()).await {
                Ok(response) => {
                    return response;
                }
                Err(e) => {
                    warn!(
                        attempt = attempt + 1,
                        max_attempts = RETRY_DELAYS_MS.len(),
                        error = %e,
                        "LLM summary generation failed, will retry"
                    );
                    last_error = Some(e);
                }
            }
        }

        // All retries failed - use fallback summary
        if let Some(e) = last_error {
            warn!(
                error = %e,
                "All LLM summary retries failed, using fallback summary"
            );
        }

        FALLBACK_SUMMARY.to_string()
    }

    /// Build the LLM prompt for summarization
    fn build_summary_prompt(
        &self,
        summarized_turns: &[&ConversationTurn],
        anchors: &[AnchorPoint],
        kept_turns: &[ConversationTurn],
    ) -> String {
        // Extract preservation context from kept turns
        let preservation_context = PreservationContext::extract_from_turns(kept_turns);

        let mut prompt = String::from(
            r#"Summarize the following conversation turns concisely, preserving key information about what was accomplished.

CONTEXT TO PRESERVE:
"#,
        );

        // Add active files if any
        if !preservation_context.active_files.is_empty() {
            prompt.push_str(&format!(
                "Active files: {}\n",
                preservation_context.active_files.join(", ")
            ));
        }

        // Add current goals if any
        if !preservation_context.current_goals.is_empty() {
            prompt.push_str(&format!(
                "Current goals: {}\n",
                preservation_context.current_goals.join("; ")
            ));
        }

        // Add build status
        prompt.push_str(&format!("Build status: {}\n\n", preservation_context.build_status));

        prompt.push_str("TURNS TO SUMMARIZE:\n\n");

        for (idx, turn) in summarized_turns.iter().enumerate() {
            // Check if this turn is an anchor point
            let is_anchor = anchors.iter().any(|a| a.timestamp == turn.timestamp);
            let anchor_marker = if is_anchor { " [ANCHOR]" } else { "" };

            prompt.push_str(&format!("--- Turn {}{} ---\n", idx + 1, anchor_marker));
            prompt.push_str(&format!("User: {}\n", turn.user_message));

            if !turn.tool_calls.is_empty() {
                let tools: Vec<String> = turn.tool_calls.iter().map(|tc| {
                    let file_info = tc.filename().map(|f| format!(" on {f}")).unwrap_or_default();
                    format!("{}{file_info}", tc.tool)
                }).collect();
                prompt.push_str(&format!("Tools used: {}\n", tools.join(", ")));
            }

            if !turn.tool_results.is_empty() {
                let results: Vec<&str> = turn
                    .tool_results
                    .iter()
                    .map(|r| if r.success { "success" } else { "failed" })
                    .collect();
                prompt.push_str(&format!("Results: {}\n", results.join(", ")));
            }

            prompt.push_str(&format!("Assistant: {}\n\n", turn.assistant_response));
        }

        prompt.push_str(
            r#"INSTRUCTIONS:
1. Provide a concise summary (2-3 paragraphs) that captures:
   - What tasks were completed
   - What files were modified and why
   - Any errors that were resolved
   - The current state of the work
2. Preserve information about anchor points (marked with [ANCHOR])
3. Focus on outcomes and decisions, not the back-and-forth conversation
4. Be specific about file names, function names, and technical details"#,
        );

        prompt
    }
}

impl Default for ContextCompactor {
    fn default() -> Self {
        Self::new()
    }
}
