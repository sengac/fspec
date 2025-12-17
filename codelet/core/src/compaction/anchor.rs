//! Anchor point detection for context compaction
//!
//! Contains anchor types, anchor points, and detection logic.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use super::model::ConversationTurn;

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
