//! Compaction metrics and result types
//!
//! Contains metrics from compaction operations and result structures.

use serde::{Deserialize, Serialize};

use super::anchor::AnchorPoint;
use super::model::ConversationTurn;

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
// COMPACTION RESULT
// ==========================================

/// Result of a compaction operation
#[derive(Debug, Clone)]
pub struct CompactionResult {
    /// Turns that were kept (from anchor point forward)
    pub kept_turns: Vec<ConversationTurn>,
    /// Warnings generated during compaction (matches TypeScript behavior)
    pub warnings: Vec<String>,
    /// LLM-generated summary of summarized turns
    pub summary: String,
    /// Compaction metrics
    pub metrics: CompactionMetrics,
    /// Anchor point used (if any)
    pub anchor: Option<AnchorPoint>,
}
