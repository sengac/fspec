//! TUI-126: the App's one way to open a URL in the browser.
//!
//! Feature: spec/features/sign-in-screens-copy-open-paste-and-wrap.feature
//!
//! The board's attachment viewer and the sign-in screens open URLs through
//! the same launcher (`open::that` in production); tests replace it with
//! `App::set_url_launcher_for_test` and record what was opened.

use std::sync::Arc;

use super::state::App;
use crate::components::Action;

/// Opens a URL; the error is shown to the user.
pub type UrlLauncher = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

/// The production launcher: the system's default browser.
pub(crate) fn system_launcher() -> UrlLauncher {
    Arc::new(|url| open::that(url).map_err(|error| error.to_string()))
}

impl App {
    /// Open `url` in the browser on the blocking pool. A failure is reported
    /// on the Provider Settings status line, followed by `instead` (what the
    /// user can do without a browser).
    pub(crate) fn open_url(&mut self, url: String, instead: &'static str) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let launcher = self.url_launcher.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || launcher(&url))
                .await
                .map_err(|error| error.to_string())
                .and_then(|opened| opened);
            if let Err(reason) = result {
                let _ = action_tx.send(Action::ProviderSettingsStatus(format!(
                    "✗ Could not open a browser: {reason}. {instead}"
                )));
            }
        }));
    }

    /// TUI-126 test seam: replace the URL launcher.
    pub fn set_url_launcher_for_test(&mut self, launcher: UrlLauncher) {
        self.url_launcher = launcher;
    }
}
