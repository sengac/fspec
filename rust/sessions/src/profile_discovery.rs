use codelet_providers::error::ModelDiscoveryFailure;
use codelet_providers::{OpenAIProvider, ProviderError};

use super::LocalServerProfile;

enum DiscoveryState {
    Supported,
    Empty,
    Unsupported(u16),
    Failed(ModelDiscoveryFailure),
    UnclassifiedFailure,
    Skipped,
}

impl DiscoveryState {
    fn label(&self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Empty => "empty",
            Self::Unsupported(_) => "unsupported",
            Self::Failed(ModelDiscoveryFailure::Authentication(_)) => "authentication_failed",
            Self::Failed(ModelDiscoveryFailure::Transport | ModelDiscoveryFailure::Timeout) => {
                "transport_failed"
            }
            Self::Failed(ModelDiscoveryFailure::Protocol) => "protocol_failed",
            Self::Failed(ModelDiscoveryFailure::Http(_)) => "http_failed",
            Self::UnclassifiedFailure => "failed",
            Self::Skipped => "skipped",
        }
    }

    fn probe_failed(&self) -> bool {
        !matches!(self, Self::Supported | Self::Empty | Self::Unsupported(_))
    }

    fn http_status(&self) -> Option<u16> {
        match self {
            Self::Unsupported(status)
            | Self::Failed(
                ModelDiscoveryFailure::Authentication(status) | ModelDiscoveryFailure::Http(status),
            ) => Some(*status),
            _ => None,
        }
    }

    fn report(&self) {
        let discovery = self.label();
        let probe_failed = self.probe_failed();
        let http_status = self.http_status();
        let timed_out = matches!(self, Self::Failed(ModelDiscoveryFailure::Timeout));
        if matches!(self, Self::Supported | Self::Empty) {
            tracing::info!(
                target: "profile_sections",
                discovery, inference = "unknown", probe_failed, ?http_status, timed_out,
                "profile model discovery",
            );
        } else {
            tracing::warn!(
                target: "profile_sections",
                discovery, inference = "unknown", probe_failed, ?http_status, timed_out,
                "profile model discovery",
            );
        }
    }
}

pub(super) fn probe_profile_models(profile: &LocalServerProfile) -> (Vec<String>, bool) {
    if !matches!(
        tokio::runtime::Handle::try_current().map(|handle| handle.runtime_flavor()),
        Ok(tokio::runtime::RuntimeFlavor::MultiThread)
    ) {
        DiscoveryState::Skipped.report();
        return (Vec::new(), true);
    }
    let result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(OpenAIProvider::list_local_models_with_auth(
            &profile.base_url,
            profile.api_key.as_deref(),
        ))
    });
    let (models, state) = match result {
        Ok(models) => {
            let state = if models.is_empty() {
                DiscoveryState::Empty
            } else {
                DiscoveryState::Supported
            };
            (models, state)
        }
        Err(ProviderError::ModelDiscoveryUnsupported { status, .. }) => {
            (Vec::new(), DiscoveryState::Unsupported(status))
        }
        Err(ProviderError::ModelDiscoveryFailed { failure, .. }) => {
            (Vec::new(), DiscoveryState::Failed(failure))
        }
        Err(_) => (Vec::new(), DiscoveryState::UnclassifiedFailure),
    };
    state.report();
    (models, state.probe_failed())
}
