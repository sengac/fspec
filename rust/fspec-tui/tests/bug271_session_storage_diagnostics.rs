//! Feature: spec/features/session-storage-diagnostics.feature

use codelet_fspec_tui::{Action, App};
use crossterm::event::KeyCode;
use serial_test::serial;

#[path = "common/session_storage_fixture.rs"]
mod fixture;
use fixture::{compact, creation_actions, drain, key, pages, screen, StorageFixture};

const DAMAGED_ID: &str = "b8c05474-d2ef-4bb9-83f6-899278219946";
const OTHER_ID: &str = "e374b804-fc07-4bae-b8e1-2ce5593b1d23";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn startup_warnings_are_visible_without_a_session() -> Result<(), Box<dyn std::error::Error>>
{
    // @step Given unreadable history exists and no session is open
    let fixture = StorageFixture::new().await?;
    let first = fixture.bad_manifest(DAMAGED_ID)?;
    let second = fixture.bad_manifest(OTHER_ID)?;
    for backend in &fixture.backends {
        // @step When each transport bootstraps the application
        let mut app = App::new(backend.clone());
        app.bootstrap().await?;
        drain(&mut app).await;
        // @step Then a scrollable warning exposes every path cause and recovery instruction without changing history
        assert!(app.compositor().contains("session-storage-dialog"));
        let visible = pages(&mut app);
        assert!(
            visible.contains(&compact(&first.display().to_string())),
            "{visible}"
        );
        assert!(
            visible.contains(&compact(&second.display().to_string())),
            "{visible}"
        );
        assert!(
            visible.contains("empty") || visible.contains("EOF"),
            "{visible}"
        );
        let recovery = visible.to_lowercase();
        assert!(
            recovery.contains("backup") || recovery.contains("backedup"),
            "{visible}"
        );
        let issues = backend.session_manifest_issues().await?;
        assert_eq!(issues, backend.session_manifest_issues().await?);
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().all(|issue| !issue.category.is_empty()
            && !issue.reason.is_empty()
            && !issue.recovery.is_empty()));
        assert!(app.agent_view_store().current_session().is_none());
        assert!(std::fs::read(&first)?.is_empty());
        assert!(std::fs::read(&second)?.is_empty());
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn reconnect_replays_unresolved_manifest_warnings() -> Result<(), Box<dyn std::error::Error>>
{
    // @step Given an unresolved manifest warning was dismissed
    let fixture = StorageFixture::new().await?;
    fixture.bad_manifest(DAMAGED_ID)?;
    for backend in &fixture.backends {
        let mut app = App::new(backend.clone());
        app.bootstrap().await?;
        drain(&mut app).await;
        key(&mut app, KeyCode::Esc);
        assert!(!app.compositor().contains("session-storage-dialog"));
        // @step When the application reconnects without an open session
        app.dispatch(Action::Reconnected);
        drain(&mut app).await;
        // @step Then the warning is visible again from a non draining snapshot
        assert!(app.compositor().contains("session-storage-dialog"));
        assert!(pages(&mut app).contains(DAMAGED_ID));
        assert!(app.agent_view_store().current_session().is_none());
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn creation_failures_are_not_missing_model_declines() -> Result<(), Box<dyn std::error::Error>>
{
    // @step Given a selected model and an unusable sessions directory
    let fixture = StorageFixture::new().await?;
    fixture.select_model();
    std::fs::write(
        fixture.directory.path().join("sessions"),
        b"not a directory",
    )?;
    for backend in &fixture.backends {
        // @step When the user creates a session from the dialog work unit or model selection
        for action in creation_actions() {
            let mut app = App::new(backend.clone());
            app.dispatch(action);
            drain(&mut app).await;
            // @step Then each transport shows the storage failure globally without publishing a session or claiming a missing model
            let visible = compact(&screen(&mut app));
            assert!(
                visible.contains("persist") || visible.contains("sessionsdir"),
                "{visible}"
            );
            assert!(!visible.contains("nodefaultmodel"), "{visible}");
            assert!(app.agent_view_store().current_session().is_none());
            assert!(backend.list_live_sessions().await?.is_empty());
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn missing_model_selection_remains_an_explicit_decline(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given no default model is selected
    let fixture = StorageFixture::new().await?;
    for backend in &fixture.backends {
        // @step When each transport attempts session creation
        let mut app = App::new(backend.clone());
        app.dispatch(Action::CreateSessionSubmitted { isolated: false });
        drain(&mut app).await;
        // @step Then the application requests model selection and creates no session
        assert!(compact(&screen(&mut app)).contains("nodefaultmodel"));
        assert!(app.agent_view_store().current_session().is_none());
        assert!(backend.list_live_sessions().await?.is_empty());
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn healthy_creation_preserves_roles_alongside_warnings(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a selected model and unreadable historical manifests
    let fixture = StorageFixture::new().await?;
    fixture.select_model();
    let damaged = fixture.bad_manifest(DAMAGED_ID)?;
    for backend in &fixture.backends {
        // @step When each transport creates a session with a role
        let session = backend
            .create_session(Some("storage investigator".into()))
            .await?;
        // @step Then a durable session with that role exists and unresolved warnings remain available
        assert!(!session.value.is_empty());
        let id = uuid::Uuid::parse_str(&session.value)?;
        assert_eq!(codelet_core::persistence::load_session(id)?.id, id);
        assert_eq!(
            backend.get_session_role(session).await?,
            Some("storage investigator".into())
        );
        let mut app = App::new(backend.clone());
        app.bootstrap().await?;
        drain(&mut app).await;
        assert!(pages(&mut app).contains(DAMAGED_ID));
        assert!(std::fs::read(&damaged)?.is_empty());
    }
    Ok(())
}
