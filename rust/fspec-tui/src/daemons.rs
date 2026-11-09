//! RPC-448: the daemons a client can switch between.
//!
//! Feature: spec/features/tui-daemon-switcher.feature
//!
//! The binary owns daemon discovery (local discovery files, saved remote
//! daemons) and installs a [`DaemonDirectory`] on the App; the board's
//! switcher lists its choices and asks it for a backend to switch onto.

use std::sync::Arc;

use async_trait::async_trait;

use crate::transport::FspecBackend;

/// One daemon the switcher offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonChoice {
    pub name: String,
    pub url: String,
    /// `true` for a saved (remote) daemon, `false` for a local one.
    pub saved: bool,
}

/// TUI-121: a saved daemon as entered in the daemon form. The token is a
/// [`TokenChange`], whose `Debug` never prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonForm {
    pub name: String,
    pub url: String,
    pub token: TokenChange,
}

pub use codelet_common::saved_daemons::{DaemonField, TokenChange};

/// TUI-123: why the daemon form was refused, shown under `field` (under
/// all fields when `None`: the saved-daemons file itself failed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonFormError {
    pub field: Option<DaemonField>,
    pub message: String,
}

/// Lists daemons and opens a backend for one of them.
#[async_trait]
pub trait DaemonDirectory: Send + Sync {
    /// BUG-453: an unreadable daemons directory or saved-daemons file is an
    /// error, never an empty list.
    fn list(&self) -> anyhow::Result<Vec<DaemonChoice>>;
    async fn connect(&self, choice: &DaemonChoice) -> anyhow::Result<Arc<dyn FspecBackend>>;

    /// TUI-121: save `form`, replacing the saved daemon `previous` (an
    /// edit) — validated like `fspec daemons add`. A directory that cannot
    /// save daemons refuses.
    fn save(&self, _previous: Option<&str>, _form: &DaemonForm) -> anyhow::Result<()> {
        anyhow::bail!("this daemon directory cannot save daemons")
    }

    /// TUI-121: forget the saved daemon `name`. A directory that cannot
    /// save daemons refuses.
    fn remove(&self, _name: &str) -> anyhow::Result<()> {
        anyhow::bail!("this daemon directory cannot remove saved daemons")
    }
}

/// A backend connected for a chosen daemon (carried by an Action).
#[derive(Clone)]
pub struct SwitchedBackend {
    pub name: String,
    pub backend: Arc<dyn FspecBackend>,
}

impl std::fmt::Debug for SwitchedBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwitchedBackend")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}
