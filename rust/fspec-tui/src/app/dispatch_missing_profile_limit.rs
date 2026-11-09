//! BUG-547: a local profile model's missing limit is asked of the user when
//! the model is selected, saved to the profile and the selection (or session
//! creation) runs again; cancelling shows the refusal.
//!
//! Feature: spec/features/a-missing-profile-model-limit-is-resolved-at-selection.feature

use codelet_rpc_types::ProfileDefinition;
use codelet_sessions::profile_limits::{LimitField, MissingProfileLimit};
use tokio::sync::mpsc::UnboundedSender;

use super::App;
use crate::components::missing_profile_limit_dialog::{
    MissingLimitRetry, MissingProfileLimitDialog, MISSING_PROFILE_LIMIT_DIALOG_ID,
};
use crate::components::Action;

impl App {
    pub(crate) fn try_dispatch_missing_profile_limit(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenMissingProfileLimitPrompt { limit, retry } => {
                let _ = self.compositor.remove(MISSING_PROFILE_LIMIT_DIALOG_ID);
                self.compositor
                    .push(Box::new(MissingProfileLimitDialog::new(
                        limit.clone(),
                        retry.clone(),
                        self.action_tx.clone(),
                    )));
            }
            Action::SaveMissingProfileLimit {
                limit,
                value,
                retry,
            } => self.spawn_save_missing_limit(limit.clone(), *value, retry.clone()),
            Action::MissingProfileLimitCancelled { limit, retry } => {
                report_refusal(&self.action_tx, retry, &limit.to_string());
            }
            Action::RetryAfterMissingProfileLimit(retry) => self.retry_after_missing_limit(retry),
            _ => return false,
        }
        true
    }

    fn spawn_save_missing_limit(
        &mut self,
        limit: MissingProfileLimit,
        value: u32,
        retry: MissingLimitRetry,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            let saved = async {
                let configs = backend
                    .list_provider_settings()
                    .await
                    .map_err(|error| error.to_string())?;
                let mut definition = current_definition(&configs, &limit)?;
                match limit.field {
                    LimitField::ContextWindow => definition.context_window = Some(value),
                    LimitField::MaxOutputTokens => definition.max_output_tokens = Some(value),
                }
                backend
                    .save_profile(limit.provider.clone(), limit.profile.clone(), definition)
                    .await
                    .map_err(|error| error.to_string())
            }
            .await;
            match saved {
                Ok(()) => {
                    tracing::info!(
                        profile = %limit.profile,
                        field = limit.field.config_key(),
                        value,
                        "BUG-547: saved the missing limit to the profile"
                    );
                    let _ = action_tx.send(Action::RetryAfterMissingProfileLimit(retry));
                }
                Err(error) => report_refusal(
                    &action_tx,
                    &retry,
                    &format!(
                        "{limit} ({} {value} could not be saved: {error})",
                        limit.field.config_key()
                    ),
                ),
            }
        }));
    }

    fn retry_after_missing_limit(&mut self, retry: &MissingLimitRetry) {
        match retry.clone() {
            MissingLimitRetry::ModelSelected {
                session,
                provider,
                model,
                selection,
            } => {
                // BUG-576: the limit is saved; a later selection stands.
                if session
                    .as_ref()
                    .is_some_and(|session| self.selected_since(session, selection))
                {
                    tracing::info!(%model, "BUG-576: a later model selection stands");
                    return;
                }
                self.dispatch(Action::ModelSelected(session, provider, model));
            }
            MissingLimitRetry::SessionCreation {
                isolated,
                role,
                workspace,
                card,
                seed_active,
                originating_session,
            } => {
                if tokio::runtime::Handle::try_current().is_err() {
                    return;
                }
                let active_session = seed_active.then(|| self.active_session_tx.clone());
                self.pending_tasks.push(tokio::spawn(
                    super::session_creation_tasks::run_session_creation(
                        self.backend.clone(),
                        super::session_creation_tasks::CreationRequest {
                            isolated,
                            role,
                            workspace,
                            card,
                        },
                        active_session,
                        originating_session,
                        self.action_tx.clone(),
                    ),
                ));
            }
        }
    }
}

/// The profile's stored definition, so saving the missing limit keeps every
/// other setting.
fn current_definition(
    configs: &[codelet_rpc_types::ProviderSettingsConfig],
    limit: &MissingProfileLimit,
) -> Result<ProfileDefinition, String> {
    configs
        .iter()
        .find(|config| config.provider_id == limit.provider)
        .and_then(|config| {
            config
                .profiles
                .iter()
                .find(|profile| profile.name == limit.profile)
        })
        .map(|profile| profile.definition.clone())
        .ok_or_else(|| {
            format!(
                "the {} profile of {} is not in the provider settings",
                limit.profile, limit.provider
            )
        })
}

/// Show a refused selection or creation where the original failure would
/// have been shown.
pub(crate) fn report_refusal(
    action_tx: &UnboundedSender<Action>,
    retry: &MissingLimitRetry,
    message: &str,
) {
    match retry {
        MissingLimitRetry::ModelSelected {
            session: Some(session),
            ..
        } => {
            let _ = action_tx.send(Action::EmitSessionNotice(
                session.clone(),
                format!("[error] Model selection failed: {message}"),
            ));
        }
        MissingLimitRetry::ModelSelected { session: None, .. } => {
            let _ = action_tx.send(Action::SessionOperationFailed(format!(
                "Model selection failed: {message}"
            )));
        }
        MissingLimitRetry::SessionCreation {
            originating_session,
            isolated,
            ..
        } => {
            if let Some(session) = originating_session {
                let operation = if *isolated {
                    "create isolated session"
                } else {
                    "create session"
                };
                let _ = action_tx.send(Action::EmitSessionNotice(
                    session.clone(),
                    format!("[error] {operation}: {message}"),
                ));
            }
            let _ = action_tx.send(Action::SessionOperationFailed(format!(
                "Session creation failed: {message}"
            )));
        }
    }
}
