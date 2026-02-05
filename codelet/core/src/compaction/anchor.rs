//! Anchor point detection for context compaction
//!
//! Contains anchor types, anchor points, and detection logic.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tracing::warn;

use super::model::ConversationTurn;

// ==========================================
// LLM RESPONSE PARSING
// ==========================================

/// LLM response structure for anchor detection
#[derive(Debug, Deserialize)]
struct LlmAnchorResponse {
    anchor_type: Option<String>,
    confidence: f64,
    description: String,
}

// Constants for timeout and JSON parsing
const LLM_TIMEOUT_SECS: u64 = 15;

/// Extract JSON content from an LLM response
/// 
/// LLMs often wrap JSON in markdown code blocks like:
/// ```json
/// [...]
/// ```
/// 
/// This function extracts the actual JSON content.
fn extract_json_from_response(response: &str) -> &str {
    let trimmed = response.trim();
    
    // Check for markdown JSON code block
    if trimmed.starts_with("```json") {
        // Find the end of the opening line
        if let Some(start) = trimmed.find('\n') {
            let after_open = &trimmed[start + 1..];
            // Find the closing ```
            if let Some(end) = after_open.rfind("```") {
                return after_open[..end].trim();
            }
        }
    }
    
    // Check for generic code block
    if trimmed.starts_with("```") {
        if let Some(start) = trimmed.find('\n') {
            let after_open = &trimmed[start + 1..];
            if let Some(end) = after_open.rfind("```") {
                return after_open[..end].trim();
            }
        }
    }
    
    // Try to find JSON array or object boundaries
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            if end > start {
                return &trimmed[start..=end];
            }
        }
    }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return &trimmed[start..=end];
            }
        }
    }
    
    // Return original if no extraction possible
    trimmed
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

impl AnchorPoint {
    /// Create synthetic anchor for timeout/failure scenarios (CTX-004 requirement)
    pub fn synthetic_checkpoint(
        turn_index: usize,
        turn: &ConversationTurn, 
        reason: &str
    ) -> Self {
        AnchorPoint {
            turn_index,
            anchor_type: AnchorType::UserCheckpoint,
            weight: 1.0, // Highest priority for reliability
            confidence: 1.0, // Synthetic anchors have full confidence
            description: format!("Synthetic anchor - {reason}"),
            timestamp: turn.timestamp,
        }
    }
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

    /// Detect anchor points in conversation turns using LLM analysis
    ///
    /// Returns Some(AnchorPoint) if confidence >= threshold, None otherwise
    pub async fn detect<F, Fut>(
        &self,
        turn: &ConversationTurn,
        turn_index: usize,
        llm_prompt: &F,
    ) -> Result<Option<AnchorPoint>>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        use tokio::time::{timeout, Duration};

        // Build LLM prompt for anchor analysis
        let prompt = self.build_anchor_analysis_prompt(turn);
        
        // Call LLM with 15-second timeout
        let llm_result = timeout(Duration::from_secs(LLM_TIMEOUT_SECS), llm_prompt(prompt)).await;
        
        match llm_result {
            Ok(Ok(response)) => {
                // Parse LLM response for anchor detection
                self.parse_llm_anchor_response(&response, turn_index, turn)
            }
            Ok(Err(_)) | Err(_) => {
                // LLM analysis failed or timed out - create synthetic anchor as fallback
                // This is REQUIRED by CTX-004 feature file: "Then the system creates a synthetic anchor as fallback"
                Ok(Some(AnchorPoint::synthetic_checkpoint(
                    turn_index, 
                    turn, 
                    "LLM analysis failed/timeout"
                )))
            }
        }
    }

    /// PERF-002: Batch detect anchor points in multiple turns using single LLM call
    ///
    /// This replaces sequential per-turn detection with batched processing to improve performance.
    /// Instead of N LLM calls, this makes 1 LLM call to analyze all turns together.
    pub async fn detect_batch<F, Fut>(
        &self,
        turns: &[ConversationTurn],
        llm_prompt: &F,
    ) -> Result<Vec<AnchorPoint>>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        use tokio::time::{timeout, Duration};

        if turns.is_empty() {
            return Ok(Vec::new());
        }

        // Build batch LLM prompt for analyzing all turns together
        let prompt = self.build_batch_anchor_analysis_prompt(turns);
        
        // Call LLM with extended timeout (more turns to analyze)
        let timeout_secs = LLM_TIMEOUT_SECS + (turns.len() as u64 * 2); // Base + 2 secs per turn
        let llm_result = timeout(Duration::from_secs(timeout_secs), llm_prompt(prompt)).await;
        
        match llm_result {
            Ok(Ok(response)) => {
                // Parse batch LLM response for anchor detection
                self.parse_batch_llm_anchor_response(&response, turns)
            }
            Ok(Err(e)) => {
                // LLM analysis failed - log the actual error
                warn!("LLM anchor detection failed: {}", e);
                let last_idx = turns.len() - 1;
                let last_turn = &turns[last_idx];
                Ok(vec![AnchorPoint::synthetic_checkpoint(
                    last_idx, 
                    last_turn, 
                    &format!("LLM analysis failed: {}", e)
                )])
            }
            Err(_) => {
                // Timeout - create synthetic anchor as fallback
                warn!("LLM anchor detection timed out after {} seconds", 
                    LLM_TIMEOUT_SECS + (turns.len() as u64 * 2));
                let last_idx = turns.len() - 1;
                let last_turn = &turns[last_idx];
                Ok(vec![AnchorPoint::synthetic_checkpoint(
                    last_idx, 
                    last_turn, 
                    "LLM analysis timed out"
                )])
            }
        }
    }

    /// Build structured prompt for LLM anchor analysis
    fn build_anchor_analysis_prompt(&self, turn: &ConversationTurn) -> String {
        format!(
            r#"Analyze this conversation turn to detect meaningful anchor points for context preservation.

TURN CONTENT:
Assistant Response: {}
Tool Calls: {:?}
Tool Results: {:?}
Previous Error State: {:?}

ANCHOR TYPES (select the most appropriate):
- ErrorResolution: Error was resolved (previous error + fix + success)
- TaskCompletion: Task was completed (modify + test + success, no previous error)  
- UserCheckpoint: User created explicit checkpoint or significant milestone
- FeatureMilestone: Feature milestone reached

ANALYSIS INSTRUCTIONS:
1. Determine if this turn represents a meaningful moment worth preserving
2. If meaningful, classify into one of the 4 anchor types above
3. Assess confidence level (0.0-1.0) - use {} as minimum threshold
4. Provide brief description of why this moment is significant

RESPONSE FORMAT (JSON):
{{"anchor_type": "TaskCompletion", "confidence": 0.92, "description": "Brief explanation"}}

If no meaningful anchor: {{"anchor_type": null, "confidence": 0.0, "description": "No significant moment detected"}}"#,
            turn.assistant_response,
            turn.tool_calls,
            turn.tool_results,
            turn.previous_error,
            self.confidence_threshold
        )
    }

    /// Parse LLM response into anchor point using secure JSON parsing
    fn parse_llm_anchor_response(
        &self, 
        response: &str, 
        turn_index: usize, 
        turn: &ConversationTurn
    ) -> Result<Option<AnchorPoint>> {
        // Extract JSON from LLM response (handles markdown code blocks)
        let json_content = extract_json_from_response(response);
        
        // Use serde for secure JSON parsing instead of manual string matching
        let parsed_response: LlmAnchorResponse = match serde_json::from_str(json_content) {
            Ok(response) => response,
            Err(e) => {
                // Log the JSON parsing error with details for debugging
                warn!(
                    "Failed to parse LLM anchor response as JSON: {}. Response preview: {}",
                    e,
                    &response.chars().take(200).collect::<String>()
                );
                // If JSON parsing fails, treat as no anchor detected
                return Ok(None);
            }
        };

        // Check if anchor type is null (no meaningful anchor detected)
        let anchor_type_str = match parsed_response.anchor_type {
            Some(anchor_type) => anchor_type,
            None => return Ok(None),
        };

        // Map string to AnchorType enum
        let anchor_type = match anchor_type_str.as_str() {
            "ErrorResolution" => AnchorType::ErrorResolution,
            "TaskCompletion" => AnchorType::TaskCompletion,
            "FeatureMilestone" => AnchorType::FeatureMilestone,
            "UserCheckpoint" => AnchorType::UserCheckpoint,
            _ => return Ok(None), // Unknown anchor type
        };

        // Check confidence threshold
        if parsed_response.confidence < self.confidence_threshold {
            return Ok(None);
        }

        Ok(Some(AnchorPoint {
            turn_index,
            anchor_type,
            weight: anchor_type.weight(),
            confidence: parsed_response.confidence,
            description: parsed_response.description,
            timestamp: turn.timestamp,
        }))
    }

    /// PERF-002: Build batch LLM prompt for analyzing multiple turns simultaneously
    fn build_batch_anchor_analysis_prompt(&self, turns: &[ConversationTurn]) -> String {
        let mut prompt = format!(
            r#"Analyze these conversation turns to detect meaningful anchor points for context preservation.

ANCHOR TYPES (select the most appropriate for each turn):
- ErrorResolution: Error was resolved (previous error + fix + success)
- TaskCompletion: Task was completed (modify + test + success, no previous error)  
- UserCheckpoint: User created explicit checkpoint or significant milestone
- FeatureMilestone: Major feature or capability achieved

ANALYSIS CRITERIA:
1. Look for task completion, error resolution, or significant milestones
2. Weight: ErrorResolution (0.9) > FeatureMilestone (0.75) > TaskCompletion (0.8) > UserCheckpoint (0.7)
3. Assess confidence level (0.0-1.0) - use {} as minimum threshold
4. Provide brief description of why each moment is significant

TURNS TO ANALYZE:
"#,
            self.confidence_threshold
        );

        for (idx, turn) in turns.iter().enumerate() {
            prompt.push_str(&format!(
                r#"
TURN {}:
Assistant Response: {}
Tool Calls: {:?}
Tool Results: {:?}
Previous Error State: {:?}

"#,
                idx,
                turn.assistant_response,
                turn.tool_calls,
                turn.tool_results,
                turn.previous_error
            ));
        }

        prompt.push_str(
            r#"
RESPONSE FORMAT (JSON array):
[
  {"turn_index": 0, "anchor_type": "TaskCompletion", "confidence": 0.92, "description": "Brief explanation"},
  {"turn_index": 2, "anchor_type": null, "confidence": 0.0, "description": "No significant moment detected"},
  ...
]

Return one entry per turn analyzed. Use null for anchor_type when no meaningful anchor is detected."#
        );

        prompt
    }

    /// PERF-002: Parse batch LLM response into multiple anchor points
    fn parse_batch_llm_anchor_response(
        &self, 
        response: &str, 
        turns: &[ConversationTurn]
    ) -> Result<Vec<AnchorPoint>> {
        // Parse response as JSON array of LlmAnchorResponse
        #[derive(Debug, Deserialize)]
        struct BatchLlmAnchorResponse {
            turn_index: usize,
            anchor_type: Option<String>,
            confidence: f64,
            description: String,
        }

        // Extract JSON from LLM response (handles markdown code blocks)
        let json_content = extract_json_from_response(response);
        
        let parsed_responses: Vec<BatchLlmAnchorResponse> = match serde_json::from_str(json_content) {
            Ok(responses) => responses,
            Err(e) => {
                // Log the JSON parsing error with details for debugging
                warn!(
                    "Failed to parse LLM anchor response as JSON: {}. Response preview: {}",
                    e,
                    &response.chars().take(200).collect::<String>()
                );
                // Return empty vec (no anchors detected)
                return Ok(Vec::new());
            }
        };

        let mut anchors = Vec::new();

        for batch_response in parsed_responses {
            // Validate turn index
            if batch_response.turn_index >= turns.len() {
                continue; // Skip invalid turn indices
            }

            // Check if anchor type is null (no meaningful anchor detected)
            let anchor_type_str = match batch_response.anchor_type {
                Some(anchor_type) => anchor_type,
                None => continue, // Skip null anchor types
            };

            // Map string to AnchorType enum
            let anchor_type = match anchor_type_str.as_str() {
                "ErrorResolution" => AnchorType::ErrorResolution,
                "TaskCompletion" => AnchorType::TaskCompletion,
                "FeatureMilestone" => AnchorType::FeatureMilestone,
                "UserCheckpoint" => AnchorType::UserCheckpoint,
                _ => continue, // Skip unknown anchor types
            };

            // Check confidence threshold
            if batch_response.confidence < self.confidence_threshold {
                continue; // Skip low confidence anchors
            }

            let turn = &turns[batch_response.turn_index];

            // All checks passed - create anchor point
            anchors.push(AnchorPoint {
                turn_index: batch_response.turn_index,
                anchor_type,
                weight: anchor_type.weight(),
                confidence: batch_response.confidence,
                description: batch_response.description,
                timestamp: turn.timestamp,
            });
        }

        Ok(anchors)
    }
}
