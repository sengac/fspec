//! Feature: spec/features/codex-committed-lineage.feature

use codelet_providers::{codex::history::CodexHistory, session::ProviderSession};
use codelet_test_helpers::setup_temp_directory;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::message::{AssistantContent, Message};
use serde_json::{json, Value};
use std::{fs, sync::Arc};

#[path = "fixtures/prov148.rs"]
mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn completed(output: Vec<Value>) -> String {
    fixture::event(json!({"type":"response.completed","response":{
        "id":"resp_1","status":"completed","output":output,"end_turn":true,"usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
    }}))
}

#[tokio::test]
async fn incompatible_continuation_cannot_send_stale_checkpoint_content() -> TestResult {
    for identity in [
        "assistant ancestry",
        "tool call ancestry",
        "tool result ancestry",
        "model",
        "protocol",
        "DAG generation",
    ] {
        // @step Given a durable Codex checkpoint for a committed DAG generation
        let directory = setup_temp_directory()?;
        let path = directory.path().join("checkpoint.json");
        let history = CodexHistory::default();
        let server =
            fixture::Server::start(vec![completed(vec![]), completed(vec![])], false).await?;
        let model = server
            .models()?
            .0
            .with_history(history.clone())
            .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
                context_capacity: Some(100_000),
                output_reservation: Some(1000),
            });
        let initial = json!({"model":"shared-model","input":[{"role":"user","content":[{"type":"input_text","text":"inspect"}]}]});
        let mut initial = initial;
        let mut output = vec![
            json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":"stale assistant"}]}),
        ];
        if identity.starts_with("tool") {
            output = vec![
                json!({"type":"function_call","call_id":"call_1","name":"inspect","arguments":"{}"}),
                json!({"type":"function_call_output","call_id":"call_1","output":"recorded"}),
            ];
        }
        if identity == "model" {
            initial["model"] = json!("other-model");
        }
        if identity == "DAG generation" {
            initial["input"].as_array_mut().ok_or("input missing")?.insert(0, json!({"role":"user","content":[{"type":"input_text","text":"<system-reminder><!-- type:compaction-dag -->old generation</system-reminder>"}]}));
        }
        history.commit(&initial, &output)?;
        fs::write(&path, serde_json::to_vec(&history.snapshot()?)?)?;
        let previous = history.snapshot()?;
        let durable = fs::read(&path)?;
        // @step And the current request changes the checkpoint's <identity>
        let mut builder = CompletionRequestBuilder::new(model.clone(), "next").messages(vec![
            Message::user("inspect"),
            Message::assistant(if identity == "assistant ancestry" {
                "changed assistant"
            } else {
                "stale assistant"
            }),
        ]);
        if identity.starts_with("tool") {
            builder = CompletionRequestBuilder::new(model.clone(), "next").messages(vec![
                Message::user("inspect"),
                Message::Assistant {
                    id: None,
                    content: rig::OneOrMany::one(AssistantContent::tool_call_with_call_id(
                        "fc_1",
                        "call_1".into(),
                        "inspect",
                        if identity == "tool call ancestry" {
                            json!({"changed":true})
                        } else {
                            json!({})
                        },
                    )),
                },
                Message::tool_result_with_call_id(
                    "fc_1",
                    Some("call_1".into()),
                    if identity == "tool result ancestry" {
                        "changed result"
                    } else {
                        "recorded"
                    },
                ),
            ]);
        } else if identity == "protocol" {
            builder = builder.additional_params(json!({"previous_response_id":"removed-delta"}));
        } else if identity == "DAG generation" {
            builder = CompletionRequestBuilder::new(model.clone(), "next").messages(vec![Message::user("<system-reminder><!-- type:compaction-dag -->changed generation</system-reminder>"), Message::user("inspect"), Message::assistant("stale assistant")]);
        }
        let request = builder.build();
        // @step When I continue the session using the same user prefix
        let result = model.stream(request).await;
        let rejected = result.is_err();
        if let Ok(mut stream) = result {
            while let Some(event) = stream.next().await {
                event?;
            }
        }
        // @step Then I receive an explicit lineage failure before any provider request
        assert!(rejected, "changed {identity} was silently accepted");
        // @step And the last committed checkpoint and generation remain unchanged
        assert_eq!(history.snapshot()?, previous);
        assert_eq!(fs::read(path)?, durable);
        // @step And no completion or alternate request is emitted
        assert!(server
            .requests
            .lock()
            .map_err(|_| "capture lock poisoned")?
            .is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn durable_generation_transition_survives_cold_resume_without_old_checkpoint_content(
) -> TestResult {
    // @step Given a committed Codex session with recorded suffix work
    let directory = setup_temp_directory()?;
    let path = directory.path().join("envelope.json");
    let session = ProviderSession::for_provider("codex");
    let history = &session
        .extension::<codelet_providers::codex::session::CodexSession>("codex")
        .ok_or("Codex extension missing")?
        .history;
    let initial = json!({"model":"shared-model","input":[{"role":"user","content":[{"type":"input_text","text":"old request"}]}]});
    history.commit(&initial, &[json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":"replaced assistant"}]})])?;
    let destination = path.clone();
    session.bind_writers(|_| {
        let destination = destination.clone();
        Arc::new(move |metadata| {
            let bytes = serde_json::to_vec(&metadata)?;
            fs::write(&destination, bytes).map_err(|error| {
                codelet_providers::session::ExtensionError::Persistence(error.to_string())
            })
        })
    })?;
    // @step And preparing the next generation checkpoint does not mutate memory or call its writer
    let previous = history.snapshot()?;
    let metadata = session.prepare_generation(1, "shared-model")?;
    assert_eq!(history.snapshot()?, previous);
    assert!(!path.exists());
    // @step When a validated DAG installation durably records a new generation and provider transition
    fs::write(&path, serde_json::to_vec(&metadata)?)?;
    // @step And the already durable checkpoint is installed without a second persistence transaction
    session.bind_writers(|_| {
        Arc::new(|_| {
            Err(codelet_providers::session::ExtensionError::Persistence(
                "second transaction forbidden".into(),
            ))
        })
    })?;
    session.install_committed_generation(1, &metadata)?;
    assert_eq!(history.generation()?, 1);
    let recorded: Value = serde_json::from_slice(&fs::read(&path)?)?;
    assert_eq!(recorded["codexContinuation"]["generation"], 1);
    // @step And I cold resume the session and send an explicit full-current-DAG request
    let resumed = ProviderSession::for_provider("codex");
    resumed.restore(&[fs::read_to_string(path)?])?;
    let history = resumed
        .extension::<codelet_providers::codex::session::CodexSession>("codex")
        .ok_or("resumed Codex extension missing")?
        .history
        .clone();
    let server = fixture::Server::start(vec![completed(vec![])], false).await?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        });
    let dag = "<system-reminder><!-- type:compaction-dag -->new committed DAG</system-reminder>";
    let request = CompletionRequestBuilder::new(model.clone(), "compatible suffix")
        .messages(vec![Message::user(dag)])
        .build();
    let mut stream = model.stream(request).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    let requests = server
        .requests
        .lock()
        .map_err(|_| "capture lock poisoned")?;
    let wire = requests
        .first()
        .ok_or("resumed request missing")?
        .to_string();
    // @step Then the captured request contains only the new committed generation and compatible suffix
    assert_eq!(wire.matches("new committed DAG").count(), 1);
    assert_eq!(wire.matches("compatible suffix").count(), 1);
    // @step And replaced assistant and tool history cannot reappear from the old checkpoint
    assert!(!wire.contains("replaced assistant"));
    assert!(!wire.contains("old request"));
    assert_eq!(serde_json::to_value(history.snapshot()?)?["generation"], 1);
    Ok(())
}

#[tokio::test]
async fn compatible_full_current_dag_continuation_preserves_recorded_suffix_work() -> TestResult {
    // @step Given a durable Codex checkpoint matching the full current DAG lineage
    let history = CodexHistory::default();
    let assistant = json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":"committed assistant"}]});
    let reasoning = json!({"type":"reasoning","summary":[],"encrypted_content":"opaque-original"});
    let call = json!({"type":"function_call","id":"fc_1","call_id":"call_1","name":"inspect","arguments":"{}"});
    let dag = "<system-reminder><!-- type:compaction-dag -->committed DAG</system-reminder>";
    let reminder = "<system-reminder><!-- type:required-context id:reminder-42 -->required reminder</system-reminder>";
    let server = fixture::Server::start(
        vec![
            completed(vec![reasoning, assistant, call]),
            completed(vec![]),
        ],
        false,
    )
    .await?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        });
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "inspect")
                .messages(vec![Message::user(dag), Message::user(reminder)])
                .build(),
        )
        .await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    // @step And compatible assistant and tool outcomes extend the committed prefix
    let request = CompletionRequestBuilder::new(model.clone(), "next")
        .messages(vec![
            Message::user(dag),
            Message::user(reminder),
            Message::user("inspect"),
            Message::assistant("committed assistant"),
            Message::Assistant {
                id: None,
                content: rig::OneOrMany::one(AssistantContent::tool_call_with_call_id(
                    "fc_1",
                    "call_1".into(),
                    "inspect",
                    json!({}),
                )),
            },
            Message::tool_result_with_call_id(
                "fc_1",
                Some("call_1".into()),
                "recorded tool result",
            ),
            Message::assistant("compatible suffix"),
        ])
        .build();
    // @step When I continue the session in explicit full-current-DAG mode
    let mut stream = model.stream(request).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    // @step Then the captured provider request contains the current DAG and compatible suffix exactly once
    let requests = server
        .requests
        .lock()
        .map_err(|_| "capture lock poisoned")?;
    let wire = requests
        .get(1)
        .ok_or("continuation was not sent")?
        .to_string();
    assert_eq!(
        wire.matches("compatible suffix").count(),
        1,
        "compatible suffix was discarded"
    );
    // @step And opaque recorded reasoning and required reminder identities are preserved
    assert_eq!(wire.matches("opaque-original").count(), 1);
    assert_eq!(wire.matches("committed DAG").count(), 1);
    assert_eq!(wire.matches("reminder-42").count(), 1);
    assert_eq!(wire.matches("recorded tool result").count(), 1);
    Ok(())
}
