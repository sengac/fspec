use crate::dispatch::InputWithImages;
use crate::thinking_config::{get_thinking_config, JsThinkingLevel};
use crate::thinking_level_detection::{
    compute_effective_thinking_level, detect_thinking_level, has_disable_keywords,
    thinking_level_from_u8,
};
use codelet_sessions::background_session::BackgroundSession;
use codelet_tools::facade::is_adaptive_thinking_model;

pub(super) fn resolve_turn_thinking(
    session: &BackgroundSession,
    input_with_images: &InputWithImages,
    current_provider: &str,
    current_model: Option<&str>,
    thinking_model_key: Option<&str>,
) -> Option<serde_json::Value> {
    let input = &input_with_images.text;
    let routing_model = thinking_model_key.or(current_model);
    let is_adaptive_model = routing_model
        .map(is_adaptive_thinking_model)
        .unwrap_or(false);
    tracing::debug!(
        "[AGENT-LOOP] thinking routing: routing_model={:?}, is_adaptive={}, base_thinking_level={}, has_ts_config={}",
        routing_model, is_adaptive_model, session.get_base_thinking_level(),
        input_with_images.thinking_config.is_some(),
    );
    if is_adaptive_model {
        let detected_level = detect_thinking_level(input);
        let force_off = has_disable_keywords(input);
        let base_level = thinking_level_from_u8(session.get_base_thinking_level());
        let effective_level =
            compute_effective_thinking_level(base_level, detected_level, force_off);
        tracing::debug!(
            "[AGENT-LOOP] adaptive path: base={:?}, detected={:?}, force_off={}, effective={:?}",
            base_level,
            detected_level,
            force_off,
            effective_level
        );
        if effective_level == JsThinkingLevel::Off {
            None
        } else {
            let config_key =
                routing_model.expect("routing_model must be Some when is_adaptive_model is true");
            match get_thinking_config(config_key.to_string(), effective_level) {
                Ok(config_str) => {
                    tracing::info!(
                        "[AGENT-LOOP] adaptive thinking selected: config_key={}, effective_level={:?}, base={:?}, detected={:?}, force_off={}, config_str={}",
                        config_key, effective_level, base_level, detected_level, force_off, config_str
                    );
                    serde_json::from_str(&config_str).ok()
                }
                Err(error) => {
                    tracing::warn!(
                        "Failed to get thinking config for adaptive model: {}",
                        error
                    );
                    None
                }
            }
        }
    } else if let Some(config_str) = input_with_images.thinking_config.as_deref() {
        serde_json::from_str(config_str).ok()
    } else {
        let detected_level = detect_thinking_level(input);
        let force_off = has_disable_keywords(input);
        let base_level = thinking_level_from_u8(session.get_base_thinking_level());
        let effective_level =
            compute_effective_thinking_level(base_level, detected_level, force_off);
        if effective_level == JsThinkingLevel::Off {
            None
        } else {
            let config_key = routing_model.unwrap_or(current_provider);
            match get_thinking_config(config_key.to_string(), effective_level) {
                Ok(config_str) => {
                    tracing::info!("Thinking level detected: {:?} (base={:?}, detected={:?}, force_off={}, config_key={})",
                        effective_level, base_level, detected_level, force_off, config_key);
                    serde_json::from_str(&config_str).ok()
                }
                Err(error) => {
                    tracing::warn!("Failed to get thinking config: {}", error);
                    None
                }
            }
        }
    }
}
