use super::{safe, RateLimits, RateWindow, State};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn apply(state: &mut State, event: &Value) {
    apply_headers(state, event);
    if !state.http_response {
        apply_header_advice(state, &event["headers"]);
    }
    if event["type"] == "codex.rate_limits" {
        let primary = &event["rate_limits"]["primary"];
        if let (Some(name), Some(percent)) = (
            event["metered_limit_name"].as_str().and_then(safe),
            primary["used_percent"].as_f64(),
        ) {
            state.status.rate_limits.insert(
                name,
                RateLimits {
                    primary: Some(RateWindow {
                        used_percent: percent,
                        window_minutes: primary["window_minutes"].as_u64(),
                        reset_at: primary["reset_at"].as_u64(),
                    }),
                },
            );
        }
    }
    if event["type"] == "response.metadata" {
        let metadata = &event["metadata"];
        if metadata["openai_verification_recommendation"]
            .as_array()
            .is_some_and(|names| names.iter().any(|name| name == "trusted_access_for_cyber"))
        {
            state.status.verification_required = true;
        }
        if metadata
            .get("openai_chatgpt_moderation_metadata")
            .is_some_and(|value| !value.is_null())
        {
            state.status.moderation_received = true;
        }
    }
    if let Some(payload) = buffering(event) {
        state.status.faster_model = match payload.get("retry_model") {
            None => state.header_advice.clone(),
            Some(value) => value.as_str().and_then(safe),
        };
        if !state.output_started {
            state.status.buffering = true;
            state.waiting = true;
        }
    }
    let assistant = assistant_output(event);
    state.output_started |= assistant;
    if assistant
        || matches!(
            event["type"].as_str(),
            Some("response.completed" | "response.failed" | "response.incomplete" | "error")
        )
    {
        state.waiting = false;
        state.status.buffering = false;
    }
}

fn buffering(event: &Value) -> Option<&serde_json::Map<String, Value>> {
    let payload = match event.get("safety_buffering") {
        Some(value) => value,
        None if event["type"] == "response.metadata"
            && event["metadata"]["type"] == "safety_buffering" =>
        {
            &event["metadata"]
        }
        None => return None,
    };
    let object = payload.as_object()?;
    for field in ["use_cases", "reasons"] {
        if !object.get(field)?.as_array()?.iter().all(Value::is_string) {
            return None;
        }
    }
    if object
        .get("retry_model")
        .is_some_and(|value| !value.is_null() && !value.is_string())
    {
        return None;
    }
    Some(object)
}

fn assistant_output(event: &Value) -> bool {
    match event["type"].as_str() {
        Some(
            "response.output_text.delta"
            | "response.output_text.done"
            | "response.refusal.delta"
            | "response.refusal.done",
        ) => true,
        Some("response.output_item.added" | "response.output_item.done") => {
            event["item"]["type"] == "message" && event["item"]["role"] == "assistant"
        }
        Some("response.content_part.added" | "response.content_part.done") => {
            matches!(
                event["part"]["type"].as_str(),
                Some("output_text" | "refusal")
            )
        }
        Some("response.completed" | "response.failed" | "response.incomplete") => event["response"]
            ["output"]
            .as_array()
            .is_some_and(|output| {
                output
                    .iter()
                    .any(|item| item["type"] == "message" && item["role"] == "assistant")
            }),
        _ => false,
    }
}

fn header_values(sources: &[&Value]) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    for source in sources {
        if let Some(source) = source.as_object() {
            for (name, value) in source {
                if let Some(value) = value
                    .as_str()
                    .or_else(|| value.as_array()?.first()?.as_str())
                {
                    headers.insert(name.to_ascii_lowercase(), value.to_owned());
                }
            }
        }
    }
    headers
}

pub(super) fn apply_header_advice(state: &mut State, source: &Value) {
    let headers = header_values(&[source]);
    if headers.contains_key("x-codex-safety-buffering-enabled")
        || headers.contains_key("x-codex-safety-buffering-faster-model")
    {
        state.header_advice = headers
            .get("x-codex-safety-buffering-faster-model")
            .and_then(|value| safe(value));
    }
}

pub(super) fn apply_headers(state: &mut State, event: &Value) {
    let headers = header_values(&[&event["headers"], &event["response"]["headers"]]);
    for (name, value) in &headers {
        match name.as_str() {
            "x-request-id" => state.status.request_id = safe(value),
            "openai-model" | "x-openai-model" => state.status.server_model = safe(value),
            "x-reasoning-included" => state.status.reasoning_included = true,
            "x-models-etag" => state.etag = Some(value.clone()),
            "x-codex-turn-state" => state.turn_state = Some(value.clone()),
            _ => {}
        }
    }
    if let Some(percent) = headers
        .get("x-codex-primary-used-percent")
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
    {
        state.status.rate_limits.insert(
            "codex".into(),
            RateLimits {
                primary: Some(RateWindow {
                    used_percent: percent,
                    window_minutes: headers
                        .get("x-codex-primary-window-minutes")
                        .and_then(|value| value.parse().ok()),
                    reset_at: headers
                        .get("x-codex-primary-reset-at")
                        .and_then(|value| value.parse().ok()),
                }),
            },
        );
    }
}
