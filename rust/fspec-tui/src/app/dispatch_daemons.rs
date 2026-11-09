//! RPC-448: App::dispatch routing for the board's daemon switcher —
//! opening it over the installed DaemonDirectory, connecting the chosen
//! daemon in a pending task, and swapping the App onto the new backend.
//!
//! Feature: spec/features/tui-daemon-switcher.feature
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature
//!
//! Switching DETACHES: the old backend is dropped (never asked to destroy
//! or interrupt anything), so sessions keep running on that daemon and the
//! connect-time session snapshot restores their tabs on switching back.

use std::sync::Arc;

use super::work_units_refetch::{refetch_checkpoint_counts, refetch_work_units, send_or_log};
use super::App;
use crate::components::Action;
use crate::daemons::{DaemonChoice, DaemonDirectory, SwitchedBackend};

impl App {
    /// Install the daemon directory the switcher lists, naming the daemon
    /// the App is connected to now.
    pub fn set_daemon_directory(&mut self, directory: Arc<dyn DaemonDirectory>, connected: String) {
        self.daemon_directory = Some(directory);
        self.board_store.set_connected_daemon(connected);
    }

    pub(crate) fn try_dispatch_daemons(&mut self, action: &Action) -> bool {
        match action {
            // TUI-129: in the mux it opens on the Board pane.
            Action::OpenDaemonSwitcher => {
                if self.board_pane_ready("Daemons") {
                    self.open_daemon_switcher();
                }
            }
            Action::CloseDaemonSwitcher => self.board_store.close_daemon_switcher(),
            Action::MoveDaemonHighlight(delta) => self.board_store.move_daemon_highlight(*delta),
            Action::ChooseDaemon(row) => self.spawn_daemon_switch(*row),
            Action::DaemonSwitched(switched) => self.handle_daemon_switched(switched.clone()),
            // TUI-121: saved-daemon management (app/dispatch_daemon_admin.rs).
            _ => return self.try_dispatch_daemon_admin(action),
        }
        true
    }

    fn open_daemon_switcher(&mut self) {
        match self.daemon_directory.clone() {
            Some(directory) => match directory.list() {
                Ok(entries) => self.board_store.open_daemon_switcher(entries),
                // BUG-453 / TUI-123: the failure is shown in the
                // switcher, never an empty list.
                Err(error) => self
                    .board_store
                    .open_daemon_switcher_failed(format!("Daemons could not be read: {error:#}")),
            },
            // TUI-123: reachable from the Settings menu too.
            None => self.dispatch(Action::SessionOperationFailed(
                "Daemons cannot be managed here: this TUI was started without a daemon directory"
                    .to_owned(),
            )),
        }
    }

    fn spawn_daemon_switch(&mut self, row: Option<usize>) {
        let choice = self.board_store.daemon_entry(row).cloned();
        self.board_store.close_daemon_switcher();
        let Some(choice) = choice else {
            return;
        };
        if Some(choice.name.as_str()) == self.board_store.connected_daemon() {
            return;
        }
        let failed = format!("Could not connect to daemon {}", choice.name);
        self.spawn_daemon_connect(choice, failed);
    }

    /// Connect to `choice` in a pending task and switch to it. TUI-123: a
    /// failure (named by kind by the directory) is shown after `failed`,
    /// and the current connection is kept.
    pub(super) fn spawn_daemon_connect(&mut self, choice: DaemonChoice, failed: String) {
        let Some(directory) = self.daemon_directory.clone() else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let still = match self.board_store.connected_daemon() {
            Some(current) => format!(" Still connected to {current}."),
            None => String::new(),
        };
        let tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            match directory.connect(&choice).await {
                Ok(backend) => {
                    let switched = SwitchedBackend {
                        name: choice.name,
                        backend,
                    };
                    send_or_log(&tx, Action::DaemonSwitched(switched));
                }
                Err(error) => send_or_log(
                    &tx,
                    Action::SessionOperationFailed(format!("{failed}: {error:#}.{still}")),
                ),
            }
        }));
    }

    fn handle_daemon_switched(&mut self, switched: SwitchedBackend) {
        for task in self.subscriber_tasks.drain(..) {
            task.abort();
        }
        self.backend = switched.backend;
        let daemon = switched.name.clone();
        self.board_store.set_connected_daemon(switched.name);
        self.board_store.set_workspaces(Vec::new());
        // The previous daemon's tabs belong to that daemon; its sessions
        // keep running there and come back with its session snapshot.
        self.agent_view_store = crate::store::AgentViewStore::default();
        self.transcript_sync.clear();
        self.model_sync.clear();
        self.continue_sync.clear();
        let _ = self.active_session_tx.send(None);
        self.navigator.active_view = crate::views::ViewMode::Board;
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        self.spawn_subscriber_tasks();
        self.spawn_load_workspaces();
        self.spawn_replay_session_snapshot();
        let backend = self.backend.clone();
        let tx = self.action_tx.clone();
        let when = format!("after switching to daemon {daemon}");
        self.pending_tasks.push(tokio::spawn(async move {
            // BUG-379: a failed reload is shown, never dropped.
            refetch_work_units(backend.as_ref(), &tx, &when).await;
            refetch_checkpoint_counts(backend.as_ref(), &tx, &when).await;
        }));
    }
}
