//! BUG-306: sessions for tests that never reach a provider. The provider is
//! selected explicitly so tests do not depend on which providers this machine
//! has credentialed (several credentialed providers refuse implicit selection).

use codelet_cli::session::Session;
use codelet_providers::{ProviderError, ProviderManager};

const PROVIDER: &str = "claude";

fn offline_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "test-no-network");
}

#[allow(dead_code)]
pub fn session() -> anyhow::Result<Session> {
    offline_credentials();
    Session::new(Some(PROVIDER))
}

#[allow(dead_code)]
pub fn provider_manager() -> Result<ProviderManager, ProviderError> {
    offline_credentials();
    ProviderManager::with_provider(PROVIDER)
}
