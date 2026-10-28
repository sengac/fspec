//! Feature: spec/features/request-admission-token-units.feature

use codelet_common::token_estimator::count_tokens;
use codelet_providers::request_admission::{
    admit_complete_request, admit_serialized_request, AdmissionLimits, RequestAdmissionError,
};
use serde_json::json;

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn large_text_requests_below_the_token_budget_remain_admissible() -> TestResult {
    let request = json!({
        "instructions": "Follow the user's configured context window.",
        "input": "retained conversation evidence ".repeat(20_000),
        "tools": [{"name":"inspect", "description":"Inspect the project files."}],
        "max_output_tokens": 128_000
    })
    .to_string();
    let estimated = count_tokens(&request) as u64;
    let limits = AdmissionLimits {
        context_capacity: Some(320_000),
        output_reservation: Some(128_000),
    };
    assert!(request.len() > 320_000);
    assert!(estimated + 128_000 < 320_000);
    for admitted in [
        admit_complete_request(request.as_bytes(), limits)?,
        admit_serialized_request(request.as_bytes(), limits, Some(0))?,
    ] {
        assert_eq!(admitted.input_tokens(), estimated);
        assert_eq!(admitted.request_bytes(), request.as_bytes());
    }
    Ok(())
}

#[test]
fn text_requests_exceeding_the_token_budget_remain_blocked() -> TestResult {
    let request = json!({"input":"retained evidence ".repeat(1000)}).to_string();
    let estimated = count_tokens(&request) as u64;
    let limits = AdmissionLimits {
        context_capacity: Some(estimated + 99),
        output_reservation: Some(100),
    };
    assert_eq!(
        admit_complete_request(request.as_bytes(), limits).err(),
        Some(RequestAdmissionError::BudgetExceeded {
            input_tokens: estimated,
            output_reservation: 100,
            context_capacity: estimated + 99,
        })
    );
    Ok(())
}

#[test]
fn inline_image_payloads_do_not_inflate_text_token_estimates() -> TestResult {
    let mut estimates = Vec::new();
    for size in [1, 100_000] {
        let mut body = json!({
            "input":[{"type":"input_image", "image_url":format!(
                "data:image/png;base64,{}", "AAAA".repeat(size)
            )}],
            "tools":[{"name":"inspect", "description":"Inspect the image"}]
        });
        let request = body.to_string();
        body["input"][0]["image_url"] = json!("");
        let expected = count_tokens(&body.to_string()) as u64 + 10_000;
        let admitted = admit_complete_request(
            request.as_bytes(),
            AdmissionLimits {
                context_capacity: Some(20_000),
                output_reservation: Some(1000),
            },
        )?;
        assert_eq!(admitted.input_tokens(), expected);
        assert_eq!(admitted.request_bytes(), request.as_bytes());
        estimates.push(admitted.input_tokens());
    }
    assert_eq!(estimates[0], estimates[1]);
    Ok(())
}

#[test]
fn invalid_text_encoding_cannot_authorize_admission() {
    let result = admit_serialized_request(
        &[0xff, 0xfe],
        AdmissionLimits {
            context_capacity: Some(1000),
            output_reservation: Some(100),
        },
        Some(0),
    );
    assert!(matches!(result, Err(RequestAdmissionError::Preparation(_))));
}

#[tokio::test]
async fn physical_provider_sends_do_not_reject_low_token_usage() -> TestResult {
    use futures::StreamExt;
    use rig::completion::{CompletionModel, CompletionRequestBuilder};

    let prompt = "retained conversation evidence ".repeat(20_000);
    assert!(count_tokens(&prompt) < 400_000 / 4);
    assert!(prompt.len() > 400_000);
    for streaming in [false, true] {
        let response = if streaming {
            fixture::terminal("completed", None, None)
        } else {
            fixture::baseline::response().to_string()
        };
        let server = fixture::Server::start(vec![response], false).await?;
        let model = server.models()?.0.with_request_admission(AdmissionLimits {
            context_capacity: Some(400_000),
            output_reservation: Some(128_000),
        });
        let request = CompletionRequestBuilder::new(model.clone(), &prompt).build();
        if streaming {
            let mut stream = model.stream(request).await?;
            while let Some(event) = stream.next().await {
                event?;
            }
        } else {
            model.completion(request).await?;
        }
        assert_eq!(server.requests.lock().map_err(|_| "capture lock")?.len(), 1);
    }
    Ok(())
}
