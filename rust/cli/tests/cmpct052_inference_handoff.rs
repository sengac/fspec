//! Feature: spec/features/compaction-inference-handoff.feature

use std::sync::{atomic::AtomicBool, Arc, Mutex, PoisonError};

use anyhow::Context;
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_tools::inject_summary::{InjectSummaryArgs, InjectSummaryResult, InjectSummaryTool};
use rig::{client::CompletionClient, tool::Tool};
use serial_test::serial;
use uuid::Uuid;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[derive(Default)]
struct Output(Mutex<Vec<StreamEvent>>);

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

async fn assert_reentry_blocked(
    session: &mut Session,
    output: &Output,
    server: &MockServer,
) -> anyhow::Result<()> {
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("test-no-network")
            .base_url(server.uri())
            .build()?;
    let agent = RigAgent::new(client.agent("o3").build(), 1);
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_agent_stream(
            agent,
            "ordinary inference must wait",
            session,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(tokio::sync::Notify::new()),
            output,
            Uuid::new_v4(),
        ),
    )
    .await?;
    assert_eq!(
        server.received_requests().await.context("requests")?.len(),
        1,
        "unreconciled queued candidate must prevent every later request"
    );
    assert!(result
        .err()
        .context("unreconciled candidate blocks stream reentry")?
        .downcast_ref::<codelet_cli::interactive::compaction_handoff::CompactionHandoffError>()
        .is_some());
    Ok(())
}

#[tokio::test]
#[serial]
async fn self_target_generation_yields_before_ordinary_inference() -> anyhow::Result<()> {
    // @step Given a streaming provider that requests self-target compaction generation
    // @step When the generation handler queues its candidate
    let (result, mut session, output, server) = drive_tool(false, true).await?;
    // @step Then the tool loop yields before another provider request
    assert!(result.is_ok(), "generation handoff: {result:?}");
    let requests = server.received_requests().await.context("requests")?;
    assert_eq!(requests.len(), 1);
    // @step And the generated candidate result reports queued application
    let history = serde_json::to_string(&session.messages)?;
    assert!(history.contains("queued"));
    codelet_cli::interactive_helpers::validate_no_orphan_tool_calls(&session.messages)
        .map_err(|errors| anyhow::anyhow!("actual pair: {errors:?}"))?;
    // @step And a later stream invocation remains blocked until durable commit reconciliation
    assert_reentry_blocked(&mut session, &output, &server).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn rejected_self_target_generation_blocks_ordinary_inference() -> anyhow::Result<()> {
    // @step Given a streaming provider that requests self-target compaction generation
    // @step When the generation handler rejects its candidate
    let (result, mut session, output, server) = drive_tool(true, true).await?;
    // @step Then ordinary inference stops with a typed blocked outcome
    assert!(result
        .err()
        .context("generation blocks")?
        .downcast_ref::<codelet_cli::interactive::compaction_handoff::CompactionHandoffError>()
        .is_some());
    let requests = server.received_requests().await.context("requests")?;
    assert_eq!(requests.len(), 1);
    // @step And no compaction completion is emitted
    assert!(!output
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .any(|event| matches!(event, StreamEvent::CompactionComplete { .. })));
    // @step And a later stream invocation remains blocked until explicit reconciliation
    assert_reentry_blocked(&mut session, &output, &server).await?;
    Ok(())
}

const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-0\" label=\"work\">evidence</dag-node>";

async fn drive_tool(
    reject: bool,
    generate: bool,
) -> anyhow::Result<(anyhow::Result<()>, Session, Output, MockServer)> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())
        .map_err(anyhow::Error::msg)?;
    std::env::set_var("OPENAI_API_KEY", "test-no-network");
    let server = MockServer::start().await;
    let arguments = serde_json::to_string(&if generate {
        serde_json::json!({})
    } else {
        serde_json::json!({"content": DAG})
    })?;
    let tool_name = if generate {
        "GenerateCompaction"
    } else {
        "inject_summary"
    };
    let chunks = [
        serde_json::json!({"id":"response1","object":"chat.completion.chunk","created":1,"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"summary-call","type":"function","function":{"name":tool_name,"arguments":arguments}}]},"finish_reason":null}]}),
        serde_json::json!({"id":"response1","object":"chat.completion.chunk","created":1,"model":"o3","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}),
    ];
    let body = format!(
        "{}data: [DONE]\n\n",
        chunks
            .iter()
            .map(|chunk| format!("data: {chunk}\n\n"))
            .collect::<String>()
    );
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("test-no-network")
            .base_url(server.uri())
            .build()?;
    codelet_core::persistence::reset_stores_for_tests();
    let manifest = codelet_core::persistence::create_session_with_provider(
        "live handoff",
        directory.path(),
        "openai",
    )
    .map_err(anyhow::Error::msg)?;
    let session_id = manifest.id;
    codelet_tools::set_inject_summary_handler(
        session_id,
        Some(Arc::new(move |_, _| {
            if reject {
                Err("candidate validation rejected".into())
            } else {
                Ok(InjectSummaryResult {
                    injected_tokens: 20,
                    remaining_budget: 1000,
                })
            }
        })),
    );
    codelet_tools::set_generate_compaction_handler(
        session_id,
        Some(Arc::new(move |_| {
            Box::pin(async move {
                if reject {
                    Err("candidate generation rejected".into())
                } else {
                    Ok(DAG.into())
                }
            })
        })),
    );
    let agent = RigAgent::new(
        client
            .agent("o3")
            .tool(InjectSummaryTool::new(session_id))
            .tool(codelet_tools::GenerateCompactionTool::new(session_id))
            .build(),
        3,
    );
    let mut session = Session::new(Some("openai"))?;
    session
        .provider_manager_mut()
        .override_model_limits(Some(100_000), Some(1000));
    let output = Output::default();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_agent_stream(
            agent,
            "summarize",
            &mut session,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(tokio::sync::Notify::new()),
            &output,
            session_id,
        ),
    )
    .await;
    codelet_tools::set_inject_summary_handler(session_id, None);
    codelet_tools::set_generate_compaction_handler(session_id, None);
    Ok((result?, session, output, server))
}

#[tokio::test]
#[serial]
async fn accepted_summary_submission_stops_inference_at_the_tool_boundary() -> anyhow::Result<()> {
    // @step Given a streaming provider that requests summary injection
    // @step When the registered handler accepts the summary for commitment
    let (result, mut session, output, server) = drive_tool(false, false).await?;
    // @step Then the tool loop returns before making another provider request
    assert!(result.is_ok(), "accepted handoff: {result:?}");
    let requests = server.received_requests().await.context("requests")?;
    assert_eq!(requests.len(), 1);
    // @step And the actual summary tool call and result remain paired
    codelet_cli::interactive_helpers::validate_no_orphan_tool_calls(&session.messages)
        .map_err(|errors| anyhow::anyhow!("actual tool pair retained: {errors:?}"))?;
    assert!(serde_json::to_string(&session.messages)?.contains("summary-call"));
    // @step And a later stream invocation remains blocked until durable commit reconciliation
    assert_reentry_blocked(&mut session, &output, &server).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn queued_summary_does_not_claim_durable_application() -> anyhow::Result<()> {
    // @step Given a registered summary handler that accepts a candidate
    let session_id = Uuid::new_v4();
    codelet_tools::set_inject_summary_handler(
        session_id,
        Some(Arc::new(|_, _| {
            Ok(InjectSummaryResult {
                injected_tokens: 20,
                remaining_budget: 1000,
            })
        })),
    );
    // @step When the summary tool serializes its accepted result
    let result = InjectSummaryTool::new(session_id)
        .call(InjectSummaryArgs {
            content: DAG.into(),
        })
        .await;
    codelet_tools::set_inject_summary_handler(session_id, None);
    let result = result?;
    // @step Then the result explicitly reports queued rather than committed
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result)?["status"],
        "queued"
    );
    // @step And the queued result does not publish an unmeasured remaining request budget
    assert!(serde_json::from_str::<serde_json::Value>(&result)?
        .get("remaining_budget")
        .is_none());
    Ok(())
}

#[tokio::test]
#[serial]
async fn rejected_summary_submission_blocks_subsequent_inference() -> anyhow::Result<()> {
    // @step Given a streaming provider that requests summary injection
    // @step When the registered handler rejects the candidate
    let (result, mut session, output, server) = drive_tool(true, false).await?;
    // @step Then the tool loop returns a typed blocked outcome before another provider request
    assert!(result
        .err()
        .context("rejected summary blocks inference")?
        .downcast_ref::<codelet_cli::interactive::compaction_handoff::CompactionHandoffError>()
        .is_some());
    let requests = server.received_requests().await.context("requests")?;
    assert_eq!(requests.len(), 1);
    // @step And no compaction completion is emitted
    assert!(!output
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .any(|event| matches!(event, StreamEvent::CompactionComplete { .. })));
    // @step And a later stream invocation remains blocked until explicit reconciliation
    assert_reentry_blocked(&mut session, &output, &server).await?;
    Ok(())
}
