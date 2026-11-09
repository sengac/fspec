//! A session's stored thinking level as the level its turns use. Prompt
//! text never changes it: the level changes only through the thinking
//! dialog or `/thinking`.
//!
//! Feature: spec/features/prompt-text-never-changes-the-thinking-level.feature
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature

use crate::thinking_config::TurnLevel;
use codelet_rpc_types::ThinkingLevel;

/// A session's stored base level as the turn's level; `None` when no level
/// was chosen (ModelDefault), in which case no thinking parameter is sent.
/// MODEL-011: the level is the one chosen, unchanged: Off is each model's
/// own off form and every other level is sent as it is.
pub fn base_thinking_level(level: &ThinkingLevel) -> Result<Option<TurnLevel>, String> {
    codelet_sessions::thinking_options::turn_level(level)
}
