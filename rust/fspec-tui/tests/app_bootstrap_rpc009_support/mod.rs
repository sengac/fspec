//! BUG-356: helpers for tests/app_bootstrap_rpc009.rs (the support-module
//! pattern: helpers here, every test in the one root file).

use codelet_test_helpers::OrFail;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use codelet_fspec_tui::app::transcript_sync::TranscriptAction;
use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use tracing::subscriber::DefaultGuard;
use tracing::Level;

pub fn wu(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// BUG-299: transcript content chunks route as `Transcript(Live(..))`.
pub fn live_transcript_for(action: &Action, session: &str) -> bool {
    matches!(
        action,
        Action::Transcript(TranscriptAction::Live(id, _)) if id == &SessionId::new(session)
    )
}

/// Drain the App's action bus until a matching action arrives or 200ms
/// elapses. Returns Some(action) on match, None on timeout.
pub async fn wait_for_action<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    // TEST-042: wait for the action itself under the deadlock guard (None
    // only when the guard expires), never for a fixed window.
    codelet_test_helpers::deadlock_guard("the awaited action", async {
        loop {
            while let Some(action) = app.try_recv_action() {
                if pred(&action) {
                    return action;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .ok()
}

/// BUG-356: captures every tracing event (DEBUG and up) emitted on the
/// current thread while the guard from `install` is held. The tests run on
/// the current-thread runtime, so spawned subscriber tasks log here too.
#[derive(Clone, Default)]
pub struct LogCapture(Arc<Mutex<Vec<u8>>>);

impl Write for LogCapture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .map_err(|error| io::Error::other(format!("log buffer lock: {error}")))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl LogCapture {
    pub fn install(&self) -> DefaultGuard {
        let writer = self.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_max_level(Level::DEBUG)
            .with_writer(move || writer.clone())
            .finish();
        tracing::subscriber::set_default(subscriber)
    }

    pub fn text(&self) -> Result<String, codelet_test_helpers::TestError> {
        let bytes = self.0.lock().or_fail("log buffer lock")?.clone();
        Ok(String::from_utf8(bytes).or_fail("captured logs are UTF-8")?)
    }
}
