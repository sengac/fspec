//! Feature: spec/features/codex-buffering-wire-compatibility.feature

use codelet_providers::codex::metadata::{CodexMetadataHandle, CodexPresentationEvent};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[path = "fixtures/bug241_protocol.rs"]
mod fixture;
use fixture::TestResult;

fn buffering() -> Value {
    json!({"use_cases":["private-case"],"reasons":["private-reason"]})
}

fn metadata(payload: Value) -> Value {
    json!({"type":"response.metadata","safety_buffering":payload})
}

#[tokio::test]
async fn ignore_invalid_buffering_payloads() -> TestResult {
    // @step Given Codex streams containing boolean null and malformed buffering payloads
    let invalid = vec![
        json!(true),
        json!(false),
        Value::Null,
        json!({}),
        json!({"use_cases":[]}),
        json!({"reasons":[]}),
        json!({"use_cases":"private-case","reasons":[]}),
        json!({"use_cases":[],"reasons":[42]}),
        json!({"use_cases":[],"reasons":[],"retry_model":42}),
    ];
    let mut events = Vec::new();
    for payload in invalid {
        let handle = CodexMetadataHandle::default();
        // @step When those payloads arrive between valid buffering observations
        handle.apply_event(&metadata(payload.clone()));
        // @step Then only valid objects affect buffering status
        assert!(!handle.status().buffering, "invalid payload: {payload}");
        handle.apply_event(&metadata(buffering()));
        assert!(handle.status().buffering);
        handle.apply_event(&metadata(payload.clone()));
        assert!(
            handle.status().buffering,
            "invalid payload cleared state: {payload}"
        );
        events.extend([metadata(payload), metadata(buffering())]);
    }
    proptest::proptest!(|(invalid_reason in proptest::num::i64::ANY)| {
        let handle = CodexMetadataHandle::default();
        handle.apply_event(&metadata(json!({"use_cases":[],"reasons":[invalid_reason]})));
        proptest::prop_assert!(!handle.status().buffering);
    });
    // @step And normal text and usage are delivered unchanged
    for websocket in [false, true] {
        fixture::consume(
            events.clone(),
            websocket,
            &[],
            CodexMetadataHandle::default(),
        )
        .await?;
    }
    Ok(())
}

#[tokio::test]
async fn respect_buffering_source_precedence() -> TestResult {
    // @step Given conflicting top level and nested Codex buffering metadata
    let nested = json!({"type":"safety_buffering","use_cases":["private-case"],"reasons":["private-reason"],"retry_model":"nested-model"});
    let mut events = Vec::new();
    for payload in [
        Value::Null,
        json!(true),
        json!(false),
        json!({}),
        json!("bad"),
    ] {
        let event =
            json!({"type":"response.metadata","safety_buffering":payload,"metadata":nested});
        let handle = CodexMetadataHandle::default();
        handle.apply_event(&event);
        // @step Then top level presence wins and nested fallback requires a metadata event
        assert!(
            !handle.status().buffering,
            "top-level presence must win: {event}"
        );
        events.push(event);
    }
    // @step When buffering metadata arrives on different event types
    for kind in ["response.created", "response.future", "response.metadata"] {
        let event = json!({"type":kind,"metadata":nested});
        let handle = CodexMetadataHandle::default();
        handle.apply_event(&event);
        assert_eq!(handle.status().buffering, kind == "response.metadata");
        // @step And private buffering reasons never enter diagnostics
        assert!(!format!("{handle:?} {:?}", handle.status()).contains("private-"));
        events.push(event);
    }
    let mut top = buffering();
    top["retry_model"] = json!("top-model");
    let event = json!({"type":"response.future","safety_buffering":top,"metadata":nested});
    let handle = CodexMetadataHandle::default();
    handle.apply_event(&event);
    assert!(handle.status().buffering);
    assert_eq!(handle.status().faster_model.as_deref(), Some("top-model"));
    events.push(event);
    for websocket in [false, true] {
        fixture::consume(
            events.clone(),
            websocket,
            &[],
            CodexMetadataHandle::default(),
        )
        .await?;
    }
    Ok(())
}

#[tokio::test]
async fn resolve_faster_model_advice_per_event() -> TestResult {
    // @step Given header advice and successive top level and nested buffering objects
    let mut events = Vec::new();
    let handle = CodexMetadataHandle::default();
    handle
        .apply_event(&json!({"headers":{"x-codex-safety-buffering-faster-model":"header-model"}}));
    for nested in [false, true] {
        for (retry, expected) in [
            (Some(json!("explicit-model")), Some("explicit-model")),
            (None, Some("header-model")),
            (Some(Value::Null), None),
            (None, Some("header-model")),
            (Some(json!("bad\u{001b}model")), None),
        ] {
            let mut payload = buffering();
            if let Some(retry) = retry {
                payload["retry_model"] = retry;
            }
            let event = if nested {
                payload["type"] = json!("safety_buffering");
                json!({"type":"response.metadata","metadata":payload})
            } else {
                metadata(payload)
            };
            // @step When retry model values are omitted null or explicit
            handle.apply_event(&event);
            // @step Then each event resolves its own advice without stale suggestions
            assert_eq!(handle.status().faster_model.as_deref(), expected, "{event}");
            events.push(event);
        }
    }
    for websocket in [false, true] {
        let handle = CodexMetadataHandle::default();
        let advice = Arc::new(Mutex::new(Vec::new()));
        let observed = advice.clone();
        handle.set_observer(Arc::new(move |status| {
            if let Ok(mut advice) = observed.lock() {
                if status.buffering {
                    advice.push(status.faster_model);
                }
            }
        }));
        // @step And the selected model remains unchanged
        let mut transport_events = events.clone();
        if websocket {
            transport_events.insert(
                0,
                json!({"type":"response.metadata","headers":{
                    "x-codex-safety-buffering-faster-model":"header-model"
                }}),
            );
        }
        fixture::consume(
            transport_events,
            websocket,
            &[("x-codex-safety-buffering-faster-model", "header-model")],
            handle,
        )
        .await?;
        let advice = advice.lock().map_err(|_| "advice observer poisoned")?;
        assert_eq!(advice.len(), 10);
        for block in advice.chunks(5) {
            assert_eq!(
                block,
                [
                    Some("explicit-model".into()),
                    Some("header-model".into()),
                    None,
                    Some("header-model".into()),
                    None
                ]
            );
        }
    }
    handle.begin_turn();
    let first = handle.scoped_stream();
    first.begin_response();
    first.apply_event(&json!({"headers":{"x-codex-safety-buffering-faster-model":"first-model"}}));
    first.apply_event(&metadata(buffering()));
    assert_eq!(handle.status().faster_model.as_deref(), Some("first-model"));
    first.apply_event(&json!({"type":"response.completed","response":{"output":[{"type":"function_call","name":"inspect"}]}}));
    assert!(!handle.status().buffering);
    let following = handle.scoped_stream();
    following.begin_response();
    following.apply_event(&metadata(buffering()));
    assert!(
        handle.status().buffering,
        "tool-only completion must not suppress the next response"
    );
    assert_eq!(handle.status().faster_model, None);
    fixture::tool_only_completion(handle.clone()).await?;
    let following = handle.scoped_stream();
    following.begin_response();
    following.apply_event(&metadata(buffering()));
    assert!(
        handle.status().buffering,
        "tool-only stream cleanup must not suppress the next response"
    );
    following.apply_event(&json!({"type":"response.completed","response":{"output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Hello"}]}]}}));
    let after_assistant = handle.scoped_stream();
    after_assistant.begin_response();
    after_assistant.apply_event(&metadata(buffering()));
    assert!(
        !handle.status().buffering,
        "assistant output must still suppress later responses"
    );
    fixture::retry_attempts_and_treatments(metadata(buffering()))?;
    fixture::transport_header_advice(metadata(buffering())).await?;
    Ok(())
}

#[tokio::test]
async fn scope_optional_metadata_to_metadata_events() -> TestResult {
    // @step Given moderation and verification metadata on recognized and unrelated events
    let optional = json!({"openai_verification_recommendation":["trusted_access_for_cyber","private-name"],"openai_chatgpt_moderation_metadata":{"reason":"private-moderation"}});
    for websocket in [false, true] {
        for kind in ["response.created", "response.future", "response.metadata"] {
            let handle = CodexMetadataHandle::default();
            // @step When Codex consumes those events over HTTP and WebSocket
            fixture::consume(
                vec![json!({"type":kind,"metadata":optional})],
                websocket,
                &[],
                handle.clone(),
            )
            .await?;
            let status = handle.status();
            // @step Then only recognized metadata events update optional status
            assert_eq!(status.moderation_received, kind == "response.metadata");
            assert_eq!(status.verification_required, kind == "response.metadata");
            let rendered = format!("{handle:?} {status:?} {}", serde_json::to_string(&status)?);
            // @step And private metadata is not exposed
            assert!(!rendered.contains("private-"));
        }
    }
    let handle = CodexMetadataHandle::default();
    let resetter = handle.clone();
    handle.set_observer(Arc::new(move |status| {
        if status.buffering {
            resetter.reset();
        }
    }));
    let presented = Arc::new(Mutex::new(Vec::new()));
    let recorded = presented.clone();
    handle.set_presentation_observer(Arc::new(move |event| {
        if let Ok(mut recorded) = recorded.lock() {
            recorded.push(event);
        }
    }));
    handle.apply_event(
        &json!({"type":"response.metadata","safety_buffering":buffering(),"metadata":optional}),
    );
    assert!(!handle.status().buffering);
    assert_eq!(
        *presented.lock().map_err(|_| "presentation lock poisoned")?,
        vec![CodexPresentationEvent::Waiting(false)]
    );
    handle.set_observer(Arc::new(|_| {}));
    Ok(())
}
