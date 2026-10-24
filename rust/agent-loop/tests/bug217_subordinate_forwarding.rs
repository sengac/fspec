//! Feature: spec/features/subordinate-forwarding-lag-recovery.feature

#[path = "support/bug217_forwarding.rs"]
mod support;

use codelet_rpc_types::StreamChunk;
use codelet_tools::bridge_relay::{register_subordinate_chunk_channel, SubordinateChunkRx};
use serde_json::Value;
use std::time::Duration;
use support::{Fixture, TestResult};

async fn next(receiver: &mut SubordinateChunkRx) -> Result<Value, Box<dyn std::error::Error>> {
    let (_, chunk) = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await?
        .ok_or("relay closed")?;
    Ok(chunk)
}

async fn assert_burst(receiver: &mut SubordinateChunkRx, prefix: &str, child: &str) -> TestResult {
    for index in 0..600 {
        let chunk = next(receiver).await?;
        assert_eq!(chunk["_relay_session_id"], child);
        if index == 100 {
            assert_eq!(
                chunk["type"], "done",
                "terminal event was lost or reordered"
            );
        } else {
            assert_eq!(chunk["text"], format!("{prefix}-{index}"));
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_a_stalled_subordinate_forwarder_replays_evicted_terminal_events_in_order(
) -> TestResult {
    // @step Given a subordinate forwarding to its root parent's relay
    let fixture = Fixture::new().await?;
    let mut relay = register_subordinate_chunk_channel(fixture.root);
    let mut observer = fixture.session()?.subscribe_to_relay()?;
    // @step When a stalled bounded broadcast receiver is overrun with text and terminal events
    fixture.burst("first").await?;
    // @step Then the relay receives every subordinate event exactly once in original order
    assert_burst(&mut relay, "first", &fixture.child.to_string()).await?;
    assert!(relay.try_recv().is_err());
    for index in 0..600 {
        let event = observer.recv().await?.ok_or("observer closed")?;
        if index == 100 {
            assert!(matches!(event, StreamChunk::Done));
        } else {
            assert!(
                matches!(event, StreamChunk::Text { text, .. } if text == format!("first-{index}"))
            );
        }
    }
    // @step And a subsequent burst is delivered without replaying earlier events
    fixture.burst("second").await?;
    assert_burst(&mut relay, "second", &fixture.child.to_string()).await?;
    assert!(relay.try_recv().is_err());
    fixture.burst("third").await?;
    assert_burst(&mut relay, "third", &fixture.child.to_string()).await?;
    for prefix in ["second", "third"] {
        for index in 0..600 {
            let event = observer.recv().await?.ok_or("observer closed")?;
            if index == 100 {
                assert!(matches!(event, StreamChunk::Done));
            } else {
                assert!(
                    matches!(event, StreamChunk::Text { text, .. } if text == format!("{prefix}-{index}"))
                );
            }
        }
    }
    assert!(relay.try_recv().is_err());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_relay_output_applies_bounded_backpressure_and_survives_disconnection(
) -> TestResult {
    // @step Given a subordinate forwarding to a relay that is not draining output
    let fixture = Fixture::new().await?;
    let relay = register_subordinate_chunk_channel(fixture.root);
    // @step When more events are emitted than the relay channel can hold
    for _ in 0..600 {
        let session = fixture.session()?;
        tokio::spawn(async move { session.handle_output(StreamChunk::text("pending".into())) })
            .await?;
    }
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    // @step Then the relay queue remains bounded
    assert!(relay.len() <= 256, "unbounded relay queue: {}", relay.len());
    // @step When the relay disconnects and a replacement relay subscribes
    drop(relay);
    let mut replacement = register_subordinate_chunk_channel(fixture.root);
    fixture
        .session()?
        .handle_output(StreamChunk::text("replacement-marker".into()));
    // @step Then new subordinate output reaches the replacement relay
    loop {
        let chunk = next(&mut replacement).await?;
        if chunk["text"] == "replacement-marker" {
            assert_eq!(chunk["_relay_session_id"], fixture.child.to_string());
            break;
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_destroying_a_subordinate_drains_terminal_output_and_closes_its_subscription(
) -> TestResult {
    // @step Given a subordinate with a stalled forwarding subscription
    let fixture = Fixture::new().await?;
    let mut relay = register_subordinate_chunk_channel(fixture.root);
    let session = fixture.session()?;
    let mut subscription = session.subscribe_to_relay()?;
    let weak = std::sync::Arc::downgrade(&session);
    // @step When terminal output is emitted and the subordinate is destroyed
    let manager = fixture.manager.clone();
    let child = fixture.child;
    tokio::spawn(async move {
        session.handle_output(StreamChunk::done());
        manager.destroy_session(&child.to_string())
    })
    .await??;
    // @step Then the relay receives the pending terminal output
    let terminal = next(&mut relay).await?;
    assert_eq!(terminal["type"], "done");
    // @step And the forwarding subscription closes after draining
    assert!(matches!(
        subscription.recv().await?,
        Some(StreamChunk::Done)
    ));
    assert!(
        tokio::time::timeout(Duration::from_secs(5), subscription.recv())
            .await??
            .is_none()
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        while weak.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_cancelling_a_subscription_releases_its_bounded_receiver() -> TestResult {
    // @step Given a subordinate with an active forwarding subscription
    let fixture = Fixture::new().await?;
    let session = fixture.session()?;
    let baseline = session.supervisor_broadcast.receiver_count();
    let subscription = session.subscribe_to_relay()?;
    assert_eq!(session.supervisor_broadcast.receiver_count(), baseline + 1);
    // @step When the forwarding subscription is cancelled
    let task = tokio::spawn(async move {
        let mut subscription = subscription;
        subscription.recv().await
    });
    task.abort();
    let _ = task.await;
    // @step Then its broadcast receiver is released without keeping the session alive
    assert_eq!(session.supervisor_broadcast.receiver_count(), baseline);
    let weak = std::sync::Arc::downgrade(&session);
    fixture
        .manager
        .destroy_session(&fixture.child.to_string())?;
    drop(session);
    assert!(weak.upgrade().is_none());
    Ok(())
}
