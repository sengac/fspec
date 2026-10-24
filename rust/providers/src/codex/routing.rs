use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Request};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct CodexRoutingHandle(Arc<Mutex<State>>);

struct State {
    affinity: String,
    account: Option<[u8; 32]>,
    auth_identity: Option<super::auth_refresh::Identity>,
    generation: u64,
    turn_state: Option<HeaderValue>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            affinity: Uuid::new_v4().to_string(),
            account: None,
            auth_identity: None,
            generation: 0,
            turn_state: None,
        }
    }
}

impl std::fmt::Debug for CodexRoutingHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CodexRoutingHandle(<private>)")
    }
}

#[derive(Clone)]
pub struct RoutingRequest {
    handle: CodexRoutingHandle,
    generation: u64,
}

impl CodexRoutingHandle {
    pub(crate) fn observe_event_turn_state(&self, value: &str) {
        if let Ok(state) = self.0.lock() {
            let request = RoutingRequest {
                handle: self.clone(),
                generation: state.generation,
            };
            drop(state);
            request.observe_turn_state(value);
        }
    }

    pub fn begin_turn(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.generation = state.generation.wrapping_add(1);
            state.turn_state = None;
        }
    }

    pub fn reset(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.affinity = Uuid::new_v4().to_string();
            state.generation = state.generation.wrapping_add(1);
            state.turn_state = None;
        }
    }

    pub fn bind_account(&self, account: &str) -> RoutingRequest {
        self.bind_identity(account, None)
    }

    fn bind_identity(
        &self,
        account: &str,
        identity: Option<super::auth_refresh::Identity>,
    ) -> RoutingRequest {
        let digest: [u8; 32] = Sha256::digest(account.as_bytes()).into();
        let generation = if let Ok(mut state) = self.0.lock() {
            if state.account.is_some_and(|previous| previous != digest)
                || state.auth_identity != identity
            {
                state.affinity = Uuid::new_v4().to_string();
                state.generation = state.generation.wrapping_add(1);
                state.turn_state = None;
            }
            state.account = Some(digest);
            state.auth_identity = identity;
            state.generation
        } else {
            u64::MAX
        };
        RoutingRequest {
            handle: self.clone(),
            generation,
        }
    }
}

impl RoutingRequest {
    pub fn observe_turn_state(&self, value: &str) {
        if value.is_empty()
            || value.len() > 4096
            || !value.bytes().all(|byte| (33..=126).contains(&byte))
        {
            return;
        }
        if let (Ok(mut state), Ok(value)) = (self.handle.0.lock(), HeaderValue::from_str(value)) {
            if state.generation == self.generation && state.turn_state.is_none() {
                state.turn_state = Some(value);
            }
        }
    }

    pub(crate) fn observe_headers(&self, headers: &HeaderMap) {
        if let Some(value) = headers
            .get("x-codex-turn-state")
            .and_then(|value| value.to_str().ok())
        {
            self.observe_turn_state(value);
        }
    }

    fn apply(&self, request: &mut Request<Bytes>) -> rig::http_client::Result<()> {
        let Ok(state) = self.handle.0.lock() else {
            return Ok(());
        };
        if state.generation != self.generation {
            return Ok(());
        }
        let mut body: Value = serde_json::from_slice(request.body()).map_err(|_| {
            rig::http_client::Error::Instance("invalid Codex routing request".into())
        })?;
        body["prompt_cache_key"] = Value::String(state.affinity.clone());
        *request.body_mut() = serde_json::to_vec(&body)
            .map_err(|_| rig::http_client::Error::Instance("invalid Codex routing request".into()))?
            .into();
        request.headers_mut().remove(http::header::CONTENT_LENGTH);
        if let Ok(key) = HeaderValue::from_str(&state.affinity) {
            request.headers_mut().insert("session_id", key);
        }
        request.headers_mut().remove("x-codex-turn-state");
        if let Some(value) = &state.turn_state {
            request
                .headers_mut()
                .insert("x-codex-turn-state", value.clone());
        }
        Ok(())
    }
}

pub(crate) fn prepare(
    request: &mut Request<Bytes>,
) -> rig::http_client::Result<Option<RoutingRequest>> {
    let Some(handle) = request.extensions().get::<CodexRoutingHandle>().cloned() else {
        return Ok(None);
    };
    let account = request
        .headers()
        .get("chatgpt-account-id")
        .or_else(|| request.headers().get(http::header::AUTHORIZATION))
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let routing = handle.bind_identity(
        account,
        request
            .extensions()
            .get::<super::auth_refresh::Identity>()
            .cloned(),
    );
    routing.apply(request)?;
    Ok(Some(routing))
}
