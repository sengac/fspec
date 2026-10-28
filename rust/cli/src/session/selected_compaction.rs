use super::Session;
use codelet_core::persistence::compaction_boundary::{CommittedGeneration, CompactionSource};
use codelet_core::persistence::compaction_commit::CompactionCommitError;
use codelet_core::persistence::SessionManifest;
use codelet_providers::selected_request::SelectedRequestPreparer;
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct SelectedPreparation {
    provider: String,
    model: Option<String>,
    selection: Option<String>,
    preparer: SelectedRequestPreparer,
}

impl Session {
    pub(super) fn matches_persisted_selection(&self, provider: &str, model: &str) -> bool {
        if self.current_model_id().as_deref() != Some(model) {
            return false;
        }
        if let Some(selection) = self.provider_manager().selected_model_string() {
            if provider == selection {
                return true;
            }
            if selection
                .split_once('/')
                .is_some_and(|(provider, _)| provider.contains(':'))
            {
                return false;
            }
        }
        let alias = match self.provider_manager().current_provider_type() {
            codelet_providers::ProviderType::Claude => Some("anthropic"),
            codelet_providers::ProviderType::Gemini => Some("google"),
            _ => None,
        };
        provider == self.current_provider_name()
            || provider == format!("{}/{model}", self.current_provider_name())
            || alias
                .is_some_and(|alias| provider == alias || provider == format!("{alias}/{model}"))
    }

    pub(super) fn validate_current_compaction_history(
        &self,
        manifest: &SessionManifest,
        source: &CompactionSource,
    ) -> Result<(), String> {
        if self.current_provider_name() != "codex" {
            return Ok(());
        }
        let selected = self
            .selected_preparer
            .as_ref()
            .ok_or("missing selected history validator")?;
        if selected.provider != self.current_provider_name()
            || selected.model != self.current_model_id()
            || selected.selection != self.provider_manager().selected_model_string()
        {
            return Err("selected history validator identity changed".into());
        }
        let reminders = super::preservation::retained_reminders(self, source)?;
        let messages = super::compaction::replay_messages(manifest, &reminders)?;
        selected
            .preparer
            .validate_current_history(&messages)
            .map_err(|error| error.to_string())
    }

    pub fn install_selected_preparer(&mut self, preparer: SelectedRequestPreparer) {
        self.selected_preparer = Some(SelectedPreparation {
            provider: self.current_provider_name().to_owned(),
            model: self.current_model_id(),
            selection: self.provider_manager().selected_model_string(),
            preparer,
        });
    }

    pub fn commit_selected_compaction(
        &mut self,
        source: CompactionSource,
        operation_id: Uuid,
        content: &str,
        publish: impl FnOnce(&CommittedGeneration) -> Result<(), String>,
    ) -> Result<SessionManifest, CompactionCommitError> {
        self.commit_selected_compaction_guarded(source, operation_id, content, || Ok(()), publish)
    }

    pub fn commit_selected_compaction_guarded(
        &mut self,
        source: CompactionSource,
        operation_id: Uuid,
        content: &str,
        check: impl Fn() -> Result<(), codelet_providers::request_admission::RequestAdmissionError>,
        publish: impl FnOnce(&CommittedGeneration) -> Result<(), String>,
    ) -> Result<SessionManifest, CompactionCommitError> {
        self.block_compaction_inference("selected compaction awaits durable reconciliation".into());
        let selected = self.selected_preparer.as_ref().ok_or_else(|| {
            CompactionCommitError::Rejected(
                "selected request admission preparer unavailable".into(),
            )
        })?;
        if selected.provider != self.current_provider_name()
            || selected.model != self.current_model_id()
            || selected.selection != self.provider_manager().selected_model_string()
            || !self.matches_persisted_selection(&source.provider, &source.model)
        {
            return Err(CompactionCommitError::Rejected(
                "selected request admission identity changed".into(),
            ));
        }
        let preparer = selected.preparer.clone();
        self.commit_durable_compaction(
            source,
            operation_id,
            content,
            |messages, generation| {
                check()?;
                let prepared = preparer.prepare(messages, generation)?;
                check()?;
                Ok(prepared)
            },
            publish,
        )
    }

    pub fn resolve_compaction_operation(&mut self) {
        if let Some(operation) = &mut self.compaction_operation {
            operation.resolve();
        }
    }
}
