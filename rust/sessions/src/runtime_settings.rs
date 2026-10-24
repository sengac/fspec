use codelet_cli::session::Session;

use crate::background_session::BackgroundSession;
use crate::model_resolution::{resolve_current_model_limits, resolve_profile_max_images};

impl BackgroundSession {
    pub fn refresh_runtime_settings_for_turn(&self, inner: &mut Session) -> Result<(), String> {
        self.reconcile_focus_reminder(inner);
        inner
            .provider_manager_mut()
            .reload_runtime_settings()
            .map_err(|error| error.to_string())?;
        inner.apply_provider_runtime_settings();
        self.inherit_continue_state(inner.continue_enabled, inner.continue_budget);
        (inner.continue_enabled, inner.continue_budget) = self.get_continue_state();
        let manager = inner.provider_manager();
        let limits = resolve_current_model_limits(manager);
        self.set_model_limits(
            limits.context_window,
            limits.max_output_tokens,
            limits.compaction_threshold,
        );
        codelet_tools::model_capabilities::set_session_model_max_images(
            self.id,
            resolve_profile_max_images(manager),
        );
        Ok(())
    }
}
