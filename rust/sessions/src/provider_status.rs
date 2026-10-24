use crate::background_session::BackgroundSession;
use codelet_providers::{
    codex::session::CodexSession,
    session::{ProviderSession, ProviderStatusEvent},
};
use codelet_rpc_types::StreamChunk;
use std::sync::Arc;

impl BackgroundSession {
    pub fn provider_metadata_status(&self) -> Option<serde_json::Value> {
        let slot = self.provider_status.lock().ok()?;
        let (provider, extensions) = slot.as_ref()?;
        extensions.status(provider)
    }

    pub fn prepare_provider_status(self: &Arc<Self>, extensions: &ProviderSession, provider: &str) {
        self.clear_provider_presentation();
        let session = Arc::downgrade(self);
        extensions.observe(
            provider,
            Arc::new(move |event| {
                if let Some(session) = session.upgrade() {
                    session.handle_output(match event {
                        ProviderStatusEvent::Notification(message) => {
                            StreamChunk::UserNotification {
                                message,
                                severity: codelet_rpc_types::NotificationSeverity::Info,
                            }
                        }
                        ProviderStatusEvent::Waiting(message) => {
                            StreamChunk::ProviderWaiting { message }
                        }
                    });
                }
            }),
        );
        if let Ok(mut slot) = self.provider_status.lock() {
            *slot = Some((provider.into(), extensions.clone()));
        }
    }

    pub(crate) fn clear_provider_presentation(&self) {
        let selected = self
            .provider_status
            .lock()
            .ok()
            .and_then(|slot| slot.clone());
        if let Some((provider, extensions)) = selected {
            if provider == "codex" {
                if let Some(codex) = extensions.extension::<CodexSession>(&provider) {
                    codex.metadata.reset();
                }
            }
        }
    }
}
