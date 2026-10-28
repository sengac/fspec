use super::{history::CodexHistory, metadata::CodexMetadataHandle, routing::CodexRoutingHandle};
use crate::session::{
    ExtensionError, ProviderSessionExtension, ProviderStatusEvent, StateWriter, StatusObserver,
};
use serde_json::{json, Value};
use std::{any::Any, sync::Arc};

#[path = "checkpoint_restore.rs"]
mod checkpoint_restore;
#[path = "compaction.rs"]
mod compaction;
pub use compaction::committed_checkpoint;

#[derive(Default)]
pub struct CodexSession {
    pub history: CodexHistory,
    pub routing: CodexRoutingHandle,
    pub metadata: CodexMetadataHandle,
    pub recovery: super::reconnect::CodexRecoveryHandle,
}

impl ProviderSessionExtension for CodexSession {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn restore(&self, envelopes: &[String]) -> Result<(), ExtensionError> {
        self.history.block();
        let selected = checkpoint_restore::select(
            envelopes,
            self.history.snapshot()?,
            self.history.generation()?,
        )?;
        self.history
            .restore_state(selected.checkpoint, selected.generation)?;
        Ok(())
    }

    fn prepare_compaction_generation(
        &self,
        generation: u64,
        model: &str,
        envelopes: &[String],
    ) -> Result<crate::session::PreparedProviderGeneration, ExtensionError> {
        compaction::prepare(&self.history, generation, model, envelopes)
    }

    fn validate_restore_for_provider(
        &self,
        provider: &str,
        envelopes: &[String],
    ) -> Result<(), ExtensionError> {
        if checkpoint_restore::ordinary(provider, envelopes)? {
            self.history.generation()?.checked_add(1).ok_or(
                super::history::CheckpointError::Lineage("generation exhausted"),
            )?;
            self.history.snapshot()?;
            return Ok(());
        }
        self.validate_restore(envelopes)
    }

    fn restore_for_provider(
        &self,
        provider: &str,
        envelopes: &[String],
    ) -> Result<(), ExtensionError> {
        if checkpoint_restore::ordinary(provider, envelopes)? {
            let generation = self.history.generation()?.checked_add(1).ok_or(
                super::history::CheckpointError::Lineage("generation exhausted"),
            )?;
            self.history.restore_state(None, generation)?;
            self.routing.reset();
            return Ok(());
        }
        self.restore(envelopes)
    }

    fn validate_restore(&self, envelopes: &[String]) -> Result<(), ExtensionError> {
        checkpoint_restore::select(
            envelopes,
            self.history.snapshot()?,
            self.history.generation()?,
        )?;
        Ok(())
    }

    fn validate_committed_restore(
        &self,
        generation: u64,
        metadata: &std::collections::HashMap<String, Value>,
        envelopes: &[String],
    ) -> Result<(), ExtensionError> {
        let checkpoint = committed_checkpoint(generation, &serde_json::to_value(metadata)?)?;
        let provider_generation = checkpoint.generation;
        let expected_model = checkpoint.model.clone();
        let isolated = CodexHistory::default();
        isolated.restore_state(self.history.snapshot()?, self.history.generation()?)?;
        isolated.install_committed_generation(provider_generation, checkpoint)?;
        let selected =
            checkpoint_restore::select(envelopes, isolated.snapshot()?, isolated.generation()?)?
                .checkpoint
                .ok_or(super::history::CheckpointError::Missing)?;
        if selected.generation != provider_generation || selected.model != expected_model {
            return Err(
                super::history::CheckpointError::Lineage("prospective replay identity").into(),
            );
        }
        Ok(())
    }

    fn owns_checkpoint_metadata(
        &self,
        metadata: &std::collections::HashMap<String, Value>,
    ) -> bool {
        metadata.contains_key("codexContinuation")
    }

    fn prepare_generation(
        &self,
        generation: u64,
        model: &str,
    ) -> Result<std::collections::HashMap<String, Value>, ExtensionError> {
        let checkpoint = self.history.prepare_generation(generation, model)?;
        Ok([(
            "codexContinuation".into(),
            serde_json::to_value(checkpoint)?,
        )]
        .into_iter()
        .collect())
    }

    fn install_committed_generation(
        &self,
        generation: u64,
        metadata: &std::collections::HashMap<String, Value>,
    ) -> Result<(), ExtensionError> {
        let checkpoint = committed_checkpoint(generation, &serde_json::to_value(metadata)?)?;
        self.history
            .install_committed_generation(checkpoint.generation, checkpoint)?;
        self.routing.reset();
        self.metadata.reset();
        Ok(())
    }

    fn transition_generation(&self, generation: u64) -> Result<(), ExtensionError> {
        self.history.transition_generation(generation)?;
        self.routing.reset();
        Ok(())
    }

    fn set_writer(&self, writer: StateWriter) -> Result<(), ExtensionError> {
        self.history
            .set_writer_with_generation(Arc::new(move |checkpoint, generation| {
                let reset = if checkpoint.is_none() {
                    json!({"version":1,"generation":generation,"protocol":"full-current-dag"})
                } else {
                    Value::Null
                };
                let value = serde_json::to_value(checkpoint).map_err(|error| error.to_string())?;
                writer(
                    [
                        ("codexContinuation".into(), value),
                        ("_codexReset".into(), reset),
                    ]
                    .into_iter()
                    .collect(),
                )
                .map_err(|error| error.to_string())
            }))?;
        Ok(())
    }

    fn begin_turn(&self) {
        self.routing.begin_turn();
        self.metadata.begin_turn();
    }

    fn reset(&self) -> Result<(), ExtensionError> {
        self.history.reset()?;
        self.routing.reset();
        self.metadata.reset();
        Ok(())
    }

    fn status(&self) -> Option<Value> {
        let mut status = json!(self.metadata.status());
        status["recovery"] = json!(self.recovery.status());
        Some(status)
    }

    fn set_observer(&self, observer: Option<StatusObserver>) {
        let recovery_observer = observer.clone();
        self.recovery.set_observer(Arc::new(move |status| {
            if let Some(observer) = &recovery_observer {
                observer(ProviderStatusEvent::Notification(status));
            }
        }));
        self.metadata
            .set_presentation_observer(Arc::new(move |event| {
                if let Some(observer) = &observer {
                    use super::metadata::CodexPresentationEvent;
                    observer(match event {
                        CodexPresentationEvent::Waiting(active) => ProviderStatusEvent::Waiting(
                            active.then(|| "Giving this request a little extra thought".into()),
                        ),
                        CodexPresentationEvent::Verification => ProviderStatusEvent::Notification(
                            "Codex account verification recommended: trusted access for cyber"
                                .into(),
                        ),
                    });
                }
            }));
    }
}
