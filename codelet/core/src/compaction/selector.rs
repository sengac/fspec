//! Turn selection for anchor-based compaction
//!
//! Contains turn selection logic and related types.

use anyhow::Result;

use super::anchor::AnchorPoint;
use super::model::ConversationTurn;

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

    /// Select turns using TypeScript-matching logic
    ///
    /// Matches TypeScript selectTurnsForCompaction() exactly:
    /// 1. ALWAYS split into recentTurns (last 2-3) and olderTurns
    /// 2. Only look for anchors in olderTurns
    /// 3. If anchor found in olderTurns: keep anchor→end of older + all recent
    /// 4. If no anchor in olderTurns: keep only recent turns
    pub fn select_turns_with_recent(
        &self,
        turns: &[ConversationTurn],
        anchors: &[AnchorPoint],
    ) -> Result<TurnSelection> {
        if turns.is_empty() {
            return Ok(TurnSelection {
                kept_turns: Vec::new(),
                summarized_turns: Vec::new(),
                preserved_anchor: None,
            });
        }

        let total_turns = turns.len();

        // ALWAYS preserve last 2-3 complete conversation turns (matches TypeScript)
        let turns_to_always_keep = 3.min(total_turns);
        let older_turns_count = total_turns.saturating_sub(turns_to_always_keep);

        // Find most recent anchor point in older turns only (matches TypeScript)
        // TypeScript: .filter(anchor => anchor.turnIndex < totalTurns - turnsToAlwaysKeep)
        let anchor_in_older = anchors
            .iter()
            .filter(|a| a.turn_index < older_turns_count)
            .max_by_key(|a| a.turn_index); // Get most recent (highest index)

        if let Some(anchor) = anchor_in_older {
            // Keep anchor + everything after it + recent turns
            // TypeScript: [...olderTurns.slice(anchorPointInOlderTurns.turnIndex), ...recentTurns]
            let mut kept_turns: Vec<TurnInfo> = Vec::new();

            // Add from anchor to end of older turns
            for idx in anchor.turn_index..older_turns_count {
                kept_turns.push(TurnInfo { turn_index: idx });
            }

            // Add all recent turns
            for idx in older_turns_count..total_turns {
                kept_turns.push(TurnInfo { turn_index: idx });
            }

            // Summarize turns before anchor point
            let summarized_turns: Vec<TurnInfo> = (0..anchor.turn_index)
                .map(|idx| TurnInfo { turn_index: idx })
                .collect();

            Ok(TurnSelection {
                kept_turns,
                summarized_turns,
                preserved_anchor: Some(anchor.clone()),
            })
        } else {
            // No anchor found in older turns - keep only recent turns, summarize the rest
            let kept_turns: Vec<TurnInfo> = (older_turns_count..total_turns)
                .map(|idx| TurnInfo { turn_index: idx })
                .collect();

            let summarized_turns: Vec<TurnInfo> = (0..older_turns_count)
                .map(|idx| TurnInfo { turn_index: idx })
                .collect();

            Ok(TurnSelection {
                kept_turns,
                summarized_turns,
                preserved_anchor: None,
            })
        }
    }

    /// Legacy method - kept for backwards compatibility
    #[allow(dead_code)]
    pub fn select_turns(
        &self,
        turns: &[ConversationTurn],
        anchor: Option<&AnchorPoint>,
    ) -> Result<TurnSelection> {
        let anchors: Vec<AnchorPoint> = anchor.cloned().into_iter().collect();
        self.select_turns_with_recent(turns, &anchors)
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
    /// Turns to keep (from anchor forward + recent turns)
    pub kept_turns: Vec<TurnInfo>,
    /// Turns to summarize (before anchor)
    pub summarized_turns: Vec<TurnInfo>,
    /// Preserved anchor point (if any found in older turns)
    pub preserved_anchor: Option<AnchorPoint>,
}

/// Information about a turn in selection result
#[derive(Debug)]
pub struct TurnInfo {
    /// Index of turn in conversation history
    pub turn_index: usize,
}
