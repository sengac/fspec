use crate::background_session::BackgroundSession;
use crate::subordinate_output::{RelayOutputError, RelaySubscription};
use codelet_rpc_types::StreamChunk;
use codelet_tools::bridge_relay::SubordinateChunkTx;
use tokio::task::JoinSet;
use uuid::Uuid;

pub fn spawn_forwarding(
    session: &BackgroundSession,
    root_parent_id: Uuid,
    convert: fn(&StreamChunk) -> serde_json::Value,
) -> Result<tokio::task::JoinHandle<()>, RelayOutputError> {
    let mut subscription = session.subscribe_to_relay()?;
    let subordinate_id = session.id;
    Ok(tokio::spawn(async move {
        let mut deliveries = JoinSet::new();
        let mut active: Vec<SubordinateChunkTx> = Vec::new();
        loop {
            codelet_tools::profile_scope!("spawn_subordinate_forwarding_task::recv_loop");
            let received = tokio::select! {
                received = subscription.recv() => received,
                Some(completed) = deliveries.join_next(), if !deliveries.is_empty() => {
                    match completed {
                        Ok(sender) => active.retain(|known| !known.same_channel(&sender)),
                        Err(error) => tracing::error!(%error, %subordinate_id, "subordinate relay delivery task failed"),
                    }
                    continue;
                }
            };
            let chunk = match received {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(error) => {
                    tracing::error!(%error, %subordinate_id, "BUG-217: subordinate forwarding stopped");
                    break;
                }
            };
            active.retain(|sender| !sender.is_closed());
            for sender in codelet_tools::get_subordinate_chunk_senders(root_parent_id) {
                if active.iter().any(|known| known.same_channel(&sender)) {
                    continue;
                }
                let reader = match subscription.fork() {
                    Ok(reader) => reader,
                    Err(error) => {
                        tracing::error!(%error, %subordinate_id, "cannot subscribe subordinate relay delivery");
                        return;
                    }
                };
                active.push(sender.clone());
                deliveries.spawn(deliver(
                    reader,
                    sender,
                    subordinate_id,
                    convert,
                    chunk.clone(),
                ));
            }
            tokio::task::yield_now().await;
        }
        while let Some(completed) = deliveries.join_next().await {
            if let Err(error) = completed {
                tracing::error!(%error, %subordinate_id, "subordinate relay delivery task failed");
            }
        }
    }))
}

async fn deliver(
    mut subscription: RelaySubscription,
    sender: SubordinateChunkTx,
    subordinate_id: Uuid,
    convert: fn(&StreamChunk) -> serde_json::Value,
    mut chunk: StreamChunk,
) -> SubordinateChunkTx {
    loop {
        let mut chunk_json = convert(&chunk);
        if let Some(object) = chunk_json.as_object_mut() {
            object.insert(
                "_relay_session_id".into(),
                serde_json::Value::String(subordinate_id.to_string()),
            );
        }
        if sender.send((subordinate_id, chunk_json)).await.is_err() {
            break;
        }
        let received = tokio::select! {
            _ = sender.closed() => break,
            received = subscription.recv() => received,
        };
        match received {
            Ok(Some(next)) => chunk = next,
            Ok(None) => break,
            Err(error) => {
                tracing::error!(%error, %subordinate_id, "subordinate relay delivery stopped");
                break;
            }
        }
    }
    sender
}
