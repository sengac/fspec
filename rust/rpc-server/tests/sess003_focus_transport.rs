//! Feature: spec/features/session-work-unit-focus-transports.feature

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_core::{persistence, session_manager_handle::SessionManagerHandle};
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::{SessionEvent, SessionId, WorkUnitFocus};
use codelet_sessions::SessionManager;
use std::{sync::Arc, time::Duration};
use tarpc::context;
use tokio::{sync::broadcast, time::timeout};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn next_focus(
    receiver: &mut broadcast::Receiver<SessionEvent>,
) -> TestResult<(SessionId, WorkUnitFocus)> {
    timeout(Duration::from_secs(5), async {
        loop {
            if let SessionEvent::FocusChanged { session_id, focus } = receiver.recv().await? {
                return Ok((session_id, focus));
            }
        }
    })
    .await?
}

struct Environment {
    variables: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl Environment {
    fn isolate(root: &std::path::Path) -> TestResult<Self> {
        let variables = ["FSPEC_USER_DIR", "ANTHROPIC_API_KEY"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        std::env::set_var("FSPEC_USER_DIR", root);
        std::env::set_var("ANTHROPIC_API_KEY", "offline-focus-test");
        codelet_common::set_data_directory(root.to_owned())?;
        persistence::reset_stores_for_tests();
        std::fs::create_dir_all(root.join("cache"))?;
        std::fs::write(
            root.join("cache/models.json"),
            include_str!("../../sessions/tests/fixtures/prov101_models.json"),
        )?;
        Ok(Self { variables })
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        persistence::reset_stores_for_tests();
        for (key, previous) in &self.variables {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn scenario_embedded_and_websocket_clients_receive_focus_events_and_reconnect_snapshots(
) -> TestResult {
    // @step Given a real session manager shared by embedded and WebSocket clients
    let root = codelet_test_helpers::setup_temp_directory()?;
    let _environment = Environment::isolate(root.path())?;
    std::fs::create_dir_all(root.path().join("spec"))?;
    std::fs::write(
        root.path().join("spec/work-units.json"),
        serde_json::to_vec(&serde_json::json!({
            "workUnits": {
                "SESS-003": {"id":"SESS-003", "title":"Focus service", "type":"story", "status":"implementing"},
                "SESS-004": {"id":"SESS-004", "title":"Next focus", "type":"story", "status":"testing"}
            }
        }))?,
    )?;
    let manager = SessionManager::new_arc();
    let manifest = persistence::create_session_with_provider(
        "focus transport",
        root.path(),
        "anthropic/claude-opus-4-5",
    )?;
    manager
        .create_session_from_manifest(&manifest, "anthropic/claude-opus-4-5")
        .await?;
    let session_id = SessionId::new(manifest.id.to_string());
    let handle: Arc<dyn SessionManagerHandle> = manager;
    let watcher = Arc::new(WorkUnitsWatcher::new(root.path())?);
    let service = Arc::new(SharedFspecService::with_session_manager(watcher, handle));
    let embedded = EmbeddedTransport::new(tokio::runtime::Handle::current(), Arc::clone(&service));
    let embedded_client = embedded.client();
    let mut embedded_events = embedded.work_unit_focus_rx();
    let (address, _stats, server) = bind_and_serve("127.0.0.1:0", service).await?;
    let (socket_a, _) = tokio_tungstenite::connect_async(format!("ws://{address}")).await?;
    let (socket_b, _) = tokio_tungstenite::connect_async(format!("ws://{address}")).await?;
    let client_a = ws_client_connect(socket_a).await?;
    let client_b = ws_client_connect(socket_b).await?;
    let mut events_a = client_a.session_events_rx();
    let mut events_b = client_b.session_events_rx();
    for client in [&client_a, &client_b] {
        let mut snapshots = client.session_snapshot_rx();
        let snapshot = timeout(Duration::from_secs(5), snapshots.recv()).await??;
        assert!(snapshot.iter().any(|info| info.id == session_id.value));
    }
    let initial = embedded_client
        .get_work_unit_focus(context::current(), session_id.clone())
        .await??;
    assert_eq!(initial, WorkUnitFocus::default());

    // @step When a client selects switches and clears session focus
    let selected = embedded_client
        .set_work_unit_focus(
            context::current(),
            session_id.clone(),
            Some("SESS-003".into()),
            "board".into(),
        )
        .await??;
    let switched = client_a
        .client()
        .set_work_unit_focus(
            context::current(),
            session_id.clone(),
            Some("SESS-004".into()),
            "tool".into(),
        )
        .await??;
    let cleared = client_b
        .client()
        .set_work_unit_focus(
            context::current(),
            session_id.clone(),
            None,
            "explicit".into(),
        )
        .await??;

    // @step Then every subscribed client receives the same revisioned focus changes
    for (position, expected) in [selected.clone(), switched.clone(), cleared.clone()]
        .into_iter()
        .enumerate()
    {
        assert_eq!(expected.revision, position as u64 + 1);
        let embedded_event = timeout(Duration::from_secs(5), embedded_events.recv()).await??;
        assert_eq!(embedded_event, (session_id.clone(), expected.clone()));
        assert_eq!(
            next_focus(&mut events_a).await?,
            (session_id.clone(), expected.clone())
        );
        assert_eq!(
            next_focus(&mut events_b).await?,
            (session_id.clone(), expected)
        );
    }
    assert_eq!(
        selected.context.as_ref().map(|card| card.id.as_str()),
        Some("SESS-003")
    );
    assert_eq!(
        switched.context.as_ref().map(|card| card.id.as_str()),
        Some("SESS-004")
    );
    assert!(cleared.context.is_none());
    assert_eq!(cleared.source, "explicit");

    // @step And a reconnecting client receives the authoritative cleared snapshot
    drop(client_b);
    let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}")).await?;
    let reconnected = ws_client_connect(socket).await?;
    let mut snapshots = reconnected.session_snapshot_rx();
    let snapshot = timeout(Duration::from_secs(5), snapshots.recv()).await??;
    let info = snapshot
        .iter()
        .find(|info| info.id == session_id.value)
        .ok_or("session absent from reconnect snapshot")?;
    assert_eq!(info.work_unit_focus, cleared);
    let mut legacy = serde_json::to_value(info)?;
    legacy
        .as_object_mut()
        .ok_or("session snapshot object")?
        .remove("work_unit_focus");
    let legacy: codelet_rpc_types::SessionInfo = serde_json::from_value(legacy)?;
    assert_eq!(legacy.work_unit_focus, WorkUnitFocus::default());
    assert_eq!(
        reconnected
            .client()
            .get_work_unit_focus(context::current(), session_id.clone())
            .await??,
        cleared
    );
    let embedded_snapshot = embedded_client
        .list_sessions(
            context::current(),
            root.path().to_string_lossy().into_owned(),
        )
        .await?;
    assert_eq!(
        embedded_snapshot
            .iter()
            .find(|info| info.id == session_id.value)
            .ok_or("embedded session snapshot")?
            .work_unit_focus,
        cleared
    );

    // @step And an older snapshot cannot supersede a newer focus revision
    let mut projection = selected;
    assert!(projection.apply_if_newer(cleared.clone()));
    assert!(!projection.apply_if_newer(switched));
    assert!(!projection.apply_if_newer(cleared.clone()));
    assert!(!projection.apply_if_newer(initial));
    assert_eq!(projection, cleared);
    server.abort();
    Ok(())
}
