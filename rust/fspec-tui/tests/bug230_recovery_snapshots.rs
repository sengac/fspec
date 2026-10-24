use std::{sync::Arc, time::Duration};

use codelet_fspec_tui::{FspecBackend, WebSocketFspecBackend};
use codelet_rpc_server::Envelope;
use codelet_rpc_types::{SessionInfo, WorkUnitContext, WorkUnitFocus};
use futures::SinkExt;
use tokio::{net::TcpListener, sync::Notify};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn snapshot(context: Option<WorkUnitContext>, revision: u64) -> SessionInfo {
    SessionInfo {
        id: "snapshot-session".into(),
        name: "Recovery session".into(),
        status: "idle".into(),
        project: "/project".into(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: None,
        work_unit_focus: WorkUnitFocus {
            revision,
            context,
            source: "recovery".into(),
        },
    }
}

/// Feature: spec/features/session-focus-recovery-snapshots.feature
#[tokio::test]
async fn scenario_live_recovery_snapshots_deliver_explicit_cleared_activity() -> TestResult {
    // @step Given a WebSocket backend subscribed to session focus updates
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let ready = Arc::new(Notify::new());
    let server_ready = ready.clone();
    let advance = Arc::new(Notify::new());
    let server_advance = advance.clone();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await?;
        let mut websocket = tokio_tungstenite::accept_async(socket).await?;
        server_ready.notified().await;
        for info in [
            snapshot(
                Some(WorkUnitContext {
                    id: "AUTH-001".into(),
                    title: "Recovery card".into(),
                    status: "implementing".into(),
                }),
                4,
            ),
            snapshot(None, 5),
        ] {
            let revision = info.work_unit_focus.revision;
            let bytes = bincode::serialize(&Envelope::SessionSnapshot(vec![info]))?;
            websocket
                .send(tokio_tungstenite::tungstenite::Message::Binary(
                    bytes.into(),
                ))
                .await?;
            if revision == 4 {
                server_advance.notified().await;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });
    let backend =
        WebSocketFspecBackend::connect(url::Url::parse(&format!("ws://{address}/"))?).await?;
    let mut updates = backend.work_unit_focus_rx();

    // @step When the server sends a recovery snapshot with an active card followed by explicit cleared focus
    ready.notify_one();

    // @step Then the subscriber receives both focus states with their authoritative revisions
    let (session, active) = tokio::time::timeout(Duration::from_secs(1), updates.recv()).await??;
    assert_eq!(session.value, "snapshot-session");
    assert_eq!(active.revision, 4);
    assert_eq!(
        active.context.map(|context| context.id),
        Some("AUTH-001".into())
    );
    advance.notify_one();
    let (session, cleared) = tokio::time::timeout(Duration::from_secs(1), updates.recv()).await??;
    assert_eq!(session.value, "snapshot-session");
    assert_eq!(cleared.revision, 5);
    assert!(cleared.context.is_none());

    // @step And it does not need to reconnect or wait for another card change
    server.abort();
    Ok(())
}
