//! Feature: spec/features/codex-interruption-status.feature

#[path = "fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "fixtures/prov160.rs"]
pub mod fixture;

use codelet_providers::codex::{
    history::CodexHistory,
    reconnect::{CodexRecoveryHandle, RecoveryStatus},
};
use fixture::{baseline, model, Endpoint, Reply, TestResult};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder},
    streaming::StreamedAssistantContent,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

fn observe(handle: &CodexRecoveryHandle) -> Arc<Mutex<Vec<String>>> {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let recorded = messages.clone();
    handle.set_observer(Arc::new(move |message| {
        if let Ok(mut messages) = recorded.lock() {
            messages.push(message);
        }
    }));
    messages
}

#[tokio::test]
async fn scenario_ordinary_setup_and_body_cancellation_remain_silent() -> TestResult {
    // @step Given a healthy Codex request awaiting initial setup or body data
    for reply in [Reply::AwaitSetup, Reply::AwaitBody] {
        let endpoint = Endpoint::vacant().await?;
        let server = endpoint.serve(reply).await?;
        let handle = CodexRecoveryHandle::default();
        let messages = observe(&handle);
        let model =
            model(&endpoint.url(), CodexHistory::default())?.with_recovery_handle(handle.clone());
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "Hello").build())
            .await?
            .filter(attempt_events::is_content);
        assert!(
            tokio::time::timeout(Duration::from_millis(150), stream.next())
                .await
                .is_err()
        );
        assert_eq!(handle.status(), None);
        assert_eq!(
            server
                .requests
                .lock()
                .map_err(|_| "requests poisoned")?
                .len(),
            1
        );

        // @step When the caller drops the pending stream before recovery begins
        drop(stream);

        // @step Then the connection closes without a recovery status or notification
        server.wait_for_close().await?;
        assert_eq!(handle.status(), None);
        assert!(!handle.is_reconnecting());
        assert!(messages.lock().map_err(|_| "messages poisoned")?.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn scenario_active_recovery_cancellation_is_reported_exactly_once() -> TestResult {
    // @step Given a Codex request waiting offline or retrying an interrupted stream
    for offline in [true, false] {
        let endpoint = Endpoint::vacant().await?;
        let server = if offline {
            None
        } else {
            Some(
                baseline::Server::start(
                    vec![
                        baseline::text(),
                        baseline::terminal("completed", Some(true), None),
                    ],
                    false,
                )
                .await?,
            )
        };
        let url = server
            .as_ref()
            .map_or_else(|| endpoint.url(), |server| server.url.clone());
        let handle = CodexRecoveryHandle::default();
        let messages = observe(&handle);
        let model = model(&url, CodexHistory::default())?.with_recovery_handle(handle.clone());
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "Hello").build())
            .await?
            .filter(attempt_events::is_content);
        if !offline {
            stream.next().await.ok_or("missing partial text")??;
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(150), stream.next())
                .await
                .is_err()
        );
        assert_eq!(
            handle.status(),
            Some(if offline {
                RecoveryStatus::WaitingForNetwork { attempt: 1 }
            } else {
                RecoveryStatus::RetryingStream { attempt: 1 }
            })
        );

        // @step When the caller drops the stream during active recovery
        drop(stream);

        // @step Then exactly one cancellation notification ends recovery without a later request
        assert_eq!(handle.status(), Some(RecoveryStatus::Cancelled));
        assert!(!handle.is_reconnecting());
        let offline_server = if offline {
            Some(endpoint.serve(Reply::Complete).await?)
        } else {
            None
        };
        tokio::time::sleep(Duration::from_secs(6)).await;
        let messages = messages.lock().map_err(|_| "messages poisoned")?;
        assert_eq!(messages.len(), 2);
        assert_eq!(
            messages.last().map(String::as_str),
            Some("Reconnection cancelled")
        );
        if let Some(server) = server {
            assert_eq!(
                server
                    .requests
                    .lock()
                    .map_err(|_| "requests poisoned")?
                    .len(),
                1
            );
        }
        if let Some(server) = offline_server {
            assert!(server
                .requests
                .lock()
                .map_err(|_| "requests poisoned")?
                .is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_terminal_completion_never_becomes_cancellation() -> TestResult {
    // @step Given a healthy or recovering Codex response completing with text or tools
    let handle = CodexRecoveryHandle::default();
    for recovering in [true, false] {
        for with_tool in [false, true] {
            let messages = observe(&handle);
            let body = [
                baseline::text(),
                if with_tool {
                    baseline::tool()
                } else {
                    String::new()
                },
                baseline::terminal("completed", Some(true), None),
            ]
            .concat();
            let bodies = if recovering {
                vec![baseline::text(), body]
            } else {
                vec![body]
            };
            let server = baseline::Server::start(bodies, false).await?;
            let model =
                model(&server.url, CodexHistory::default())?.with_recovery_handle(handle.clone());
            let mut stream = model
                .stream(CompletionRequestBuilder::new(model.clone(), "Hello").build())
                .await?;

            // @step When the caller drops the stream immediately after final or tool completion
            tokio::time::timeout(Duration::from_secs(3), async {
                while let Some(event) = stream.next().await {
                    match event? {
                        StreamedAssistantContent::ToolCall(_) if with_tool => {
                            return Ok::<_, Box<dyn std::error::Error>>(())
                        }
                        StreamedAssistantContent::Final(_) if !with_tool => {
                            return Ok::<_, Box<dyn std::error::Error>>(())
                        }
                        _ => {}
                    }
                }
                Err("missing terminal completion".into())
            })
            .await??;
            drop(stream);

            // @step Then completion keeps its terminal recovery status without cancellation
            assert!(!handle.is_reconnecting());
            assert_eq!(
                handle.status(),
                if recovering {
                    Some(RecoveryStatus::Recovered)
                } else {
                    None
                }
            );
            let messages = messages.lock().map_err(|_| "messages poisoned")?;
            if recovering {
                assert_eq!(messages.len(), 2);
                assert_eq!(messages.last().map(String::as_str), Some("✓ Reconnected"));
            } else {
                assert!(messages.is_empty());
            }
        }
    }
    Ok(())
}
