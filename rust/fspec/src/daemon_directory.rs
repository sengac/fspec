//! RPC-448: the client binary's daemon directory — live local daemons from
//! their discovery files (RPC-446) plus saved daemons (RPC-447).
//!
//! Feature: spec/features/tui-daemon-switcher.feature
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use codelet_common::saved_daemons;
use codelet_fspec_tui::daemon_connect::{connect_daemon_trusting, DAEMON_CONNECT_TIMEOUT};
use codelet_fspec_tui::daemons::{DaemonChoice, DaemonDirectory, DaemonForm};
use codelet_fspec_tui::{Action, App, FspecBackend};
use tokio::sync::mpsc::UnboundedSender;
use url::Url;

use crate::common;

/// Lists local and saved daemons; connecting starts the new backend's
/// reconnect supervisor on the App's action bus.
pub struct ClientDaemonDirectory {
    pub action_tx: UnboundedSender<Action>,
}

/// The switcher name of a local daemon: its workspace folder and port.
pub fn local_name(entry: &common::DaemonEntry) -> String {
    let folder = entry
        .workspace
        .as_deref()
        .and_then(|path| std::path::Path::new(path).file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "daemon".to_string());
    format!("{folder}:{}", entry.port)
}

/// BUG-595: the switcher choice for a local daemon, dialed at the scheme
/// and host its discovery entry names.
pub fn local_choice(entry: &common::DaemonEntry) -> DaemonChoice {
    DaemonChoice {
        name: local_name(entry),
        url: entry.url(),
        saved: false,
    }
}

/// The switcher name for the daemon at `url`: a saved name, a local
/// daemon's name, or the URL itself when no local daemon owns that port.
/// BUG-430: an unreadable local daemon registry is an error.
pub fn name_for(url: &Url, saved_name: Option<&str>) -> Result<String> {
    if let Some(name) = saved_name {
        return Ok(name.to_string());
    }
    let local = common::list_live_daemons(&common::daemons_dir()?)?;
    Ok(local
        .iter()
        .find(|entry| url.port() == Some(entry.port))
        .map(local_name)
        .unwrap_or_else(|| url.to_string()))
}

/// BUG-430: install the board's daemon switcher on `app`, naming the daemon
/// at `url` as connected — the one place client and combined mode do it.
pub fn install_daemon_switcher(app: &mut App, url: &Url, saved_name: Option<&str>) -> Result<()> {
    let directory = ClientDaemonDirectory {
        action_tx: app.action_tx_clone(),
    };
    let connected = name_for(url, saved_name)?;
    app.set_daemon_directory(Arc::new(directory), connected);
    Ok(())
}

#[async_trait]
impl DaemonDirectory for ClientDaemonDirectory {
    fn list(&self) -> anyhow::Result<Vec<DaemonChoice>> {
        // BUG-453: unreadable daemon state is an error, not an empty list.
        let mut choices: Vec<DaemonChoice> = common::list_live_daemons(&common::daemons_dir()?)?
            .iter()
            .map(local_choice)
            .collect();
        choices.extend(
            saved_daemons::list(&saved_daemons::default_path()?)?
                .into_iter()
                .map(|daemon| DaemonChoice {
                    name: daemon.name,
                    url: daemon.url,
                    saved: true,
                }),
        );
        Ok(choices)
    }

    async fn connect(&self, choice: &DaemonChoice) -> Result<Arc<dyn FspecBackend>> {
        let (token, ca) = if choice.saved {
            // RPC-458: a saved daemon's own CA is trusted beside the roots.
            let saved = saved_daemons::find(&saved_daemons::default_path()?, &choice.name)?
                .ok_or_else(|| anyhow::anyhow!("no saved daemon named {}", choice.name))?;
            let ca = crate::daemons_cmd::saved_ca(&saved);
            (saved.token, ca)
        } else {
            // BUG-428: a local daemon reads FSPEC_DAEMON_TOKEN (RPC-447); the
            // client presents the same variable.
            (local_daemon_token()?, None)
        };
        let url = Url::parse(&choice.url)?;
        // TUI-123: give up after a timeout and name the failure by kind
        // (refused, timed out, TLS, token rejected).
        let backend = connect_daemon_trusting(&url, token, ca, DAEMON_CONNECT_TIMEOUT).await?;
        backend
            .supervisor_handle()
            .start(url, self.action_tx.clone());
        Ok(Arc::new(backend))
    }

    /// TUI-121: save to ~/.fspec/daemons.json with the CLI's validation.
    fn save(&self, previous: Option<&str>, form: &DaemonForm) -> Result<()> {
        saved_daemons::update(
            &saved_daemons::default_path()?,
            previous,
            &form.name,
            &form.url,
            form.token.clone(),
        )?;
        Ok(())
    }

    /// TUI-121: forget a saved daemon.
    fn remove(&self, name: &str) -> Result<()> {
        if !saved_daemons::remove(&saved_daemons::default_path()?, name)? {
            anyhow::bail!("no saved daemon named {name}");
        }
        Ok(())
    }
}

/// BUG-428: the bearer for a local daemon — `FSPEC_DAEMON_TOKEN` when set;
/// a value that is not valid Unicode is an error.
pub fn local_daemon_token() -> Result<Option<String>> {
    match std::env::var("FSPEC_DAEMON_TOKEN") {
        Ok(token) => Ok(Some(token)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(anyhow::anyhow!("FSPEC_DAEMON_TOKEN is unusable: {error}")),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod combined_mode_tests;

#[cfg(test)]
mod bug595_tests;
