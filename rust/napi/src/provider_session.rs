use codelet_providers::{session::ProviderSession, ProviderAdapter};
use codelet_sessions::background_session::BackgroundSession;
use std::sync::Arc;

pub(crate) fn prepare_provider<Provider: ProviderAdapter>(
    provider: Provider,
    session: &Arc<BackgroundSession>,
    extensions: &mut ProviderSession,
) -> Provider {
    let name = provider.provider_name();
    extensions.ensure_provider(name);
    extensions.begin_turn();
    session.prepare_provider_status(extensions, name);
    provider.with_session(extensions)
}
