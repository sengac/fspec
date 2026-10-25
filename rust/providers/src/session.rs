use serde_json::Value;
use std::{
    any::Any,
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error(transparent)]
    Checkpoint(#[from] crate::codex::history::CheckpointError),
    #[error("invalid provider session state: {0}")]
    Json(#[from] serde_json::Error),
    #[error("provider session persistence failed: {0}")]
    Persistence(String),
}

pub type StateWriter = Arc<
    dyn Fn(std::collections::HashMap<String, Value>) -> Result<(), ExtensionError> + Send + Sync,
>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderStatusEvent {
    Notification(String),
    Waiting(Option<String>),
}

pub type StatusObserver = Arc<dyn Fn(ProviderStatusEvent) + Send + Sync>;

pub trait ProviderSessionExtension: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn restore(&self, envelopes: &[String]) -> Result<(), ExtensionError>;
    fn validate_restore(&self, _envelopes: &[String]) -> Result<(), ExtensionError> {
        Ok(())
    }

    fn restore_for_provider(
        &self,
        _provider: &str,
        envelopes: &[String],
    ) -> Result<(), ExtensionError> {
        self.restore(envelopes)
    }
    fn validate_restore_for_provider(
        &self,
        _provider: &str,
        envelopes: &[String],
    ) -> Result<(), ExtensionError> {
        self.validate_restore(envelopes)
    }
    fn owns_checkpoint_metadata(&self, _metadata: &HashMap<String, Value>) -> bool {
        false
    }

    fn validate_committed_restore(
        &self,
        _generation: u64,
        _metadata: &HashMap<String, Value>,
        envelopes: &[String],
    ) -> Result<(), ExtensionError> {
        self.validate_restore(envelopes)
    }
    fn set_writer(&self, writer: StateWriter) -> Result<(), ExtensionError>;
    fn begin_turn(&self);
    fn reset(&self) -> Result<(), ExtensionError>;
    fn transition_generation(&self, _generation: u64) -> Result<(), ExtensionError> {
        self.reset()
    }
    fn prepare_generation(
        &self,
        _generation: u64,
        _model: &str,
    ) -> Result<HashMap<String, Value>, ExtensionError> {
        Ok(HashMap::new())
    }
    fn install_committed_generation(
        &self,
        _generation: u64,
        _metadata: &HashMap<String, Value>,
    ) -> Result<(), ExtensionError> {
        Ok(())
    }

    fn status(&self) -> Option<Value> {
        None
    }
    fn set_observer(&self, _observer: Option<StatusObserver>) {}
}

#[derive(Clone, Default)]
pub struct ProviderSession {
    extensions: BTreeMap<String, Arc<dyn ProviderSessionExtension>>,
}

impl std::fmt::Debug for ProviderSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderSession")
            .field("extensions", &self.extensions.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl ProviderSession {
    pub fn for_provider(provider: &str) -> Self {
        let mut session = Self::default();
        session.ensure_provider(provider);
        session
    }

    pub fn ensure_provider(&mut self, provider: &str) {
        if provider == "github-copilot" && !self.extensions.contains_key(provider) {
            self.register(
                provider,
                Arc::new(crate::copilot::session::CopilotSession::default()),
            );
        } else if provider != "github-copilot" {
            if let Some(extension) = self.extensions.get("github-copilot") {
                let _ = extension.reset();
            }
        }
        if provider == "codex" && !self.extensions.contains_key(provider) {
            self.register(
                provider,
                Arc::new(crate::codex::session::CodexSession::default()),
            );
        }
    }

    pub fn for_compaction_checkpoint(
        &self,
        provider: &str,
        metadata: &HashMap<String, Value>,
    ) -> Self {
        Self {
            extensions: self
                .extensions
                .iter()
                .filter(|(name, extension)| {
                    name.as_str() == provider || extension.owns_checkpoint_metadata(metadata)
                })
                .map(|(name, extension)| (name.clone(), extension.clone()))
                .collect(),
        }
    }

    pub fn for_selected_provider(&self, provider: &str) -> Self {
        Self {
            extensions: self
                .extensions
                .get_key_value(provider)
                .map(|(name, extension)| (name.clone(), extension.clone()))
                .into_iter()
                .collect(),
        }
    }

    pub fn register(&mut self, provider: &str, extension: Arc<dyn ProviderSessionExtension>) {
        self.extensions.insert(provider.into(), extension);
    }

    pub fn bind_live_session(&mut self, owner: uuid::Uuid) {
        crate::copilot::session::bind_live_session(self, owner);
    }

    pub fn extension<T: ProviderSessionExtension>(&self, provider: &str) -> Option<&T> {
        self.extensions.get(provider)?.as_any().downcast_ref()
    }

    pub fn restore(&self, envelopes: &[String]) -> Result<(), ExtensionError> {
        for extension in self.extensions.values() {
            extension.restore(envelopes)?;
        }
        Ok(())
    }

    pub fn validate_restore(&self, envelopes: &[String]) -> Result<(), ExtensionError> {
        for extension in self.extensions.values() {
            extension.validate_restore(envelopes)?;
        }
        Ok(())
    }

    pub fn restore_for_provider(
        &self,
        provider: &str,
        envelopes: &[String],
    ) -> Result<(), ExtensionError> {
        for extension in self.extensions.values() {
            extension.restore_for_provider(provider, envelopes)?;
        }
        Ok(())
    }

    pub fn validate_restore_for_provider(
        &self,
        provider: &str,
        envelopes: &[String],
    ) -> Result<(), ExtensionError> {
        for extension in self.extensions.values() {
            extension.validate_restore_for_provider(provider, envelopes)?;
        }
        Ok(())
    }

    pub fn validate_committed_restore(
        &self,
        generation: u64,
        metadata: &HashMap<String, Value>,
        envelopes: &[String],
    ) -> Result<(), ExtensionError> {
        for extension in self.extensions.values() {
            extension.validate_committed_restore(generation, metadata, envelopes)?;
        }
        Ok(())
    }

    pub fn prepare_generation(
        &self,
        generation: u64,
        model: &str,
    ) -> Result<HashMap<String, Value>, ExtensionError> {
        let mut metadata = HashMap::new();
        for extension in self.extensions.values() {
            metadata.extend(extension.prepare_generation(generation, model)?);
        }
        Ok(metadata)
    }

    pub fn install_committed_generation(
        &self,
        generation: u64,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), ExtensionError> {
        for extension in self.extensions.values() {
            extension.install_committed_generation(generation, metadata)?;
        }
        Ok(())
    }

    pub fn transition_generation(&self, generation: u64) -> Result<(), ExtensionError> {
        for extension in self.extensions.values() {
            extension.transition_generation(generation)?;
        }
        Ok(())
    }

    pub fn bind_writers(&self, writer: impl Fn(&str) -> StateWriter) -> Result<(), ExtensionError> {
        for (provider, extension) in &self.extensions {
            extension.set_writer(writer(provider))?;
        }
        Ok(())
    }

    pub fn begin_turn(&self) {
        for extension in self.extensions.values() {
            extension.begin_turn();
        }
    }

    pub fn reset(&self) -> Result<(), ExtensionError> {
        for extension in self.extensions.values() {
            extension.reset()?;
        }
        Ok(())
    }

    pub fn status(&self, provider: &str) -> Option<Value> {
        self.extensions.get(provider)?.status()
    }

    pub fn observe(&self, provider: &str, observer: StatusObserver) {
        for (name, extension) in &self.extensions {
            extension.set_observer((name == provider).then(|| observer.clone()));
        }
    }
}
