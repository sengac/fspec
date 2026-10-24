use codelet_providers::codex::{reconnect::CodexNetworkFailure, session::CodexSession};
use codelet_providers::session::ProviderSession;
use std::error::Error;
use std::time::Duration;

pub async fn wait_for_stall_deadline(session: &ProviderSession, provider: &str, timeout: Duration) {
    let mut deadline = tokio::time::Instant::now() + timeout;
    loop {
        let now = tokio::time::Instant::now();
        let waiting = provider == "codex"
            && session
                .extension::<CodexSession>("codex")
                .is_some_and(|extension| extension.recovery.is_reconnecting());
        if waiting {
            deadline = now + timeout;
        } else if now >= deadline {
            return;
        }
        tokio::time::sleep(
            deadline
                .saturating_duration_since(now)
                .min(Duration::from_millis(50)),
        )
        .await;
    }
}

pub fn should_retry_network_error(provider: &str, error: &(dyn Error + 'static)) -> bool {
    if provider == "codex" {
        return false;
    }
    let mut source = Some(error);
    let mut transient = false;
    while let Some(error) = source {
        if error.downcast_ref::<CodexNetworkFailure>().is_some() {
            return false;
        }
        transient |= super::error_classifiers::is_transient_network_error(&error.to_string());
        source = error.source();
    }
    transient
}
