//! Unit tests for [`CopilotHttpClient`] (PROV-055).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use serde_json::json;

#[test]
fn classify_empty_body_returns_default() {
    let (classification, _) = classify_and_cache_body(bytes::Bytes::new());
    assert!(!classification.is_vision);
    assert!(!classification.is_agent);
}

#[test]
fn classify_text_only_body_is_neither() {
    let body = json!({
        "model": "gpt-4o",
        "messages": [{ "role": "user", "content": "hi" }]
    });
    let bytes = bytes::Bytes::from(serde_json::to_vec(&body).unwrap());
    let (c, _) = classify_and_cache_body(bytes);
    assert!(!c.is_vision);
    assert!(!c.is_agent);
}

#[test]
fn classify_vision_body_is_vision() {
    let body = json!({
        "model": "gpt-4o",
        "messages": [{
            "role": "user",
            "content": [
                { "type": "text", "text": "describe" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,xxx" } }
            ]
        }]
    });
    let bytes = bytes::Bytes::from(serde_json::to_vec(&body).unwrap());
    let (c, _) = classify_and_cache_body(bytes);
    assert!(c.is_vision);
}

#[test]
fn classify_agent_metadata_is_agent() {
    let body = json!({
        "model": "gpt-5",
        "messages": [{ "role": "user", "content": "run" }],
        "metadata": { "mode": "agent" }
    });
    let bytes = bytes::Bytes::from(serde_json::to_vec(&body).unwrap());
    let (c, _) = classify_and_cache_body(bytes);
    assert!(c.is_agent);
}

#[test]
fn classify_invalid_json_falls_back_to_default() {
    let (c, _) = classify_and_cache_body(bytes::Bytes::from_static(b"not json at all"));
    assert!(!c.is_vision);
    assert!(!c.is_agent);
}

#[test]
fn classify_and_cache_injects_cache_control_for_claude() {
    let body = json!({
        "model": "claude-sonnet-4",
        "messages": [
            { "role": "system", "content": "You are helpful." },
            { "role": "user", "content": "Hello" }
        ],
        "tools": [
            { "type": "function", "function": { "name": "read", "description": "Read" } }
        ]
    });
    let bytes = bytes::Bytes::from(serde_json::to_vec(&body).unwrap());
    let (_, new_body) = classify_and_cache_body(bytes);
    let result: serde_json::Value = serde_json::from_slice(&new_body).unwrap();
    assert_eq!(
        result["messages"][0].get("copilot_cache_control"),
        Some(&json!({ "type": "ephemeral" })),
        "Claude system message should get copilot_cache_control"
    );
    assert_eq!(
        result["tools"][0].get("copilot_cache_control"),
        Some(&json!({ "type": "ephemeral" })),
        "Last tool should get copilot_cache_control"
    );
}

#[test]
fn classify_and_cache_does_not_inject_for_gpt() {
    let body = json!({
        "model": "gpt-5",
        "messages": [
            { "role": "system", "content": "You are helpful." },
            { "role": "user", "content": "Hello" }
        ]
    });
    let original_bytes = serde_json::to_vec(&body).unwrap();
    let bytes = bytes::Bytes::from(original_bytes);
    let (_, new_body) = classify_and_cache_body(bytes);
    let result: serde_json::Value = serde_json::from_slice(&new_body).unwrap();
    assert!(
        result["messages"][0].get("copilot_cache_control").is_none(),
        "GPT system message should NOT get copilot_cache_control"
    );
}

#[test]
fn inject_headers_strips_stale_authorization() {
    let req = Request::builder()
        .uri("https://api.githubcopilot.com/chat/completions")
        .header(http::header::AUTHORIZATION, "Bearer stale_token")
        .body(bytes::Bytes::new())
        .unwrap();
    let classification = RequestClassification::default();
    let req = inject_copilot_headers(req, classification, "fresh_token");
    assert_eq!(
        req.headers().get(http::header::AUTHORIZATION).unwrap(),
        "Bearer fresh_token"
    );
}

#[test]
fn inject_headers_adds_all_required_copilot_headers_for_user_text() {
    let req = Request::builder()
        .uri("https://api.githubcopilot.com/chat/completions")
        .body(bytes::Bytes::new())
        .unwrap();
    let classification = RequestClassification::default();
    let req = inject_copilot_headers(req, classification, "ghu_tok");
    let headers = req.headers();
    assert_eq!(headers.get("x-initiator").unwrap(), "user");
    assert!(headers
        .get(http::header::USER_AGENT)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("codelet/"));
    assert_eq!(headers.get(http::header::AUTHORIZATION).unwrap(), "Bearer ghu_tok");
    assert_eq!(headers.get("openai-intent").unwrap(), "conversation-edits");
    assert!(headers.get("copilot-vision-request").is_none());
}

#[test]
fn inject_headers_adds_vision_header_when_classification_is_vision() {
    let req = Request::builder()
        .uri("https://api.githubcopilot.com/chat/completions")
        .body(bytes::Bytes::new())
        .unwrap();
    let classification = RequestClassification {
        is_vision: true,
        is_agent: false,
    };
    let req = inject_copilot_headers(req, classification, "ghu_tok");
    assert_eq!(req.headers().get("copilot-vision-request").unwrap(), "true");
}

#[test]
fn inject_headers_sets_agent_initiator_for_agent_classification() {
    let req = Request::builder()
        .uri("https://api.githubcopilot.com/chat/completions")
        .body(bytes::Bytes::new())
        .unwrap();
    let classification = RequestClassification {
        is_vision: false,
        is_agent: true,
    };
    let req = inject_copilot_headers(req, classification, "ghu_tok");
    assert_eq!(req.headers().get("x-initiator").unwrap(), "agent");
}

#[test]
fn default_client_has_empty_token() {
    let c = CopilotHttpClient::default();
    assert_eq!(c.access_token(), "");
}

#[test]
fn new_client_wraps_token() {
    let c = CopilotHttpClient::new("ghu_abc".to_string());
    assert_eq!(c.access_token(), "ghu_abc");
}
