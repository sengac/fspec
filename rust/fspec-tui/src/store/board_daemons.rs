//! RPC-448: the board's daemon switcher state — the daemons on offer, the
//! connected one and the dropdown highlight.
//!
//! Feature: spec/features/tui-daemon-switcher.feature

use crate::daemons::DaemonChoice;

use super::BoardStore;

#[derive(Debug, Default)]
pub struct DaemonSwitcher {
    entries: Vec<DaemonChoice>,
    connected: Option<String>,
    /// `Some(highlighted row)` while the dropdown is open.
    open: Option<usize>,
    /// TUI-123: why the daemons could not be listed (shown in the open
    /// switcher instead of rows).
    error: Option<String>,
}

impl BoardStore {
    /// The daemon the board is connected to, when a directory is installed.
    pub fn connected_daemon(&self) -> Option<&str> {
        self.daemons.connected.as_deref()
    }

    pub fn set_connected_daemon(&mut self, name: String) {
        self.daemons.connected = Some(name);
    }

    pub fn daemons(&self) -> &[DaemonChoice] {
        &self.daemons.entries
    }

    /// Open the dropdown over `entries`, highlighting the connected daemon.
    pub fn open_daemon_switcher(&mut self, entries: Vec<DaemonChoice>) {
        let row = entries
            .iter()
            .position(|entry| Some(entry.name.as_str()) == self.daemons.connected.as_deref())
            .unwrap_or(0);
        self.daemons.entries = entries;
        self.daemons.open = Some(row);
        self.daemons.error = None;
    }

    /// TUI-123: open the dropdown showing why the daemons could not be
    /// listed — never an empty list.
    pub fn open_daemon_switcher_failed(&mut self, error: String) {
        self.daemons.entries = Vec::new();
        self.daemons.open = Some(0);
        self.daemons.error = Some(error);
    }

    /// TUI-123: the listing error the open switcher shows.
    pub fn daemon_switcher_error(&self) -> Option<&str> {
        self.daemons.error.as_deref()
    }

    /// TUI-123: replace the listed daemons without opening the dropdown.
    pub fn set_daemon_entries(&mut self, entries: Vec<DaemonChoice>) {
        self.daemons.entries = entries;
        self.daemons.error = None;
    }

    pub fn close_daemon_switcher(&mut self) {
        self.daemons.open = None;
    }

    /// The highlighted dropdown row while the switcher is open.
    pub fn daemon_switcher(&self) -> Option<usize> {
        self.daemons.open
    }

    pub fn move_daemon_highlight(&mut self, delta: isize) {
        let last = self.daemons.entries.len().saturating_sub(1);
        if let Some(row) = self.daemons.open.as_mut() {
            *row = row.saturating_add_signed(delta).min(last);
        }
    }

    /// The daemon at dropdown `row` (the highlight when `None`).
    pub fn daemon_entry(&self, row: Option<usize>) -> Option<&DaemonChoice> {
        self.daemons.entries.get(row.or(self.daemons.open)?)
    }
}
