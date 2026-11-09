//! TUI-121: adding, editing and removing saved daemons from the switcher.
//!
//! Feature: spec/features/manage-saved-daemons-from-the-tui.feature
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature
//!
//! TUI-123: a name already used by a listed daemon (saved or local,
//! ignoring case) is refused naming it; a refusal reopens the form with the
//! reason under the field it concerns; saving a new daemon connects to it
//! (it stays saved when that fails); editing the connected daemon keeps the
//! connection and says the change applies on the next connect; removal asks
//! y/n and removing the connected daemon keeps the connection, the header
//! then naming it by URL.

use super::App;
use crate::components::daemon_form_dialog::{DaemonFormDialog, DAEMON_FORM_DIALOG_ID};
use crate::components::daemon_remove_confirm_dialog::DaemonRemoveConfirmDialog;
use crate::components::notification_dialog::NotificationDialog;
use crate::components::Action;
use crate::daemons::{DaemonChoice, DaemonField, DaemonForm, DaemonFormError, TokenChange};
use codelet_common::saved_daemons::SavedDaemonError;

impl App {
    pub(crate) fn try_dispatch_daemon_admin(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenAddDaemonForm => {
                self.board_store.close_daemon_switcher();
                let empty = DaemonForm {
                    name: String::new(),
                    url: String::new(),
                    token: TokenChange::Keep,
                };
                self.open_daemon_form(None, &empty, None);
            }
            Action::OpenEditDaemonForm => {
                if let Some(choice) = self.highlighted_saved_daemon() {
                    let form = DaemonForm {
                        name: choice.name.clone(),
                        url: choice.url,
                        token: TokenChange::Keep,
                    };
                    self.open_daemon_form(Some(choice.name), &form, None);
                }
            }
            Action::SaveDaemon { previous, form } => self.save_daemon(previous.as_deref(), form),
            Action::RemoveSavedDaemon => {
                if let Some(choice) = self.highlighted_saved_daemon() {
                    self.compositor
                        .push(Box::new(DaemonRemoveConfirmDialog::new(
                            choice,
                            self.action_tx.clone(),
                        )));
                }
            }
            Action::ConfirmRemoveSavedDaemon(choice) => self.remove_saved_daemon(choice),
            _ => return false,
        }
        true
    }

    fn open_daemon_form(
        &mut self,
        previous: Option<String>,
        form: &DaemonForm,
        error: Option<DaemonFormError>,
    ) {
        let _ = self.compositor.remove(DAEMON_FORM_DIALOG_ID);
        self.compositor.push(Box::new(DaemonFormDialog::new(
            previous,
            form,
            error,
            self.action_tx.clone(),
        )));
    }

    /// The highlighted switcher entry when it is a saved daemon; a local
    /// (discovered) daemon is refused with the reason shown.
    fn highlighted_saved_daemon(&mut self) -> Option<DaemonChoice> {
        let choice = self.board_store.daemon_entry(None).cloned()?;
        self.board_store.close_daemon_switcher();
        if choice.saved {
            return Some(choice);
        }
        self.dispatch(Action::SessionOperationFailed(format!(
            "{} is a local daemon, not a saved daemon: it cannot be edited or removed",
            choice.name
        )));
        None
    }

    fn save_daemon(&mut self, previous: Option<&str>, form: &DaemonForm) {
        let Some(directory) = self.daemon_directory.clone() else {
            return self.dispatch(Action::SessionOperationFailed(
                "No daemon directory is installed; daemons cannot be saved".to_owned(),
            ));
        };
        let refuse = |app: &mut App, error: DaemonFormError| {
            app.open_daemon_form(previous.map(str::to_owned), form, Some(error));
        };
        // The listing is re-read: another process may have saved daemons.
        let listed = match directory.list() {
            Ok(listed) => listed,
            Err(error) => return refuse(self, form_error(&error)),
        };
        if let Some(clash) = name_clash(&listed, previous, &form.name) {
            return refuse(self, clash);
        }
        if let Err(error) = directory.save(previous, form) {
            return refuse(self, form_error(&error));
        }
        let name = form.name.trim().to_owned();
        let listed = match directory.list() {
            Ok(listed) => listed,
            Err(error) => {
                return self.dispatch(Action::SessionOperationFailed(format!(
                    "Saved daemon {name}, but daemons could not be read: {error:#}"
                )))
            }
        };
        let saved = listed
            .iter()
            .find(|choice| choice.saved && choice.name == name)
            .cloned();
        self.board_store.set_daemon_entries(listed);
        match (previous, saved) {
            // Product owner, 2026-11-05: a new daemon is connected to at
            // once; it stays saved when that fails.
            (None, Some(choice)) => {
                let failed = format!("Saved daemon {name}, but could not connect");
                self.spawn_daemon_connect(choice, failed);
            }
            (None, None) => self.dispatch(Action::SessionOperationFailed(format!(
                "Saved daemon {name}, but it is not listed; it cannot be connected to"
            ))),
            (Some(previous), _) => {
                if Some(previous) == self.board_store.connected_daemon() {
                    self.board_store.set_connected_daemon(name.clone());
                    self.compositor.push(Box::new(
                        NotificationDialog::info(format!(
                            "Saved daemon {name}. The TUI stays connected with the previous \
                             settings; the change applies the next time it connects to {name}."
                        ))
                        .with_auto_dismiss_ms(0),
                    ));
                }
                self.dispatch(Action::OpenDaemonSwitcher);
            }
        }
    }

    /// Remove a saved daemon (after its y/n prompt). Removing the connected
    /// daemon keeps the connection; the header then names it by URL.
    fn remove_saved_daemon(&mut self, choice: &DaemonChoice) {
        let Some(directory) = self.daemon_directory.clone() else {
            return;
        };
        match directory.remove(&choice.name) {
            Ok(()) => {
                if Some(choice.name.as_str()) == self.board_store.connected_daemon() {
                    self.board_store.set_connected_daemon(choice.url.clone());
                }
                self.dispatch(Action::OpenDaemonSwitcher);
            }
            Err(error) => self.dispatch(Action::SessionOperationFailed(format!(
                "The saved daemon {} could not be removed: {error:#}",
                choice.name
            ))),
        }
    }
}

/// A refusal shown under the form field it concerns.
fn form_error(error: &anyhow::Error) -> DaemonFormError {
    DaemonFormError {
        field: error
            .chain()
            .find_map(|cause| cause.downcast_ref::<SavedDaemonError>())
            .and_then(SavedDaemonError::field),
        message: format!("{error:#}"),
    }
}

/// TUI-123: `name` clashes (ignoring case) with a listed daemon other than
/// the saved daemon being edited.
fn name_clash(
    listed: &[DaemonChoice],
    previous: Option<&str>,
    name: &str,
) -> Option<DaemonFormError> {
    let wanted = name.trim().to_lowercase();
    let clash = listed.iter().find(|choice| {
        !(choice.saved && Some(choice.name.as_str()) == previous)
            && choice.name.to_lowercase() == wanted
    })?;
    let message = if clash.saved {
        format!("a daemon named {} is already saved", clash.name)
    } else {
        format!("{} is the name of a local daemon", clash.name)
    };
    Some(DaemonFormError {
        field: Some(DaemonField::Name),
        message,
    })
}
