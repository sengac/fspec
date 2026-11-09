//! Feature: spec/features/codex-offline-reconnection.feature

#[path = "fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "fixtures/prov160.rs"]
pub mod fixture;
#[path = "fixtures/pending_phase.rs"]
mod pending_phase;

use codelet_providers::codex::{
    history::CodexHistory,
    reconnect::{CodexRecoveryHandle, RecoveryStatus},
};
use fixture::{collect_request, model, Endpoint, Reply, TestResult};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::message::{Message, ToolResultContent, UserContent};
use rig::OneOrMany;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[tokio::test]
async fn scenario_offline_sessions_recover_the_original_turn_after_reception_returns() -> TestResult
{
    // @step Given a Codex session without network reception
    let endpoint = Endpoint::vacant().await?;
    let history = CodexHistory::default();
    let initial_model = model(&endpoint.url(), history.clone())?;
    let initial = serde_json::from_slice(
        &initial_model.prepare_request(
            CompletionRequestBuilder::new(initial_model.clone(), "inspect")
                .preamble("Test instructions".into())
                .build(),
        )?,
    )?;
    let call = json!({"type":"function_call","id":"fc_saved","call_id":"call_saved","name":"inspect","arguments":"{}"});
    history.commit(&initial, std::slice::from_ref(&call))?;
    let previous = history.snapshot()?;
    let recovery = CodexRecoveryHandle::default();
    let statuses = Arc::new(Mutex::new(Vec::new()));
    let recorded = statuses.clone();
    recovery.set_observer(Arc::new(move |status| {
        if let Ok(mut statuses) = recorded.lock() {
            statuses.push(status);
        }
    }));
    let model = model(&endpoint.url(), history.clone())?.with_recovery_handle(recovery.clone());
    let result = Message::User {
        content: OneOrMany::one(UserContent::tool_result_with_call_id(
            "fc_saved",
            "call_saved".into(),
            OneOrMany::one(ToolResultContent::text(
                "completed before reception was lost",
            )),
        )),
    };
    let request = CompletionRequestBuilder::new(model.clone(), result)
        .preamble("Test instructions".into())
        .messages(vec![Message::user("inspect")])
        .build();
    let expected: Value = serde_json::from_slice(&model.prepare_request(request.clone())?)?;
    let pending = collect_request(model, request);
    tokio::pin!(pending);
    let started = Instant::now();

    // @step When connection attempts fail for longer than seven seconds before reception returns
    // TEST-042: no waiting window; the turn must stay pending until recovery
    // reports its second offline attempt.
    codelet_test_helpers::pending_until("the second offline attempt", &mut pending, || {
        matches!(
            recovery.status(),
            Some(RecoveryStatus::WaitingForNetwork { attempt: 2 })
        )
    })
    .await
    .map_err(|_| "original turn terminated while the network was still offline")?;
    // The first network backoff is 5 s (providers codex reconnect), so the
    // second attempt starts at least that long after the first (a lower
    // bound, never a deadline).
    assert!(started.elapsed() >= Duration::from_secs(5));
    assert!(recovery.is_reconnecting());
    assert!(matches!(
        recovery.status(),
        Some(RecoveryStatus::WaitingForNetwork { attempt: 2 })
    ));
    assert_eq!(history.snapshot()?, previous);
    let server = endpoint.serve(Reply::Complete).await?;
    let observed = codelet_test_helpers::deadlock_guard("&mut pending", &mut pending).await??;

    // @step Then the original full request succeeds without a synthetic user message or transport change
    assert_eq!(observed.text, "Hello");
    assert_eq!(observed.finals.len(), 1);
    assert_eq!(observed.tools, 0);
    assert!(!recovery.is_reconnecting());
    assert!(matches!(recovery.status(), Some(RecoveryStatus::Recovered)));
    let requests = server.requests.lock().map_err(|_| "capture unavailable")?;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].body, expected);
    assert!(requests[0].headers.starts_with("POST /responses HTTP/1.1"));
    assert!(!requests[0]
        .headers
        .to_ascii_lowercase()
        .contains("upgrade:"));
    let input = requests[0].body["input"]
        .as_array()
        .ok_or("input missing")?;
    assert_eq!(input.len(), 3);
    assert_eq!(input[1], call);
    assert_eq!(input[2]["type"], "function_call_output");
    assert_eq!(input[2]["output"], "completed before reception was lost");
    assert_eq!(
        input.iter().filter(|item| item["role"] == "user").count(),
        1
    );
    assert!(requests[0].body.get("previous_response_id").is_none());
    assert_eq!(history.generation()?, 0);
    assert!(history.snapshot()?.is_some());
    let statuses = statuses.lock().map_err(|_| "status capture unavailable")?;
    assert!(statuses
        .iter()
        .any(|status: &String| status.to_ascii_lowercase().contains("waiting")));
    assert!(statuses
        .last()
        .is_some_and(|status| status.to_ascii_lowercase().contains("reconnected")));
    Ok(())
}

#[tokio::test]
async fn scenario_cancellation_drops_setup_reads_and_offline_backoff() -> TestResult {
    let mut servers = Vec::new();
    for reply in [Some(Reply::AwaitSetup), Some(Reply::AwaitBody), None] {
        // @step Given a Codex request awaiting setup or body data or offline backoff
        let endpoint = Endpoint::vacant().await?;
        let server = match reply {
            Some(reply) => Some(endpoint.serve(reply).await?),
            None => None,
        };
        let history = CodexHistory::default();
        let recovery = CodexRecoveryHandle::default();
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let recorded = statuses.clone();
        recovery.set_observer(Arc::new(move |status| {
            if let Ok(mut statuses) = recorded.lock() {
                statuses.push(status);
            }
        }));
        let model = model(&endpoint.url(), history.clone())?.with_recovery_handle(recovery.clone());
        let request = CompletionRequestBuilder::new(model.clone(), "Hello")
            .preamble("Test instructions".into())
            .build();
        let mut stream = model
            .stream(request)
            .await?
            .filter(attempt_events::is_content);
        // The selected network phase must still be pending once reached.
        pending_phase::pending_until(&mut stream, || match &server {
            Some(server) => Ok(server
                .requests
                .lock()
                .map_err(|_| "capture unavailable")?
                .len()
                == 1),
            None => Ok(recovery.is_reconnecting()),
        })
        .await?;
        if let Some(server) = &server {
            assert_eq!(
                server
                    .requests
                    .lock()
                    .map_err(|_| "capture unavailable")?
                    .len(),
                1
            );
        } else {
            assert!(recovery.is_reconnecting());
            assert!(matches!(
                recovery.status(),
                Some(RecoveryStatus::WaitingForNetwork { attempt: 1 })
            ));
        }

        // @step When the caller drops the pending request stream
        let cancelled = Instant::now();
        drop(stream);

        // @step Then cancellation clears reconnect state promptly and no later request occurs
        assert!(!recovery.is_reconnecting());
        assert_eq!(
            recovery.status(),
            reply.is_none().then_some(RecoveryStatus::Cancelled)
        );
        assert!(cancelled.elapsed() < Duration::from_millis(250));
        assert_eq!(history.snapshot()?, None);
        {
            let statuses = statuses.lock().map_err(|_| "status capture unavailable")?;
            if reply.is_none() {
                assert_eq!(statuses.len(), 2);
                assert_eq!(
                    statuses.last().map(String::as_str),
                    Some("Reconnection cancelled")
                );
            } else {
                assert!(statuses.is_empty());
            }
        }
        match server {
            Some(server) => {
                server.wait_for_close().await?;
                servers.push((server, 1));
            }
            None => servers.push((endpoint.serve(Reply::Complete).await?, 0)),
        }
    }
    // TEST-042: no waiting window; every stream was dropped and its recovery
    // reported cancelled above, so the request counts are final.
    for (server, expected_requests) in servers {
        assert_eq!(
            server
                .requests
                .lock()
                .map_err(|_| "capture unavailable")?
                .len(),
            expected_requests
        );
    }
    Ok(())
}
