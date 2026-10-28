use codelet_providers::{codex::history::CodexHistory, request_admission::AdmissionLimits};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder, GetTokenUsage};
use rig::streaming::StreamedAssistantContent;
use serde_json::{json, Value};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn terminal(usage: Value, end_turn: bool) -> String {
    let mut response = fixture::baseline::response();
    response["end_turn"] = json!(end_turn);
    response["usage"] = usage;
    fixture::event(json!({"type":"response.completed","response":response}))
}

fn usage(input: u64, cached: u64) -> Value {
    json!({"input_tokens":input,"output_tokens":0,"total_tokens":input,
        "input_tokens_details":{"cached_tokens":cached},"output_tokens_details":{"reasoning_tokens":0}})
}

async fn invalid_physical(continuation: bool) -> TestResult {
    let server = fixture::Server::start(
        vec![
            terminal(usage(10, 15), !continuation),
            terminal(usage(10, 0), true),
        ],
        false,
    )
    .await?;
    let history = CodexHistory::default();
    let before = history.snapshot()?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(10000),
            output_reservation: Some(100),
        });
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "hello")
                .max_tokens(100)
                .build(),
        )
        .await?;
    let mut errors = Vec::new();
    let mut finals = 0;
    let mut measurements = 0;
    while let Some(event) = stream.next().await {
        match event {
            Err(error) => errors.push(error.to_string()),
            Ok(StreamedAssistantContent::Final(_)) => finals += 1,
            Ok(StreamedAssistantContent::Usage(_)) => measurements += 1,
            _ => {}
        }
    }
    assert_eq!(
        errors.len(),
        1,
        "invalid physical usage accepted: {errors:?}"
    );
    assert!(errors[0].contains("usage"), "wrong failure: {errors:?}");
    assert_eq!(finals, 0);
    assert_eq!(measurements, 0);
    assert_eq!(server.requests.lock().map_err(|_| "capture lock")?.len(), 1);
    assert_eq!(history.snapshot()?, before);
    Ok(())
}

async fn missing_physical(continuation: bool) -> TestResult {
    let mut response = fixture::baseline::response();
    response["end_turn"] = json!(!continuation);
    response
        .as_object_mut()
        .ok_or("response object")?
        .remove("usage");
    let server = fixture::Server::start(
        vec![
            fixture::event(json!({"type":"response.completed","response":response})),
            terminal(usage(10, 0), true),
        ],
        false,
    )
    .await?;
    let history = CodexHistory::default();
    let before = history.snapshot()?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(10000),
            output_reservation: Some(100),
        });
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "hello")
                .max_tokens(100)
                .build(),
        )
        .await?;
    let observed = tokio::time::timeout(std::time::Duration::from_secs(4), async {
        let mut errors = Vec::new();
        let mut finals = 0;
        let mut measurements = 0;
        while let Some(event) = stream.next().await {
            match event {
                Err(error) => errors.push(error.to_string()),
                Ok(StreamedAssistantContent::Final(_)) => finals += 1,
                Ok(StreamedAssistantContent::Usage(_)) => measurements += 1,
                _ => {}
            }
        }
        (errors, finals, measurements)
    })
    .await?;
    assert_eq!(
        observed.0.len(),
        1,
        "missing physical usage accepted: {observed:?}"
    );
    assert!(
        observed.0[0].contains("missing physical usage"),
        "{observed:?}"
    );
    assert_eq!(observed.1, 0);
    assert_eq!(observed.2, 0);
    assert_eq!(server.requests.lock().map_err(|_| "capture lock")?.len(), 1);
    assert_eq!(history.snapshot()?, before);
    Ok(())
}

#[tokio::test]
async fn missing_final_physical_usage_cannot_publish_completion() -> TestResult {
    // @step Given a Codex final terminal without physical usage
    // @step When the physical stream is consumed
    // @step Then missing usage fails without measured usage or final completion
    // @step And the committed checkpoint remains unchanged
    missing_physical(false).await
}

#[tokio::test]
async fn missing_physical_usage_cannot_start_a_continuation() -> TestResult {
    // @step Given a Codex continuation terminal without physical usage
    // @step When the physical stream is consumed
    // @step Then missing usage fails before continuation without measured usage or final completion
    // @step And the committed checkpoint remains unchanged
    missing_physical(true).await
}

#[tokio::test]
async fn invalid_final_physical_usage_cannot_publish_completion() -> TestResult {
    // @step Given a Codex terminal whose cache subset exceeds input tokens
    // @step When the physical stream is consumed
    // @step Then invalid usage fails without measured usage or final completion
    // @step And the committed checkpoint remains unchanged
    invalid_physical(false).await
}

#[tokio::test]
async fn later_valid_usage_cannot_rehabilitate_an_invalid_physical_measurement() -> TestResult {
    // @step Given invalid physical usage followed by a valid continuation terminal
    // @step When the physical stream is consumed
    // @step Then invalid usage fails before continuation without measured usage or final completion
    // @step And the committed checkpoint remains unchanged
    invalid_physical(true).await
}

#[tokio::test]
async fn missing_physical_detail_remains_unknown_in_aggregate_spend() -> TestResult {
    // @step Given Codex physical usage without cache or reasoning details
    let server = fixture::Server::start(
        vec![terminal(
            json!({"input_tokens":10,"output_tokens":2,"total_tokens":12}),
            true,
        )],
        false,
    )
    .await?;
    let model = server.models()?.0.with_request_admission(AdmissionLimits {
        context_capacity: Some(10000),
        output_reservation: Some(100),
    });
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "hello")
                .max_tokens(100)
                .build(),
        )
        .await?;
    // @step When the physical stream finishes
    let mut final_response = None;
    while let Some(event) = stream.next().await {
        if let StreamedAssistantContent::Final(response) = event? {
            final_response = Some(response);
        }
    }
    // @step Then aggregate inclusive totals remain measured and both detail subsets remain unknown
    let response = final_response.ok_or("missing final")?;
    let measured = response.token_usage().ok_or("missing usage")?;
    assert_eq!(measured.total_tokens, 12);
    assert_eq!(measured.cache_read_input_tokens, None);
    assert_eq!(measured.reasoning_tokens, None);
    Ok(())
}

#[tokio::test]
async fn aggregate_physical_usage_overflow_fails_boundedly() -> TestResult {
    // @step Given two individually valid Codex measurements whose sum exceeds the token range
    let server = fixture::Server::start(
        vec![
            terminal(usage(u64::MAX, 0), false),
            terminal(usage(1, 0), true),
        ],
        false,
    )
    .await?;
    let history = CodexHistory::default();
    let before = history.snapshot()?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(10000),
            output_reservation: Some(100),
        });
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "hello")
                .max_tokens(100)
                .build(),
        )
        .await?;
    // @step When the continuation stream is consumed
    let mut errors = Vec::new();
    let mut finals = 0;
    while let Some(event) = stream.next().await {
        match event {
            Err(error) => errors.push(error.to_string()),
            Ok(StreamedAssistantContent::Final(_)) => finals += 1,
            _ => {}
        }
    }
    // @step Then accounting fails without final completion or a changed checkpoint
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("overflow"), "wrong failure: {errors:?}");
    assert_eq!(finals, 0);
    assert_eq!(history.snapshot()?, before);
    Ok(())
}
