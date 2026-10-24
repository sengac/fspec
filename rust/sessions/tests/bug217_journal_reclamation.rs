//! Feature: spec/features/subordinate-journal-prefix-reclamation.feature

mod journal {
    include!("../src/subordinate_output.rs");

    pub fn retained_bytes(subscription: &RelaySubscription) -> Result<u64, RelayOutputError> {
        let journal = subscription
            .journal
            .lock()
            .map_err(|_| RelayOutputError::Poisoned)?;
        Ok(journal.file.metadata()?.len())
    }
}

use codelet_rpc_types::StreamChunk;
use journal::{RelayOutput, RelaySubscription};
use tokio::sync::broadcast;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn event(index: usize) -> StreamChunk {
    StreamChunk::text(format!("{index}:{}", "x".repeat(2048)))
}

async fn receive(subscription: &mut RelaySubscription, index: usize) -> TestResult {
    let chunk = subscription.recv().await?.ok_or("journal closed")?;
    assert!(
        matches!(chunk, StreamChunk::Text { text, .. } if text == format!("{index}:{}", "x".repeat(2048)))
    );
    Ok(())
}

#[tokio::test]
async fn scenario_continuously_lagging_subscribers_retain_only_their_unread_suffix_and_bounded_slack(
) -> TestResult {
    // @step Given two journal subscribers continuously trail publication by different small amounts
    let output = RelayOutput::default();
    let (notifications, _) = broadcast::channel(256);
    let mut fast = output.subscribe(notifications.subscribe())?;
    let mut slow = fast.fork()?;
    // @step When sustained output grows far beyond their unread backlog
    for index in 0..2000 {
        output.append(&event(index));
        if index >= 1 {
            receive(&mut fast, index - 1).await?;
        }
        if index >= 3 {
            receive(&mut slow, index - 3).await?;
        }
    }
    // @step Then retained journal storage stays bounded by unread data and compaction slack
    let retained = journal::retained_bytes(&slow)?;
    assert!(
        retained < 128 * 1024,
        "retained {retained} bytes for three unread records"
    );
    // @step And both subscribers receive every remaining event exactly once in order
    receive(&mut fast, 1999).await?;
    for index in 1997..2000 {
        receive(&mut slow, index).await?;
    }
    drop(notifications);
    assert!(fast.recv().await?.is_none());
    assert!(slow.recv().await?.is_none());
    Ok(())
}

#[tokio::test]
async fn scenario_cancelling_the_slowest_subscriber_allows_its_consumed_prefix_to_be_reclaimed(
) -> TestResult {
    // @step Given a slow journal subscriber retains a large prefix already read by another subscriber
    let output = RelayOutput::default();
    let (notifications, _) = broadcast::channel(256);
    let mut fast = output.subscribe(notifications.subscribe())?;
    let slow = output.subscribe(notifications.subscribe())?;
    for index in 0..200 {
        output.append(&event(index));
    }
    for index in 0..199 {
        receive(&mut fast, index).await?;
    }
    assert!(journal::retained_bytes(&fast)? > 256 * 1024);
    // @step When the slow subscriber is cancelled and another event is published
    drop(slow);
    output.append(&event(200));
    // @step Then the retained journal no longer includes that obsolete prefix
    let retained = journal::retained_bytes(&fast)?;
    assert!(
        retained < 128 * 1024,
        "obsolete prefix retained {retained} bytes"
    );
    // @step And the remaining subscriber receives its unread events in order
    receive(&mut fast, 199).await?;
    receive(&mut fast, 200).await?;
    drop(notifications);
    assert!(fast.recv().await?.is_none());
    Ok(())
}
