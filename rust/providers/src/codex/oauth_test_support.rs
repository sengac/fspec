use std::sync::Mutex;

#[derive(Clone)]
pub struct OAuthTestConfig {
    pub issuer: String,
    pub callback_address: std::net::SocketAddr,
    pub state: String,
}

static CONFIG: Mutex<Option<OAuthTestConfig>> = Mutex::new(None);

pub struct OAuthTestGuard;

pub fn configure(config: OAuthTestConfig) -> OAuthTestGuard {
    *CONFIG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(config);
    OAuthTestGuard
}

pub(crate) fn config() -> Option<OAuthTestConfig> {
    CONFIG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

impl Drop for OAuthTestGuard {
    fn drop(&mut self) {
        *CONFIG
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
}
