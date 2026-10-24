use std::{sync::Arc, time::Duration};

use codelet_core::{persistence, session_manager_handle::SessionManagerHandle};
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;

mod common;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Environment(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl Environment {
    fn isolate(root: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let saved = ["FSPEC_USER_DIR", "ANTHROPIC_API_KEY"]
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
        Ok(Self(saved))
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        persistence::reset_stores_for_tests();
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

async fn settle(app: &mut App) {
    loop {
        while let Some(task) = app.next_pending_task() {
            assert!(task.await.is_ok());
        }
        let Some(action) = app.try_recv_action() else {
            break;
        };
        app.dispatch(action);
    }
}

async fn wait_for_card(app: &mut App, session: &SessionId, card: Option<&str>) -> TestResult {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            settle(app).await;
            if app
                .agent_view_store()
                .work_unit_context_for(session)
                .map(|context| context.id.as_str())
                == card
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    Ok(())
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn reconnect_snapshots_and_remote_changes_reach_real_tui_backends() -> TestResult {
    // @step Given a newly connected client observing two existing sessions
    let root = codelet_test_helpers::setup_temp_directory()?;
    let _environment = Environment::isolate(root.path())?;
    let (project, _) = common::temp_service();
    let manager = SessionManager::new_arc();
    let mut sessions = Vec::new();
    for name in ["focused", "cleared"] {
        let manifest = persistence::create_session_with_provider(
            name,
            project.path(),
            "anthropic/claude-opus-4-5",
        )?;
        manager
            .create_session_from_manifest(&manifest, "anthropic/claude-opus-4-5")
            .await?;
        sessions.push(SessionId::new(manifest.id.to_string()));
    }
    let handle: Arc<dyn SessionManagerHandle> = manager;
    let watcher = Arc::new(codelet_core::work_units::WorkUnitsWatcher::new(
        project.path(),
    )?);
    let service = Arc::new(SharedFspecService::with_session_manager(watcher, handle));
    let embedded = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    embedded
        .set_work_unit_focus(sessions[0].clone(), Some("AUTH-002".into()), "test".into())
        .await?;
    let (address, server) = common::start_ws_server(service).await;
    let websocket = Arc::new(WebSocketFspecBackend::connect(common::ws_url(address)).await?);
    let mut app = App::new(websocket);

    // @step When it receives authoritative card focus for one session and cleared focus for the other
    app.bootstrap().await?;
    settle(&mut app).await;
    wait_for_card(&mut app, &sessions[0], Some("AUTH-002")).await?;

    // @step Then only the first session has a card suffix and a board badge
    assert!(app
        .agent_view_store()
        .work_unit_context_for(&sessions[1])
        .is_none());
    assert_eq!(
        app.board_store().session_for("AUTH-002"),
        Some(&sessions[0])
    );
    let mut embedded_app = App::new(embedded.clone());
    embedded_app.bootstrap().await?;
    wait_for_card(&mut embedded_app, &sessions[0], Some("AUTH-002")).await?;
    embedded
        .set_work_unit_focus(sessions[0].clone(), None, "remote-clear".into())
        .await?;
    wait_for_card(&mut app, &sessions[0], None).await?;
    wait_for_card(&mut embedded_app, &sessions[0], None).await?;
    assert!(app.board_store().session_for("AUTH-002").is_none());
    let reconnect = Arc::new(WebSocketFspecBackend::connect(common::ws_url(address)).await?);
    let mut reconnect_app = App::new(reconnect);
    reconnect_app.bootstrap().await?;
    settle(&mut reconnect_app).await;
    assert!(reconnect_app
        .agent_view_store()
        .work_unit_context_for(&sessions[0])
        .is_none());
    assert!(reconnect_app
        .board_store()
        .session_for("AUTH-002")
        .is_none());

    // @step And an older initial snapshot cannot replace a newer live focus update
    let latest = embedded
        .set_work_unit_focus(sessions[0].clone(), Some("AUTH-002".into()), "newer".into())
        .await?;
    wait_for_card(&mut app, &sessions[0], Some("AUTH-002")).await?;
    app.dispatch(Action::WorkUnitFocusChanged(
        sessions[0].clone(),
        codelet_rpc_types::WorkUnitFocus {
            revision: latest.revision - 1,
            context: None,
            source: "old-snapshot".into(),
        },
    ));
    assert_eq!(
        app.board_store().session_for("AUTH-002"),
        Some(&sessions[0])
    );

    // @step Given an existing session focused on card "AUTH-001" and card "AUTH-002" on the board
    app.dispatch(Action::OpenAgentView(Some(sessions[1].clone())));
    // @step When the user enters card "AUTH-002"
    app.dispatch(Action::EnterWorkUnit("AUTH-002".into()));
    settle(&mut app).await;
    // @step Then the backend and all card displays identify "AUTH-002"
    assert_eq!(
        embedded
            .get_work_unit_focus(sessions[1].clone())
            .await?
            .context
            .map(|context| context.id),
        Some("AUTH-002".into())
    );
    assert_eq!(
        embedded
            .get_work_unit_context(sessions[1].clone())
            .await?
            .map(|context| context.id),
        Some("AUTH-002".into())
    );
    server.abort();
    Ok(())
}
