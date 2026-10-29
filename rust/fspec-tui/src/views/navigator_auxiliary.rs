use crate::components::Action;

use super::{Navigator, ViewMode};

impl ViewMode {
    pub(super) fn is_auxiliary(self) -> bool {
        matches!(
            self,
            Self::ProviderSettings
                | Self::ModelSelector
                | Self::Blocklist
                | Self::ChangedFiles
                | Self::Checkpoints
        )
    }
}

impl Navigator {
    pub(super) fn apply_auxiliary_action(&mut self, action: &Action) -> bool {
        let opening = match action {
            Action::OpenProviderSettingsView => Some(ViewMode::ProviderSettings),
            Action::OpenModelSelectorView => Some(ViewMode::ModelSelector),
            Action::OpenBlocklistView => Some(ViewMode::Blocklist),
            Action::OpenChangedFilesView => Some(ViewMode::ChangedFiles),
            Action::OpenCheckpointsView => Some(ViewMode::Checkpoints),
            _ => None,
        };
        if let Some(destination) = opening {
            if !self.active_view.is_auxiliary() {
                self.auxiliary_origin = Some(self.active_view);
            }
            self.active_view = destination;
            return true;
        }

        let closing = match action {
            Action::CloseProviderSettingsView => ViewMode::ProviderSettings,
            Action::CloseModelSelectorView | Action::ModelSelected(..) => ViewMode::ModelSelector,
            Action::CloseBlocklistView => ViewMode::Blocklist,
            Action::CloseChangedFilesView | Action::CloseCheckpointsView
                if self.active_view == ViewMode::Mux =>
            {
                return false;
            }
            Action::CloseChangedFilesView => ViewMode::ChangedFiles,
            Action::CloseCheckpointsView => ViewMode::Checkpoints,
            _ => return false,
        };
        if self.active_view == closing {
            self.active_view = self.auxiliary_origin.take().unwrap_or_default();
        }
        true
    }
}
