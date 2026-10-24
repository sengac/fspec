//! Feature: spec/features/independent-subordinate-relay-progress.feature

#[path = "support/bug217_forwarding.rs"]
mod support;

use codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::subordinate_forwarding::spawn_forwarding;
use codelet_tools::bridge_relay::{register_subordinate_chunk_channel, SubordinateChunkRx};
use serde_json::Value;
use std::time::Duration;
use support::{Fixture, TestResult};

async fn next(receiver: &mut SubordinateChunkRx) -> Result<Value, Box<dyn std::error::Error>> {
    let (_, chunk) = tokio::time::timeout(Duration::from_secs(3), receiver.recv())
        .await?
        .ok_or("relay closed")?;
    Ok(chunk)
}

async fn drain(receiver: &mut SubordinateChunkRx, child: uuid::Uuid) -> TestResult {
    for index in 0..600 {
        let chunk = next(receiver).await?;
        assert_eq!(chunk["_relay_session_id"], child.to_string());
        if index == 100 {
            assert_eq!(chunk["type"], "done");
        } else {
            assert_eq!(chunk["text"], format!("independent-{index}"));
        }
    }
    assert!(receiver.try_recv().is_err());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_a_stalled_relay_does_not_block_a_healthy_relay_or_lose_its_own_backlog(
) -> TestResult {
    // @step Given a subordinate has a stalled relay and a healthy relay on its root parent
    let fixture = Fixture::new().await?;
    let mut stalled = register_subordinate_chunk_channel(fixture.root);
    let mut healthy = register_subordinate_chunk_channel(fixture.root);
    // @step When more events than a relay channel can hold are emitted including completion
    fixture.burst("independent").await?;
    // @step Then the healthy relay receives every event exactly once in order
    drain(&mut healthy, fixture.child).await?;
    assert_eq!(stalled.len(), 256);
    // @step And disconnecting the healthy relay releases its subscription without new output
    drop(healthy);
    let session = fixture.session()?;
    tokio::time::timeout(Duration::from_secs(3), async {
        while session.supervisor_broadcast.receiver_count() != 2 {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    // @step And the stalled relay later receives the same ordered events without gaps
    drain(&mut stalled, fixture.child).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_cancelling_the_forwarding_owner_releases_blocked_relay_subscriptions(
) -> TestResult {
    // @step Given an independently owned forwarder has a relay with a full bounded queue
    let fixture = Fixture::new().await?;
    let session = fixture.session()?;
    let baseline = session.supervisor_broadcast.receiver_count();
    let root = uuid::Uuid::new_v4();
    let relay = register_subordinate_chunk_channel(root);
    let owner = spawn_forwarding(&session, root, stream_chunk_to_json_value)?;
    fixture.burst("independent").await?;
    tokio::time::timeout(Duration::from_secs(3), async {
        while relay.len() != 256 {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    // @step When the forwarding owner is cancelled
    owner.abort();
    let _ = owner.await;
    // @step Then all of its relay subscriptions are released while the relay stays connected
    tokio::time::timeout(Duration::from_secs(3), async {
        while session.supervisor_broadcast.receiver_count() != baseline {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    assert_eq!(relay.len(), 256);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_late_relays_receive_new_output_without_replaying_already_forwarded_history(
) -> TestResult {
    // @step Given one relay has received subordinate output
    let fixture = Fixture::new().await?;
    let session = fixture.session()?;
    let root = uuid::Uuid::new_v4();
    let owner = spawn_forwarding(&session, root, stream_chunk_to_json_value)?;
    let mut first = register_subordinate_chunk_channel(root);
    session.handle_output(StreamChunk::text("earlier".into()));
    assert_eq!(next(&mut first).await?["text"], "earlier");
    // @step When a second relay connects before new output is emitted
    let mut second = register_subordinate_chunk_channel(root);
    session.handle_output(StreamChunk::text("new".into()));
    // @step Then both relays receive the new output without replaying earlier output
    assert_eq!(next(&mut first).await?["text"], "new");
    assert_eq!(next(&mut second).await?["text"], "new");
    // @step And destroying the subordinate drains completion to both relays and closes forwarding
    session.handle_output(StreamChunk::done());
    fixture
        .manager
        .destroy_session(&fixture.child.to_string())?;
    drop(session);
    assert_eq!(next(&mut first).await?["type"], "done");
    assert_eq!(next(&mut second).await?["type"], "done");
    tokio::time::timeout(Duration::from_secs(3), owner).await??;
    assert!(first.try_recv().is_err());
    assert!(second.try_recv().is_err());
    Ok(())
}
