//! TUI-131 test support: an App over a `MockBackend`, the user's keys and
//! pastes, and the long texts the scenarios show.
//!
//! Feature: spec/features/long-text-wraps-instead-of-being-cut-off.feature

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use codelet_fspec_tui::daemons::{DaemonChoice, DaemonDirectory, DaemonForm};
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use codelet_test_helpers::TestResult;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::common::{MockBackend, TaskOutcome};

pub fn sid(id: &str) -> SessionId {
    SessionId::new(id)
}

pub struct Fixture {
    pub app: App,
    pub mock: Arc<MockBackend>,
}

pub fn fixture_with(mock: Arc<MockBackend>) -> Fixture {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    Fixture {
        app: App::new(backend),
        mock,
    }
}

pub fn fixture() -> Fixture {
    fixture_with(Arc::new(MockBackend::new()))
}

/// An App on the agent view of session s-1.
pub async fn agent_fixture(mock: Arc<MockBackend>) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::SessionCreated(sid("s-1")));
    fx.app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    settle(&mut fx.app).await?;
    Ok(fx)
}

/// Run every parked task and queued action until the App is quiet.
pub async fn settle(app: &mut App) -> TestResult {
    for _ in 0..64 {
        let mut progressed = false;
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
            progressed = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            progressed = true;
        }
        if !progressed {
            return Ok(());
        }
    }
    Ok(())
}

pub async fn key(fx: &mut Fixture, code: KeyCode) -> TestResult {
    let _ = fx
        .app
        .handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    settle(&mut fx.app).await
}

pub async fn press(fx: &mut Fixture, ch: char) -> TestResult {
    key(fx, KeyCode::Char(ch)).await
}

pub async fn paste(fx: &mut Fixture, text: &str) -> TestResult {
    let _ = fx.app.handle_paste(text);
    settle(&mut fx.app).await
}

/// Exactly `n` characters of words `{tag}0 {tag}1 …` ending with the word
/// `last` (so the last word is unique in the text).
pub fn prose(tag: &str, n: usize, last: &str) -> String {
    let body_len = n.saturating_sub(last.len() + 1);
    let mut body = String::new();
    let mut i = 0;
    while body.len() < body_len {
        if !body.is_empty() {
            body.push(' ');
        }
        body.push_str(&format!("{tag}{i}"));
        i += 1;
    }
    body.truncate(body_len);
    if body.ends_with(' ') {
        body.pop();
        body.push('x');
    }
    format!("{body} {last}")
}

/// `prefix` padded with letters to exactly `n` characters.
pub fn token(prefix: &str, n: usize) -> String {
    let mut out = prefix.to_string();
    let mut i = 0u8;
    while out.len() < n {
        out.push(char::from(b'a' + i % 26));
        i = i.wrapping_add(1);
    }
    out.truncate(n);
    out
}

/// Saved daemons kept in memory; connecting is never asked for here.
pub struct MemoryDaemons(pub Mutex<Vec<DaemonChoice>>);

#[async_trait]
impl DaemonDirectory for MemoryDaemons {
    fn list(&self) -> anyhow::Result<Vec<DaemonChoice>> {
        let daemons = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("daemons lock poisoned"))?;
        Ok(daemons.clone())
    }

    async fn connect(&self, choice: &DaemonChoice) -> anyhow::Result<Arc<dyn FspecBackend>> {
        anyhow::bail!("no connection to {} in this test", choice.name)
    }

    fn save(&self, previous: Option<&str>, form: &DaemonForm) -> anyhow::Result<()> {
        let mut daemons = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("daemons lock poisoned"))?;
        daemons.retain(|daemon| Some(daemon.name.as_str()) != previous);
        daemons.push(DaemonChoice {
            name: form.name.clone(),
            url: form.url.clone(),
            saved: true,
        });
        Ok(())
    }
}
