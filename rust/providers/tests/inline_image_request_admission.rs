//! Feature: spec/features/inline-image-request-admission.feature

use codelet_providers::codex::{history::CodexHistory, CodexResponsesModel};
use codelet_providers::request_admission::{AdmissionLimits, RequestAdmissionError};
use codelet_providers::selected_request::{media_occupancy, SelectedRequestPreparer};
use futures::StreamExt;
use proptest::prelude::*;
use rig::completion::{CompletionModel, CompletionRequestBuilder, ToolDefinition};
use rig::message::{AssistantContent, ImageMediaType, Message, ToolResultContent, UserContent};
use serde_json::{json, Value};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn inline_image(payload: &str) -> Message {
    Message::User {
        content: rig::OneOrMany::one(UserContent::image_base64(
            payload,
            Some(ImageMediaType::PNG),
            None,
        )),
    }
}

fn preparer(model: CodexResponsesModel, capacity: u64) -> SelectedRequestPreparer {
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(1000)
        .preamble("required instructions".into())
        .tools(vec![ToolDefinition {
            name: "inspect".into(),
            description: "required schema".into(),
            parameters: json!({"type":"object","properties":{"image":{"type":"string"}}}),
        }])
        .build();
    SelectedRequestPreparer::new(
        model,
        template,
        AdmissionLimits {
            context_capacity: Some(capacity),
            output_reservation: Some(1000),
        },
    )
}

proptest! {
    #[test]
    fn inline_image_payload_size_does_not_become_text_occupancy(size in 1usize..100_000) {
        let payload = "AAAA".repeat(size);
        let body = json!({"input":[{"role":"user","content":[{
            "type":"input_image", "image_url":format!("data:image/png;base64,{payload}")
        }]}]});
        prop_assert_eq!(media_occupancy(&body), Some(10_000));
    }
}

#[tokio::test]
async fn inline_image_reservations_can_exhaust_the_input_budget() -> TestResult {
    let server = fixture::Server::start(vec![], false).await?;
    let history = CodexHistory::default();
    let before = history.snapshot()?;
    let prepare = preparer(server.models()?.0.with_history(history.clone()), 15_000);
    let image = inline_image("AAAA");
    let fitting = prepare.prepare(std::slice::from_ref(&image), 1)?;
    assert!(fitting.input_tokens() >= 10_000);
    let rejected = prepare.prepare(&[image.clone(), image], 1);
    assert!(matches!(
        rejected,
        Err(RequestAdmissionError::BudgetExceeded { input_tokens, .. }) if input_tokens >= 20_000
    ));
    assert_eq!(before, history.snapshot()?);
    assert!(server
        .requests
        .lock()
        .map_err(|_| "request lock")?
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn unknown_media_remains_unavailable_for_admission() -> TestResult {
    for media in [
        json!({"type":"input_image","image_url":"https://example.test/image.png"}),
        json!({"type":"input_audio","input_audio":{"data":"AAAA","format":"wav"}}),
        json!({"type":"input_file","file_id":"opaque"}),
        json!({"type":"input_image","image_url":"data:application/pdf;base64,AAAA"}),
        json!({"type":"input_image","image_url":"data:image/png;base64,"}),
    ] {
        assert_eq!(media_occupancy(&json!({"input":[media]})), None);
    }
    let server = fixture::Server::start(vec![], false).await?;
    let prepare = preparer(server.models()?.0, 100_000);
    let remote = Message::User {
        content: rig::OneOrMany::one(UserContent::image_url(
            "https://example.test/image.png",
            None,
            None,
        )),
    };
    assert!(matches!(
        prepare.prepare(&[remote], 1),
        Err(RequestAdmissionError::UnknownMediaOccupancy)
    ));
    Ok(())
}

#[tokio::test]
async fn selected_inline_image_preparation_preserves_schemas_and_generation() -> TestResult {
    let server = fixture::Server::start(vec![], false).await?;
    let history = CodexHistory::default();
    let before = history.snapshot()?;
    let prepare = preparer(server.models()?.0.with_history(history.clone()), 20_000);
    let mut estimates = Vec::new();
    for size in [1, 100_000] {
        let payload = "AAAA".repeat(size);
        let prepared = prepare.prepare(&[inline_image(&payload)], 1)?;
        estimates.push(prepared.input_tokens());
        assert_eq!(prepared.generation(), 1);
        assert_eq!(prepared.output_reservation(), 1000);
        let mut body: Value = serde_json::from_slice(prepared.request_bytes())?;
        let url = format!("data:image/png;base64,{payload}");
        assert_eq!(body["input"][0]["content"][0]["image_url"], url);
        assert!(body.to_string().contains("required schema"));
        assert!(body.to_string().contains("required instructions"));
        body["input"][0]["content"][0]["image_url"] = json!("");
        assert_eq!(
            prepared.input_tokens(),
            codelet_common::token_estimator::count_tokens(&body.to_string()) as u64 + 10_000
        );
    }
    assert_eq!(estimates[0], estimates[1]);
    assert_eq!(before, history.snapshot()?);
    assert!(server
        .requests
        .lock()
        .map_err(|_| "request lock")?
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn physical_codex_image_sends_use_the_selected_image_estimate() -> TestResult {
    let payload = "AAAA".repeat(100_000);
    for streaming in [false, true] {
        let responses = if streaming {
            vec![
                fixture::terminal("completed", Some(false), None),
                fixture::terminal("completed", Some(true), None),
            ]
        } else {
            vec![fixture::baseline::response().to_string()]
        };
        let server = fixture::Server::start(responses, false).await?;
        let model = server.models()?.0.with_request_admission(AdmissionLimits {
            context_capacity: Some(20_000),
            output_reservation: Some(1000),
        });
        let call = Message::Assistant {
            id: None,
            content: rig::OneOrMany::one(AssistantContent::tool_call_with_call_id(
                "image_call",
                "call_image".into(),
                "view_image",
                json!({"path":"synthetic.png"}),
            )),
        };
        let result = Message::User {
            content: rig::OneOrMany::one(UserContent::tool_result_with_call_id(
                "image_call",
                "call_image".into(),
                rig::OneOrMany::one(ToolResultContent::image_base64(
                    &payload,
                    Some(ImageMediaType::PNG),
                    None,
                )),
            )),
        };
        let request = CompletionRequestBuilder::new(model.clone(), result)
            .messages(vec![Message::user("inspect the synthetic image"), call])
            .max_tokens(1000)
            .build();
        if streaming {
            let mut stream = model.stream(request).await?;
            while let Some(event) = stream.next().await {
                event?;
            }
        } else {
            model.completion(request).await?;
        }
        let requests = server.requests.lock().map_err(|_| "request lock")?;
        assert_eq!(requests.len(), if streaming { 2 } else { 1 });
        for request in requests.iter() {
            assert_eq!(
                request["input"][2]["output"][0]["image_url"],
                format!("data:image/png;base64,{payload}")
            );
        }
    }
    Ok(())
}
