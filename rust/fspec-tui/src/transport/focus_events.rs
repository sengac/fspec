use codelet_rpc::FspecServiceClient;
use codelet_rpc_types::{SessionEvent, SessionId, SessionInfo, WorkUnitFocus};
use tokio::sync::broadcast;

pub(super) fn subscribe(
    mut events: broadcast::Receiver<SessionEvent>,
    mut snapshots: broadcast::Receiver<Vec<SessionInfo>>,
    client: FspecServiceClient,
) -> broadcast::Receiver<(SessionId, WorkUnitFocus)> {
    let (sender, receiver) = broadcast::channel(16);
    tokio::spawn(async move {
        loop {
            tokio::select! {
                () = sender.closed() => break,
                snapshot = snapshots.recv() => match snapshot {
                    Ok(sessions) => {
                        for session in sessions {
                            let _ = sender.send((SessionId::new(session.id), session.work_unit_focus));
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => refresh(&client, &sender).await,
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                event = events.recv() => match event {
                    Ok(SessionEvent::FocusChanged { session_id, focus }) => {
                        if sender.send((session_id, focus)).is_err() { break; }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        refresh(&client, &sender).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    });
    receiver
}

async fn refresh(
    client: &FspecServiceClient,
    sender: &broadcast::Sender<(SessionId, WorkUnitFocus)>,
) {
    if let Ok(sessions) = client.list_live_sessions(tarpc::context::current()).await {
        for session in sessions {
            if let Ok(Ok(focus)) = client
                .get_work_unit_focus(tarpc::context::current(), session.clone())
                .await
            {
                let _ = sender.send((session, focus));
            }
        }
    }
}
