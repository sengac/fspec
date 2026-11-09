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
    // BUG-465: this refresh has no caller to return to; a failed listing
    // or focus read is logged and that refresh is skipped.
    let sessions = match client.list_live_sessions(tarpc::context::current()).await {
        Ok(Ok(sessions)) => sessions,
        Ok(Err(error)) => {
            tracing::error!(%error, "cannot list live sessions for focus refresh");
            return;
        }
        Err(error) => {
            tracing::error!(%error, "cannot reach the daemon for focus refresh");
            return;
        }
    };
    for session in sessions {
        match client
            .get_work_unit_focus(tarpc::context::current(), session.clone())
            .await
        {
            Ok(Ok(focus)) => {
                let _ = sender.send((session, focus));
            }
            Ok(Err(error)) => {
                tracing::error!(session = %session.value, %error, "cannot read the session focus")
            }
            Err(error) => {
                tracing::error!(session = %session.value, %error, "cannot reach the daemon for the session focus")
            }
        }
    }
}
