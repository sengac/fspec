//! Feature: spec/features/codex-metadata-presentation.feature

#[path = "support/bug241_metadata.rs"]
mod support;
#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod transport;

use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::json;
use support::{buffering, notices, reasoning, verification, waiting, Fixture, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn routine_metadata_remains_diagnostic_only() -> TestResult {
    // @step Given a Codex session receiving headers rate limits and moderation metadata
    let mut fixture = Fixture::new().await?;
    let handle = fixture.codex()?.metadata.clone();
    // @step When the diagnostic metadata changes during a turn
    handle.apply_event(&json!({"type":"response.metadata", "headers":{
        "x-request-id":"req-241", "openai-model":"serving-model",
        "x-reasoning-included":"true", "x-codex-primary-used-percent":"12.5"
    }, "metadata":{"openai_chatgpt_moderation_metadata":{"private":"PRIVATE-marker"}}}));
    handle.apply_event(
        &json!({"type":"codex.rate_limits", "metered_limit_name":"codex",
        "rate_limits":{"primary":{"used_percent":15}}}),
    );
    // @step Then session diagnostics update without user notifications
    assert!(notices(&fixture.drain()).is_empty());
    assert_eq!(handle.status().request_id.as_deref(), Some("req-241"));
    assert!(handle.status().moderation_received);
    assert_eq!(
        handle.status().rate_limits["codex"]
            .primary
            .as_ref()
            .map(|rate| rate.used_percent),
        Some(15.0)
    );
    assert!(!format!("{:?}", handle.status()).contains("PRIVATE"));
    // @step And non Codex sessions emit no Codex presentation
    fixture
        .session
        .prepare_provider_status(&fixture.extensions, "openai");
    handle.apply_event(&buffering());
    assert!(fixture.session.provider_metadata_status().is_none());
    assert!(fixture.drain().is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn buffering_is_temporary_waiting_state() -> TestResult {
    // @step Given a Codex turn receiving valid buffering observations
    let mut fixture = Fixture::new().await?;
    let mut supervisor = fixture.session.subscribe_to_stream();
    let mut relay = fixture.session.subscribe_to_relay()?;
    let handle = fixture.codex()?.metadata.clone();
    let server = transport::Server::start(
        vec![[
            transport::event(buffering()),
            transport::event(buffering()),
            transport::text(),
            transport::event(buffering()),
            transport::terminal("completed", None, None),
        ]
        .concat()],
        false,
    )
    .await?;
    let model = server.models()?.0.with_metadata_handle(handle);
    // @step When repeated observations arrive before and after assistant output
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "Hello").build())
        .await?;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while stream.next().await.transpose()?.is_some() {}
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await??;
    let chunks = fixture.drain();
    // @step Then waiting is shown once and dismissed when output begins
    assert_eq!(
        waiting(&chunks),
        vec![
            Some("Giving this request a little extra thought".into()),
            None
        ]
    );
    // @step And buffering never becomes transcript notifications or changes the selected model
    assert!(notices(&chunks).is_empty());
    assert!(fixture.session.get_buffered_output(100).is_empty());
    assert!(matches!(
        supervisor.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(25), relay.recv())
            .await
            .is_err()
    );
    for chunk in &chunks {
        let wire = codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value(chunk);
        assert_eq!(wire["type"], "providerWaiting");
    }
    {
        let requests = server.requests.lock().map_err(|_| "request lock")?;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0]["model"], "shared-model");
    }
    support::verify_retry_waiting(&mut fixture).await?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn old_streams_cannot_revive_waiting_state() -> TestResult {
    // @step Given an old Codex stream and a newer active turn
    let mut fixture = Fixture::new().await?;
    let server = transport::Server::start(vec![
        [transport::event(buffering()), reasoning("thinking"),
         transport::event(json!({"type":"response.metadata", "safety_buffering":{"use_cases":[], "reasons":[], "retry_model":"stale-model"}})),
         reasoning("old update")].concat(),
        [transport::event(buffering()), reasoning("thinking")].concat()
    ], false).await?;
    let model = server
        .models()?
        .0
        .with_metadata_handle(fixture.codex()?.metadata.clone());
    let mut old = model
        .stream(CompletionRequestBuilder::new(model.clone(), "Old").build())
        .await?;
    for _ in 0..2 {
        tokio::time::timeout(std::time::Duration::from_secs(3), old.next())
            .await?
            .ok_or("old stream ended")??;
    }
    fixture.extensions.begin_turn();
    let mut current = model
        .stream(CompletionRequestBuilder::new(model.clone(), "New").build())
        .await?;
    for _ in 0..2 {
        tokio::time::timeout(std::time::Duration::from_secs(3), current.next())
            .await?
            .ok_or("new stream ended")??;
    }
    fixture.drain();
    // @step When old buffering events and stream cleanup arrive
    tokio::time::timeout(std::time::Duration::from_secs(3), old.next())
        .await?
        .ok_or("old update missing")??;
    drop(old);
    // @step Then the newer waiting state is unchanged
    assert!(fixture.codex()?.metadata.status().buffering);
    assert!(fixture.codex()?.metadata.status().faster_model.is_none());
    assert!(waiting(&fixture.drain()).is_empty());
    // @step And completion failure cancellation and reset clear current waiting
    drop(current);
    assert!(!fixture.codex()?.metadata.status().buffering);
    assert_eq!(waiting(&fixture.drain()), vec![None]);
    fixture.extensions.begin_turn();
    fixture.codex()?.metadata.apply_event(&buffering());
    fixture.extensions.reset()?;
    assert!(!fixture.codex()?.metadata.status().buffering);
    assert!(!fixture.codex()?.metadata.status().moderation_received);
    fixture.extensions.begin_turn();
    fixture.codex()?.metadata.apply_event(&buffering());
    fixture.session.interrupt();
    assert!(!fixture.codex()?.metadata.status().buffering);
    fixture.extensions.begin_turn();
    let lazy_server = transport::Server::start(
        vec![[transport::event(buffering()), reasoning("held")].concat()],
        false,
    )
    .await?;
    let lazy_model = lazy_server
        .models()?
        .0
        .with_metadata_handle(fixture.codex()?.metadata.clone());
    let mut held = lazy_model
        .stream(CompletionRequestBuilder::new(lazy_model.clone(), "Held").build())
        .await?;
    for _ in 0..2 {
        tokio::time::timeout(std::time::Duration::from_secs(3), held.next())
            .await?
            .ok_or("held stream ended")??;
    }
    let unpolled = lazy_model
        .stream(CompletionRequestBuilder::new(lazy_model.clone(), "Never polled").build())
        .await?;
    drop(unpolled);
    drop(held);
    assert!(!fixture.codex()?.metadata.status().buffering);
    support::verify_callback_ordering(&mut fixture)?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn verification_remains_actionable_and_turn_scoped() -> TestResult {
    // @step Given a Codex session receiving verification and recovery events
    let mut fixture = Fixture::new().await?;
    let handle = fixture.codex()?.metadata.clone();
    // @step When verification repeats alongside unrelated metadata across turns
    for turn in 0..2 {
        if turn > 0 {
            fixture.extensions.begin_turn();
        }
        handle.apply_event(&verification());
        handle.apply_event(&verification());
        handle.apply_event(
            &json!({"type":"response.metadata", "headers":{"x-request-id":format!("req-{turn}")}}),
        );
        // @step Then verification is announced once per turn without stale metadata labels
        let messages = notices(&fixture.drain());
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("verification"));
        assert!(!messages[0].contains("buffering"));
        assert!(!messages[0].contains("moderation"));
    }
    // @step And recovery notifications remain available
    let server = transport::Server::start(
        vec![String::new(), transport::terminal("completed", None, None)],
        false,
    )
    .await?;
    let model = server
        .models()?
        .0
        .with_recovery_handle(fixture.codex()?.recovery.clone());
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "Hello").build())
        .await?;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while stream.next().await.transpose()?.is_some() {}
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await??;
    let messages = notices(&fixture.drain());
    assert!(messages
        .iter()
        .any(|message| message.contains("Reconnecting")));
    assert!(messages
        .iter()
        .any(|message| message.contains("Reconnected")));
    assert!(fixture.session.provider_metadata_status().is_some());
    support::verify_observer_deduplication()?;
    Ok(())
}
