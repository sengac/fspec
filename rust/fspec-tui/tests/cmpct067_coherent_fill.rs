//! Feature: spec/features/coherent-request-context-fill.feature

use codelet_fspec_tui::store::agent_view::TokenState;
use codelet_rpc_types::{ContextFillInfo, StreamChunk};
use serde_json::json;

#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn background() -> Result<
    (
        tempfile::TempDir,
        codelet_sessions::background_session::BackgroundSession,
    ),
    Box<dyn std::error::Error>,
> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let session = codelet_cli::session::Session::from_provider_manager(
        codelet_providers::ProviderManager::for_testing(
            codelet_providers::ProviderType::OpenAI,
            Some(4000),
            Some(100),
        ),
    );
    let background = codelet_sessions::background_session::BackgroundSession::new(
        uuid::Uuid::new_v4(),
        "context".into(),
        directory.path().to_string_lossy().into(),
        None,
        None,
        session,
        tokio::sync::mpsc::channel(4).0,
        None,
        None,
        None,
        tokio::sync::broadcast::channel(32).0,
        tokio::sync::broadcast::channel(4).0,
        tokio::sync::broadcast::channel(4).0,
    );
    Ok((directory, background))
}

fn fill(percent: u32, occupancy: f64) -> StreamChunk {
    StreamChunk::context_fill_update(ContextFillInfo {
        fill_percentage: percent,
        effective_tokens: occupancy,
        threshold: 2000.0,
        context_window: 4000.0,
    })
}

#[test]
fn multisegment_spend_cannot_change_current_request_fill() -> TestResult {
    // @step Given a current request snapshot and cumulative output from earlier segments
    let mut state = TokenState::default();
    state.apply_chunk(&fill(55, 1100.0));
    let tokens = serde_json::from_value(json!({"TokenUpdate":{"tokens":{
        "input_tokens":1000,"output_tokens":20000,"reasoning_tokens":3000,
        "cache_read_input_tokens":400,"cache_creation_input_tokens":null,
        "cumulative_billed_input":40000,"cumulative_billed_output":20000,
        "tokens_per_second":null
    }}}))?;
    // @step When the TUI reduces a context update followed by cumulative token updates
    state.apply_chunk(&tokens);
    // @step Then its threshold-relative current request fill remains unchanged
    assert_eq!(
        state.context_fill_pct, 55,
        "display spend overwrote physical occupancy"
    );
    // @step And cumulative generated spend remains separately visible
    assert_eq!(state.output_tokens, 20000);
    Ok(())
}

#[test]
#[serial_test::serial]
fn stale_context_events_cannot_overwrite_a_newer_generation() -> TestResult {
    // @step Given an estimated committed generation snapshot followed by a new measured request
    let mut state = TokenState::default();
    let (_directory, background) = background()?;
    let mut emitted = background.subscribe_to_stream();
    let snapshot: StreamChunk = serde_json::from_value(json!({"CommittedCompaction":{"snapshot":{
        "generation":7,"tokens":{"input_tokens":200,"output_tokens":0},
        "contextFill":{"fill_percentage":10,"effective_tokens":200,
            "threshold":2000,"context_window":4000},"originalTokens":3000,
        "estimated":true,"reconciled":false
    }}}))?;
    state.apply_chunk(&snapshot);
    background.handle_output(snapshot);
    let measured: StreamChunk = serde_json::from_value(json!({"ContextSnapshot":{"snapshot":{
        "model":"codex","generation":7,"request":2,"input_tokens":1000,
        "output_tokens":100,"capacity":4000,"threshold":2000,
        "provenance":"measured"
    }}}))?;
    state.apply_chunk(&measured);
    background.handle_output(measured);
    // @step When an older generation or earlier request event arrives afterward
    state.apply_chunk(&fill(95, 1900.0));
    for (generation, request) in [(6, 99), (7, 1)] {
        let old = serde_json::from_value(json!({"ContextSnapshot":{"snapshot":{
            "model":"codex","generation":generation,"request":request,"input_tokens":1900,
            "output_tokens":0,"capacity":4000,"threshold":2000,
            "provenance":"measured"
        }}}))?;
        state.apply_chunk(&old);
        background.handle_output(old);
    }
    // @step Then the latest measured context remains unchanged on backend and TUI
    assert_eq!(
        state.context_fill_pct, 55,
        "stale event overwrote measured generation"
    );
    assert!(!state.estimated);
    let mut contexts = Vec::new();
    while let Ok(event) = emitted.try_recv() {
        if let StreamChunk::ContextSnapshot { snapshot } = event {
            contexts.push((snapshot.generation, snapshot.request, snapshot.input_tokens));
        }
    }
    assert_eq!(contexts, [(7, 2, Some(1000))]);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn unknown_occupancy_and_capacity_cannot_claim_measured_fit() -> TestResult {
    use codelet_providers::request_admission::{AdmissionLimits, RequestAdmissionError};
    use futures::StreamExt;
    use rig::completion::{CompletionModel, CompletionRequestBuilder};
    use rig::message::{ImageMediaType, Message, UserContent};

    // @step Given context usage or capacity is unavailable
    for missing_occupancy in [true, false] {
        let (_directory, background) = background()?;
        let mut events = background.subscribe_to_stream();
        let snapshot = codelet_rpc_types::RequestContextSnapshot {
            model: "shared-model".into(),
            generation: 7,
            request: 2,
            physical_request: None,
            input_tokens: (!missing_occupancy).then_some(1000),
            output_tokens: Some(100),
            capacity: missing_occupancy.then_some(4000),
            threshold: Some(2000),
            provenance: codelet_rpc_types::ContextProvenance::Unknown,
        };
        // @step When backend and TUI consume the context snapshot
        background.handle_output(StreamChunk::ContextSnapshot {
            snapshot: snapshot.clone(),
        });
        let chunk = events.try_recv()?;
        let mut state = TokenState::default();
        state.apply_chunk(&chunk);
        // @step Then provenance remains unknown and no guessed fit percentage authorizes inference
        let StreamChunk::ContextSnapshot {
            snapshot: published,
        } = chunk
        else {
            return Err("backend omitted context snapshot".into());
        };
        assert_eq!(
            published.provenance,
            codelet_rpc_types::ContextProvenance::Unknown
        );
        assert!(published.context_fill().is_none());
        assert!(state.estimated);
        assert_eq!(state.context_threshold_tokens, 0);
        assert_eq!(state.context_fill_pct, 0);
        let server = fixture::Server::start(
            vec![fixture::event(json!({
                "type":"response.completed", "response":fixture::baseline::response()
            }))],
            false,
        )
        .await?;
        let model = server.models()?.0.with_request_admission(AdmissionLimits {
            context_capacity: snapshot.capacity,
            output_reservation: Some(100),
        });
        let content = if missing_occupancy {
            Message::User {
                content: rig::OneOrMany::one(UserContent::image_url(
                    "https://example.test/image.png",
                    Some(ImageMediaType::PNG),
                    None,
                )),
            }
        } else {
            Message::user("known text occupancy")
        };
        let request = CompletionRequestBuilder::new(model.clone(), "next")
            .messages(vec![content])
            .build();
        let mut stream = model.stream(request).await?;
        let expected = if missing_occupancy {
            RequestAdmissionError::UnknownMediaOccupancy
        } else {
            RequestAdmissionError::MissingContextCapacity
        };
        let mut rejected = false;
        while let Some(event) = stream.next().await {
            match event {
                Err(rig::completion::CompletionError::RequestError(error)) => {
                    assert_eq!(
                        error.downcast_ref::<RequestAdmissionError>(),
                        Some(&expected)
                    );
                    rejected = true;
                }
                Ok(rig::streaming::StreamedAssistantContent::Final(_)) => {
                    return Err("unknown context authorized a physical request".into());
                }
                Err(error) => return Err(error.into()),
                _ => {}
            }
        }
        assert!(rejected, "unknown context must reject physical inference");
        assert!(server.requests.lock().map_err(|_| "wire lock")?.is_empty());
    }
    Ok(())
}
