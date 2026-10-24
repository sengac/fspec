//! Feature: spec/features/stream-recovery-compaction-builder.feature

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use codelet_cli::compaction_operation::CompactionFailure;
use codelet_cli::compactor_sub_agent::{
    set_compactor_sub_agent_handler, set_terminal_overflow_owner,
};
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::{
    append_message, create_session_with_provider, get_message, load_session, reset_stores_for_tests,
};
use codelet_core::RigAgent;
use codelet_providers::{ProviderManager, ProviderType};
use rig::{client::CompletionClient, message::Message};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Work\">Recovered task context.</dag-node>\n<dag-files>\n</dag-files>";

enum Recovery {
    Hook,
    BuilderFailure,
    ProviderOverflow,
    RetryTooLarge,
}

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

async fn exercise(recovery: Recovery) -> TestResult {
    let builder_fails = matches!(recovery, Recovery::BuilderFailure);
    let provider_overflow = matches!(recovery, Recovery::ProviderOverflow);
    let retry_too_large = matches!(recovery, Recovery::RetryTooLarge);
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("stream recovery", directory.path(), "openai")?;
    let history = if provider_overflow {
        "original question".to_owned()
    } else {
        "oversized conversation history ".repeat(6_000)
    };
    append_message(&mut manifest, "user", &history)?;
    append_message(&mut manifest, "assistant", "work in progress")?;
    let originals = manifest
        .messages
        .iter()
        .map(|reference| get_message(reference.message_id))
        .collect::<Result<Vec<_>, _>>()?;
    let originals = serde_json::to_value(originals)?;
    let durable = serde_json::to_value(load_session(manifest.id)?)?;

    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(16_000), Some(4096));
    manager.set_model_direct("openai", "selected-model", Some(16_000), Some(4096), None)?;
    let mut session = Session::from_provider_manager(manager);
    session.messages = vec![
        Message::user(history),
        Message::assistant("work in progress"),
    ];
    let messages = serde_json::to_value(&session.messages)?;
    let calls = Arc::new(AtomicUsize::new(0));
    let builder_calls = calls.clone();
    set_compactor_sub_agent_handler(
        manifest.id,
        Some(Arc::new(move |_, _| {
            builder_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                if builder_fails {
                    Err("declared builder failure".to_owned())
                } else if retry_too_large {
                    Ok(DAG.replace("Recovered task context.", &"context ".repeat(900)))
                } else {
                    Ok(DAG.to_owned())
                }
            })
        })),
    );
    set_terminal_overflow_owner(manifest.id, !provider_overflow);

    let server = MockServer::start().await;
    let chunk = serde_json::json!({
        "id":"recovered", "object":"chat.completion.chunk", "created":1,
        "model":"selected-model",
        "choices":[{"index":0,"delta":{"role":"assistant","content":"resumed"},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":100,"completion_tokens":1,"total_tokens":101}
    });
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n")),
        )
        .mount(&server)
        .await;
    if provider_overflow {
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error":{"code":"context_length_exceeded","message":"declared rejection"}
            })))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
    }
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline")
            .base_url(server.uri())
            .build()?;
    let agent = RigAgent::new(client.agent("selected-model").max_tokens(4096).build(), 1);
    let output = Output::default();
    let busy = Arc::new(AtomicBool::new(false));
    let prompt = if retry_too_large {
        "task ".repeat(1_500)
    } else {
        "resume original task".to_owned()
    };
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        run_agent_stream(
            agent,
            &prompt,
            &mut session,
            Arc::new(AtomicBool::new(false)),
            busy.clone(),
            Arc::new(tokio::sync::Notify::new()),
            &output,
            manifest.id,
        ),
    )
    .await;
    set_compactor_sub_agent_handler(manifest.id, None);
    set_terminal_overflow_owner(manifest.id, false);
    let result = result?;

    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "recovery must invoke the registered builder: {result:?}"
    );
    assert!(!busy.load(Ordering::Acquire));
    let completions = output
        .0
        .lock()
        .map_err(|error| error.to_string())?
        .iter()
        .filter(|event| matches!(event, StreamEvent::CompactionComplete(_)))
        .count();
    let requests = server
        .received_requests()
        .await
        .ok_or("requests unavailable")?;
    if builder_fails || retry_too_large {
        let error = result.err().ok_or("builder failure must stop recovery")?;
        if builder_fails {
            assert!(matches!(
                error.downcast_ref::<CompactionFailure>(),
                Some(CompactionFailure::Builder(message)) if message == "declared builder failure"
            ));
        } else {
            assert!(matches!(
                error.downcast_ref::<CompactionFailure>(),
                Some(CompactionFailure::Commit(message)) if message.contains("admission")
            ));
        }
        assert_eq!(completions, 0);
        assert!(requests.is_empty());
        assert!(session.ensure_compaction_inference_ready().is_err());
        assert_eq!(serde_json::to_value(&session.messages)?, messages);
        assert_eq!(serde_json::to_value(load_session(manifest.id)?)?, durable);
    } else {
        result?;
        assert_eq!(completions, 1);
        assert_eq!(requests.len(), if provider_overflow { 2 } else { 1 });
        let request = requests.last().ok_or("resumed request")?;
        let body = String::from_utf8_lossy(&request.body);
        assert!(body.contains("Recovered task context."));
        assert!(body.contains("resume original task"));
        assert!(!body.contains("oversized conversation history"));
        session.ensure_compaction_inference_ready()?;
        let committed = load_session(manifest.id)?;
        let boundary = committed
            .compaction_commit
            .ok_or("durable compaction boundary")?;
        assert_eq!(boundary.identity.generation, 1);
        assert!(!boundary.publication_pending);
    }
    reset_stores_for_tests();
    let retained = manifest
        .messages
        .iter()
        .map(|reference| get_message(reference.message_id))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(serde_json::to_value(retained)?, originals);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn hook_triggered_recovery_commits_a_builder_dag_before_resuming() -> TestResult {
    exercise(Recovery::Hook).await
}

#[tokio::test]
#[serial_test::serial]
async fn builder_failure_stops_hook_triggered_recovery_without_retrying_oversized_history(
) -> TestResult {
    exercise(Recovery::BuilderFailure).await
}

#[tokio::test]
#[serial_test::serial]
async fn provider_overflow_invokes_the_registered_compaction_builder() -> TestResult {
    exercise(Recovery::ProviderOverflow).await
}

#[tokio::test]
#[serial_test::serial]
async fn recovery_admits_the_resume_prompt_before_committing_a_replacement() -> TestResult {
    exercise(Recovery::RetryTooLarge).await
}
