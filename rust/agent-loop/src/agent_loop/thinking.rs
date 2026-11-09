use crate::thinking_config::ThinkingTarget;
use crate::thinking_level_detection::base_thinking_level;
use codelet_sessions::background_session::BackgroundSession;

/// The thinking parameters a turn sends: the session's chosen level in the
/// dispatched provider's form (MODEL-010 wire table). The turn's prompt text
/// is not an input: it never changes the level. A per-turn raw
/// `thinking_config` is refused: levels are chosen with the thinking dialog
/// or `/thinking`, and nothing is sent raw or silently dropped.
pub fn resolve_turn_thinking(
    session: &BackgroundSession,
    thinking_config: Option<&str>,
    target: &ThinkingTarget,
) -> Result<Option<serde_json::Value>, String> {
    if thinking_config.is_some() {
        return Err(
            "a per-turn thinking config is not sent: choose the thinking level with the \
             thinking dialog or /thinking"
                .to_string(),
        );
    }
    // `None` = no level chosen; no thinking parameter is sent on any provider.
    let level = base_thinking_level(&session.get_base_thinking_level())?;
    let config = target.config_for(level.as_ref())?;
    tracing::info!(
        "[AGENT-LOOP] thinking selected: wire={:?}, level={:?}, config={:?}",
        target.wire(),
        level,
        config
    );
    Ok(config)
}
