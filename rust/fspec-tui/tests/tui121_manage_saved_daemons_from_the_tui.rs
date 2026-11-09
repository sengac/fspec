//! TUI-121 — add, edit and remove saved daemons from the TUI.
//!
//! Feature: spec/features/manage-saved-daemons-from-the-tui.feature

use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use codelet_common::saved_daemons::{self, SavedDaemon};
use codelet_fspec_tui::daemons::{DaemonChoice, DaemonDirectory, DaemonForm};
use codelet_fspec_tui::{App, FspecBackend};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::MockBackend;

const FORM: &str = "daemon-form-dialog";

/// A directory over a real saved-daemons file plus one local daemon.
struct FileDirectory {
    path: PathBuf,
}

#[async_trait]
impl DaemonDirectory for FileDirectory {
    fn list(&self) -> anyhow::Result<Vec<DaemonChoice>> {
        let mut choices = vec![DaemonChoice {
            name: "alpha".into(),
            url: "ws://127.0.0.1:4001".into(),
            saved: false,
        }];
        choices.extend(
            saved_daemons::list(&self.path)?
                .into_iter()
                .map(|daemon| DaemonChoice {
                    name: daemon.name,
                    url: daemon.url,
                    saved: true,
                }),
        );
        Ok(choices)
    }

    async fn connect(&self, choice: &DaemonChoice) -> anyhow::Result<Arc<dyn FspecBackend>> {
        Err(anyhow::anyhow!(
            "not connecting to {} in this test",
            choice.name
        ))
    }

    fn save(&self, previous: Option<&str>, form: &DaemonForm) -> anyhow::Result<()> {
        saved_daemons::update(
            &self.path,
            previous,
            &form.name,
            &form.url,
            form.token.clone(),
        )?;
        Ok(())
    }

    fn remove(&self, name: &str) -> anyhow::Result<()> {
        if !saved_daemons::remove(&self.path, name)? {
            anyhow::bail!("no saved daemon named {name}");
        }
        Ok(())
    }
}

async fn app_with(path: PathBuf) -> Result<App, Box<dyn std::error::Error>> {
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.set_daemon_directory(Arc::new(FileDirectory { path }), "alpha".to_string());
    app.bootstrap().await?;
    settle(&mut app).await?;
    Ok(app)
}

async fn settle(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    for _ in 0..5 {
        // TEST-042: scheduler rounds, not a time window: on the
        // current-thread test runtime every ready task (a spawned backend
        // call, a subscriber forwarding a push) runs before this resumes.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
        }
    }
    Ok(())
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

async fn press(app: &mut App, code: KeyCode) -> Result<(), codelet_test_helpers::TestError> {
    app.handle_event(&key(code));
    settle(app).await?;
    Ok(())
}

async fn type_text(app: &mut App, text: &str) -> Result<(), codelet_test_helpers::TestError> {
    for character in text.chars() {
        app.handle_event(&key(KeyCode::Char(character)));
    }
    settle(app).await?;
    Ok(())
}

fn screen(app: &mut App) -> String {
    let area = Rect::new(0, 0, 160, 40);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Open the switcher and fill the add form: name, Tab, url, Tab, token.
async fn add_daemon(
    app: &mut App,
    name: &str,
    url: &str,
    token: &str,
) -> Result<Vec<String>, codelet_test_helpers::TestError> {
    press(app, KeyCode::Char('s')).await?;
    press(app, KeyCode::Char('a')).await?;
    let mut frames = vec![screen(app)];
    type_text(app, name).await?;
    press(app, KeyCode::Tab).await?;
    type_text(app, url).await?;
    press(app, KeyCode::Tab).await?;
    type_text(app, token).await?;
    frames.push(screen(app));
    press(app, KeyCode::Enter).await?;
    frames.push(screen(app));
    Ok(frames)
}

#[tokio::test(flavor = "current_thread")]
async fn adding_a_saved_daemon_from_the_switcher() -> TestResult {
    // @step Given the daemon switcher is open
    let home = tempfile::tempdir()?;
    let path = home.path().join("daemons.json");
    let mut app = app_with(path.clone()).await?;
    // @step When the user adds daemon "work" at "wss://fspec.example.com" with token "secret-token"
    let frames = add_daemon(&mut app, "work", "wss://fspec.example.com", "secret-token").await?;
    // @step Then "work" is saved with that URL and token
    assert_eq!(
        saved_daemons::find(&path, "work")?,
        Some(SavedDaemon {
            name: "work".into(),
            url: "wss://fspec.example.com".into(),
            token: Some("secret-token".into()),
            ca: None,
        })
    );
    // @step And the switcher lists "work"
    assert!(app
        .board_store()
        .daemons()
        .iter()
        .any(|d| d.name == "work" && d.saved));
    // @step And "secret-token" never appears on screen
    for frame in frames {
        assert!(!frame.contains("secret-token"), "{frame}");
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn a_remote_daemon_without_a_token_is_refused_and_the_form_reopens() -> TestResult {
    // @step Given the daemon switcher is open
    let home = tempfile::tempdir()?;
    let path = home.path().join("daemons.json");
    let mut app = app_with(path.clone()).await?;
    // @step When the user adds daemon "work" at "wss://fspec.example.com" without a token
    add_daemon(&mut app, "work", "wss://fspec.example.com", "").await?;
    // @step Then nothing is saved
    assert!(saved_daemons::list(&path)?.is_empty());
    // @step And the form is open again showing that the daemon needs a token
    assert!(app.compositor().contains(FORM));
    let text = screen(&mut app);
    assert!(text.contains("needs a token"), "{text}");
    assert!(text.contains("wss://fspec.example.com"), "{text}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn editing_a_saved_daemon_keeps_its_token_unless_a_new_one_is_typed() -> TestResult {
    // @step Given saved daemon "work" at "wss://fspec.example.com" with token "secret-token"
    let home = tempfile::tempdir()?;
    let path = home.path().join("daemons.json");
    saved_daemons::save(
        &path,
        SavedDaemon {
            name: "work".into(),
            url: "wss://fspec.example.com".into(),
            token: Some("secret-token".into()),
            ca: None,
        },
    )?;
    let mut app = app_with(path.clone()).await?;
    // @step When the user edits it to "wss://fspec2.example.com" leaving the token empty
    press(&mut app, KeyCode::Char('s')).await?;
    press(&mut app, KeyCode::Down).await?;
    press(&mut app, KeyCode::Char('e')).await?;
    assert!(app.compositor().contains(FORM));
    assert!(!screen(&mut app).contains("secret-token"));
    press(&mut app, KeyCode::Tab).await?;
    for _ in 0.."wss://fspec.example.com".len() {
        app.handle_event(&key(KeyCode::Backspace));
    }
    type_text(&mut app, "wss://fspec2.example.com").await?;
    press(&mut app, KeyCode::Enter).await?;
    // @step Then "work" is saved with the new URL and the token "secret-token"
    assert_eq!(
        saved_daemons::list(&path)?,
        vec![SavedDaemon {
            name: "work".into(),
            url: "wss://fspec2.example.com".into(),
            token: Some("secret-token".into()),
            ca: None,
        }]
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn removing_a_saved_daemon_but_not_a_local_one() -> TestResult {
    // @step Given the switcher lists local daemon "alpha" and saved daemon "work"
    let home = tempfile::tempdir()?;
    let path = home.path().join("daemons.json");
    saved_daemons::save(
        &path,
        SavedDaemon {
            name: "work".into(),
            url: "ws://127.0.0.1:4002".into(),
            token: None,
            ca: None,
        },
    )?;
    let mut app = app_with(path.clone()).await?;
    // @step When the user removes "work" and then tries to remove "alpha"
    press(&mut app, KeyCode::Char('s')).await?;
    press(&mut app, KeyCode::Down).await?;
    press(&mut app, KeyCode::Char('d')).await?;
    // TUI-123: removal asks y/n first.
    press(&mut app, KeyCode::Char('y')).await?;
    press(&mut app, KeyCode::Char('s')).await?;
    press(&mut app, KeyCode::Char('d')).await?;
    // @step Then "work" is no longer saved
    assert!(saved_daemons::list(&path)?.is_empty());
    // @step And an error explains that the local daemon "alpha" is not saved
    assert!(app.compositor().contains("session-storage-dialog"));
    let text = screen(&mut app);
    assert!(
        text.contains("alpha") && text.contains("not a saved daemon"),
        "{text}"
    );
    Ok(())
}
